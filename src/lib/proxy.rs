#![allow(dead_code)]
#![allow(unused_imports)]

use super::encrypt;
use actix_multipart::{Multipart};
use actix_web::client::Client;
use actix_web::{middleware, web, App, Error, HttpRequest, HttpResponse, HttpServer};
use clap::{value_t, Arg};
use futures::Future;
use std::net::ToSocketAddrs;
use url::Url;

fn forward(
    req: HttpRequest,
    multipart: Multipart,
    url: web::Data<Url>,
    client: web::Data<Client>,
) -> impl Future<Item = HttpResponse, Error = Error> {

    let mut new_url = url.get_ref().clone();
    new_url.set_path(req.uri().path());
    new_url.set_query(req.uri().query());

    let forwarded_req = client.request_from(new_url.as_str(), req.head());
    let forwarded_req = if let Some(addr) = req.head().peer_addr {
        forwarded_req.header("x-forwarded-for", format!("{}", addr.ip()))
    } else {
        forwarded_req
    };


    use sodiumoxide::crypto::secretstream::xchacha20poly1305::{Key};
    use sodiumoxide::crypto::secretstream::xchacha20poly1305;
    use futures::stream;
    use futures::stream::Stream;

    let key: Key = encrypt::build_key();

    let (mut enc_stream, header) = xchacha20poly1305::Stream::init_push(&key).unwrap();

    // let chunck_size = 2;

    use bytes::Bytes;
    let header_bytes = Bytes::from(header.as_ref());

    let header_stream = stream::once::<Bytes, Error>(Ok(header_bytes));

    use futures::future::Future;
    use futures::future;

    // let encoder = multipart
    //     .map_err(error::ErrorInternalServerError)
    //     .map(|field| field)

    // let result_stream = header_stream.chain(encoder);
    //
    use bytes::{BytesMut};

    // let multipart_stream  =
    //     multipart
    //     .map(|field| {
    //         field.fold(BytesMut::with_capacity(1024), |acc, bytes: Bytes| future::ok(acc.extend_from_slice(&bytes)))
    //     });
    //
    let multipart_stream = multipart.flatten();

    forwarded_req
        .send_stream(multipart_stream)
        .map_err(Error::from)
        .map(|res:  actix_web::client::ClientResponse<_>| {
            let mut client_resp = HttpResponse::build(res.status());
            for (header_name, header_value) in
                res.headers().iter().filter(|(h, _)| *h != "connection")
            {
                client_resp.header(header_name.clone(), header_value.clone());
            }
            client_resp.streaming(res)
        })
}

fn main() -> std::io::Result<()> {
    let matches = clap::App::new("HTTP Proxy")
        .arg(
            Arg::with_name("listen_addr")
                .takes_value(true)
                .value_name("LISTEN ADDR")
                .index(1)
                .required(true),
        )
        .arg(
            Arg::with_name("listen_port")
                .takes_value(true)
                .value_name("LISTEN PORT")
                .index(2)
                .required(true),
        )
        .arg(
            Arg::with_name("forward_addr")
                .takes_value(true)
                .value_name("FWD ADDR")
                .index(3)
                .required(true),
        )
        .arg(
            Arg::with_name("forward_port")
                .takes_value(true)
                .value_name("FWD PORT")
                .index(4)
                .required(true),
        )
        .get_matches();

    let listen_addr = matches.value_of("listen_addr").unwrap();
    let listen_port = value_t!(matches, "listen_port", u16).unwrap_or_else(|e| e.exit());

    let forwarded_addr = matches.value_of("forward_addr").unwrap();
    let forwarded_port =
        value_t!(matches, "forward_port", u16).unwrap_or_else(|e| e.exit());

    let forward_url = Url::parse(&format!(
        "http://{}",
        (forwarded_addr, forwarded_port)
            .to_socket_addrs()
            .unwrap()
            .next()
            .unwrap()
    ))
    .unwrap();

    HttpServer::new(move || {
        App::new()
            .data(forward_url.clone())
            .data(actix_web::client::Client::new())
            .wrap(middleware::Logger::default())
            .default_service(web::route().to_async(forward))
    })
    .bind((listen_addr, listen_port))?
    .system_exit()
    .run()
}

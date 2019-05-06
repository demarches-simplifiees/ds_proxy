use super::key::*;
use super::encrypt::*;
use super::decoder::*;
use actix_web::client::Client;
use actix_web::{middleware, web, App, Error, HttpRequest, HttpResponse, HttpServer};
use clap::{value_t, Arg};
use futures::Future;
use actix_web::guard;

const URL: &str = "https://storage.gra5.cloud.ovh.net/***";
const GET_URL: &str = "https://storage.gra5.cloud.ovh.net/***";

fn forward(
    _req: HttpRequest,
    payload: web::Payload,
    client: web::Data<Client>,
) -> impl Future<Item = HttpResponse, Error = Error> {

    let key = build_key();
    let encoder = Encoder::new(key, 512, Box::new(payload));

    client.put(URL)
        .header("User-Agent", "Actix-web")
        .send_stream(encoder)
        .map_err(|e| {
            println!("==== erreur1 ====");
            println!("{:?}", e);
            Error::from(e)
        })
    .map(|res| {
        let mut client_resp = HttpResponse::build(res.status());
        for (header_name, header_value) in
            res.headers().iter().filter(|(h, _)| *h != "connection")
            {
                client_resp.header(header_name.clone(), header_value.clone());
            }
        client_resp.streaming(res)
    })
}

fn fetch(
    _req: HttpRequest,
    client: web::Data<Client>,
) -> impl Future<Item = HttpResponse, Error = Error> {

    client.get(GET_URL)
        .header("User-Agent", "Actix-web")
        .send()
        .map_err(|e| {
            println!("==== erreur1 ====");
            println!("{:?}", e);
            Error::from(e)
        })
    .map(|res| {
        let mut client_resp = HttpResponse::build(res.status());
        for (header_name, header_value) in
            res.headers().iter().filter(|(h, _)| *h != "connection")
            {
                client_resp.header(header_name.clone(), header_value.clone());
            }

        let key = build_key();
        let decoder = Decoder::new(key, 512, Box::new(res));

        client_resp.streaming(decoder)
    })
}


pub fn main() -> std::io::Result<()> {
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
        .get_matches();

    let listen_addr = matches.value_of("listen_addr").unwrap();
    let listen_port = value_t!(matches, "listen_port", u16).unwrap_or_else(|e| e.exit());

    HttpServer::new(move || {
        App::new()
            .data(actix_web::client::Client::new())
            .wrap(middleware::Logger::default())
            .service(web::resource(".*").guard(guard::Get()).to_async(fetch))
            .default_service(web::route().guard(guard::Put()).to_async(forward))
    })
    .bind((listen_addr, listen_port))?
        .system_exit()
        .run()
}

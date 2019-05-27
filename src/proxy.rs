use super::config::Config;
use super::decoder::*;
use super::encrypt::*;
use super::key::*;
use actix_web::client::Client;
use actix_web::guard;
use actix_web::http::Uri;
use actix_web::{middleware, web, App, Error, HttpRequest, HttpResponse, HttpServer};
use futures::Future;
use futures::IntoFuture;
use std::time::Duration;

fn create_url(base_url: &str, uri: &Uri) -> String {
    format!("{}{}", base_url, uri)
}

fn forward(
    req: HttpRequest,
    payload: web::Payload,
    client: web::Data<Client>,
    upstream_base_url: web::Data<String>,
    _noop: web::Data<bool>,
) -> impl Future<Item = HttpResponse, Error = Error> {
    let key = build_key(
        "some_key".to_string().as_bytes(),
        &[
            170, 111, 168, 154, 69, 120, 180, 73, 145, 157, 199, 205, 254, 227, 149, 8, 204, 185,
            14, 56, 249, 178, 47, 47, 189, 158, 227, 250, 192, 13, 41, 76,
        ],
    );
    let encoder = Encoder::new(key, 512, Box::new(payload));

    let put_url = create_url(upstream_base_url.get_ref(), &req.uri());

    client
        .put(put_url)
        .timeout(Duration::from_secs(600))
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
    req: HttpRequest,
    client: web::Data<Client>,
    upstream_base_url: web::Data<String>,
    noop: web::Data<bool>,
) -> impl Future<Item = HttpResponse, Error = Error> {
    let get_url = create_url(upstream_base_url.get_ref(), &req.uri());

    client
        .get(get_url)
        .timeout(Duration::from_secs(600))
        .header("User-Agent", "Actix-web")
        .send()
        .map_err(|e| {
            println!("==== erreur1 ====");
            println!("{:?}", e);
            Error::from(e)
        })
        .map(move |res| {
            let mut client_resp = HttpResponse::build(res.status());
            for (header_name, header_value) in
                res.headers().iter().filter(|(h, _)| *h != "connection")
            {
                client_resp.header(header_name.clone(), header_value.clone());
            }

            let key = build_key(
                "some_key".to_string().as_bytes(),
                &[
                    170, 111, 168, 154, 69, 120, 180, 73, 145, 157, 199, 205, 254, 227, 149, 8,
                    204, 185, 14, 56, 249, 178, 47, 47, 189, 158, 227, 250, 192, 13, 41, 76,
                ],
            );
            if *noop.get_ref() {
                client_resp.streaming(res)
            } else {
                let decoder = Decoder::new(key, 512, Box::new(res));
                client_resp.streaming(decoder)
            }
        })
}

fn default(_req: HttpRequest) -> impl IntoFuture<Item = &'static str, Error = Error> {
    Ok("Hello world!\r\n")
}

pub fn main(
    listen_addr: &str,
    listen_port: u16,
    upstream_base_url: String,
    _config: Config,
) -> std::io::Result<()> {
    let noop = false;
    HttpServer::new(move || {
        App::new()
            .data(actix_web::client::Client::new())
            .data(upstream_base_url.clone())
            .data(noop)
            .wrap(middleware::Logger::default())
            .service(web::resource(".*").guard(guard::Get()).to_async(fetch))
            .service(web::resource(".*").guard(guard::Put()).to_async(forward)
            .default_service(web::route()).to_async(default))
    })
    .bind((listen_addr, listen_port))?
    .system_exit()
    .run()
}

use super::config::{Config, DsKey};
use super::decoder::*;
use super::encoder::*;
use actix_web::client::Client;
use actix_web::guard;
use actix_web::{middleware, web, App, Error, HttpRequest, HttpResponse, HttpServer};
use futures::Future;
use futures::IntoFuture;
use std::time::Duration;
use futures::stream::Stream;

const TIMEOUT_DURATION:Duration = Duration::from_secs(600);
const USER_AGENT:&str = "Actix-web";

fn forward(
    req: HttpRequest,
    payload: web::Payload,
    client: web::Data<Client>,
    config: web::Data<Config>,
    _noop: web::Data<bool>,
    key: web::Data<DsKey>,
) -> impl Future<Item = HttpResponse, Error = Error> {

    let config_ref = config.get_ref();

    let put_url = config_ref.create_url(&req.uri());

    let stream_to_send: Box<Stream<Item = _, Error = _>> = if config_ref.noop {
        Box::new(payload)
    } else {
        Box::new(Encoder::new(key.get_ref().clone(), 512, Box::new(payload)))
    };

    client
        .put(put_url)
        .timeout(TIMEOUT_DURATION)
        .header("User-Agent", USER_AGENT)
        .send_stream(stream_to_send)
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
    config: web::Data<Config>,
    noop: web::Data<bool>,
    key: web::Data<DsKey>,
) -> impl Future<Item = HttpResponse, Error = Error> {
    let get_url=  config.get_ref().create_url(&req.uri());

    client
        .get(get_url)
        .timeout(TIMEOUT_DURATION)
        .header("User-Agent", USER_AGENT)
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

            if *noop.get_ref() {
                client_resp.streaming(res)
            } else {
                let decoder = Decoder::new(key.get_ref().clone(), 512, Box::new(res));
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
    config: &'static Config,
) -> std::io::Result<()> {
    let noop = false;
    let key = config.clone().create_key().unwrap();

    HttpServer::new(move || {
        App::new()
            .data(actix_web::client::Client::new())
            .data(config.clone())
            .data(noop)
            .data(key.clone())
            .wrap(middleware::Logger::default())
            .service(web::resource(".*").guard(guard::Get()).to_async(fetch))
            .service(web::resource(".*").guard(guard::Put()).to_async(forward))
            .default_service(web::route().to_async(default))
    })
    .bind((listen_addr, listen_port))?
    .system_exit()
    .run()
}

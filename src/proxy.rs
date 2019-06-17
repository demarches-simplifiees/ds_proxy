use super::config::Config;
use super::decoder::*;
use super::encoder::*;
use actix_web::client::Client;
use actix_web::guard;
use actix_web::{middleware, web, App, Error, HttpRequest, HttpResponse, HttpServer};
use futures::Future;
use futures::IntoFuture;
use std::time::Duration;
use futures::stream::Stream;

const TIMEOUT_DURATION:Duration = Duration::from_secs(60*60);

// Encryption changes the value of those headers
static HEADERS_TO_REMOVE: [actix_web::http::header::HeaderName; 3] = [
    actix_web::http::header::CONTENT_LENGTH,
    actix_web::http::header::CONTENT_TYPE,
    actix_web::http::header::ETAG,
];

fn forward(
    req: HttpRequest,
    payload: web::Payload,
    client: web::Data<Client>,
    config: web::Data<Config>,
) -> impl Future<Item = HttpResponse, Error = Error> {

    let config_ref = config.get_ref();

    let put_url = config.create_url(&req.uri());

    let mut forwarded_req = client
        .request_from(put_url.as_str(), req.head())
        .timeout(TIMEOUT_DURATION);

    for header in &HEADERS_TO_REMOVE {
        forwarded_req
            .headers_mut()
            .remove(header);
    }

    let stream_to_send: Box<Stream<Item = _, Error = _>> = if config_ref.noop {
        Box::new(payload)
    } else {
        Box::new(Encoder::new(config_ref.key.clone(), config.chunk_size, Box::new(payload)))
    };

    forwarded_req
        .send_stream(stream_to_send)
        .map_err(Error::from)
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
    payload: web::Payload,
    client: web::Data<Client>,
    config: web::Data<Config>,
) -> impl Future<Item = HttpResponse, Error = Error> {
    let get_url=  config.get_ref().create_url(&req.uri());

    client
        .request_from(get_url.as_str(), req.head())
        .timeout(TIMEOUT_DURATION)
        .send_stream(payload)
        .map_err(Error::from)
        .map(move |res| {
            let mut client_resp = HttpResponse::build(res.status());
            for (header_name, header_value) in
                res.headers().iter().filter(|(h, _)| *h != "connection")
            {
                client_resp.header(header_name.clone(), header_value.clone());
            }

            if config.get_ref().noop {
                client_resp.streaming(res)
            } else {
                let decoder = Decoder::new(config.get_ref().key.clone(), Box::new(res));
                client_resp.streaming(decoder)
            }
        })
}

fn options(
    req: HttpRequest,
    payload: web::Payload,
    client: web::Data<Client>,
    config: web::Data<Config>,
) -> impl Future<Item = HttpResponse, Error = Error> {
    let options_url = config.get_ref().create_url(&req.uri());

    client
        .request_from(options_url.as_str(), req.head())
        .timeout(TIMEOUT_DURATION)
        .send_stream(payload)
        .map_err(Error::from)
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

fn default(_req: HttpRequest) -> impl IntoFuture<Item = &'static str, Error = Error> {
    Ok("Hello world!\r\n")
}

pub fn main(
    config: Config,
) -> std::io::Result<()> {

    let address = config.address.clone().unwrap();

    HttpServer::new(move || {
        App::new()
            .data(actix_web::client::Client::new())
            .data(config.clone())
            .wrap(middleware::Logger::default())
            .service(web::resource(".*").guard(guard::Get()).to_async(fetch))
            .service(web::resource(".*").guard(guard::Put()).to_async(forward))
            .service(web::resource(".*").guard(guard::Options()).to_async(options))
            .default_service(web::route().to_async(default))
    })
    .bind(address)?
    .system_exit()
    .run()
}

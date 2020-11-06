use super::config::Config;
use super::decoder::*;
use super::encoder::*;
use actix_web::client::Client;
use actix_web::guard;
use actix_web::http::header;
use actix_web::{middleware, web, App, Error, HttpRequest, HttpResponse, HttpServer};
use futures_core::stream::Stream;
use log::error;
use std::time::Duration;

const TIMEOUT_DURATION: Duration = Duration::from_secs(60 * 60);

static FORWARD_REQUEST_HEADERS_TO_REMOVE: [header::HeaderName; 3] = [
    // Connection settings (keepalived) must not be resend
    header::CONNECTION,
    // Encryption changes the length of the content
    header::CONTENT_LENGTH,
    // The awc client does not handle expect header
    // https://github.com/actix/actix-web/issues/1775
    header::EXPECT,
];

static FORWARD_RESPONSE_HEADERS_TO_REMOVE: [header::HeaderName; 2] = [
    // Connection settings (keepalived) must not be resend
    header::CONNECTION,
    // Encryption changes the length of the content
    // and we use chunk transfert-encoding
    header::CONTENT_LENGTH,
];

static FETCH_REQUEST_HEADERS_TO_REMOVE: [header::HeaderName; 1] = [
    // Connection settings (keepalived) must not be resend
    header::CONNECTION,
];

static FETCH_RESPONSE_HEADERS_TO_REMOVE: [header::HeaderName; 3] = [
    // We cannot honor accept ranges as the initial key
    // is located at the beginning of the stream and
    // the content is chunked encrypted
    header::ACCEPT_RANGES,
    // Connection settings (keepalived) must not be resend
    header::CONNECTION,
    // Encryption changes the length of the content
    // and we use chunk transfert-encoding
    header::CONTENT_LENGTH,
];

async fn ping() -> HttpResponse {
    let mut response = match std::env::current_dir() {
        Ok(path_buff) => {
            if path_buff.join("maintenance").exists() {
                HttpResponse::NotFound()
            } else {
                HttpResponse::Ok()
            }
        }

        // the server cannot even read a directory
        Err(_) => HttpResponse::InternalServerError(),
    };

    response
        .set_header(header::CONTENT_TYPE, "application/json")
        .body("{}")
}

async fn forward(
    req: HttpRequest,
    payload: web::Payload,
    client: web::Data<Client>,
    config: web::Data<Config>,
) -> Result<HttpResponse, Error> {
    let put_url = config.create_url(&req.uri());

    let mut forwarded_req = client
        .request_from(put_url.as_str(), req.head())
        .timeout(TIMEOUT_DURATION);

    for header in &FORWARD_REQUEST_HEADERS_TO_REMOVE {
        forwarded_req.headers_mut().remove(header);
    }

    let stream_to_send: Box<dyn Stream<Item = _> + Unpin> = if config.noop {
        Box::new(payload)
    } else {
        Box::new(Encoder::new(
            config.key.clone(),
            config.chunk_size,
            Box::new(payload),
        ))
    };

    forwarded_req
        .send_stream(stream_to_send)
        .await
        .map_err(Error::from)
        .map(|res| {
            if res.status().is_client_error() || res.status().is_server_error() {
                error!("forward error {:?} {:?}", req, res);
            }

            let mut client_resp = HttpResponse::build(res.status());

            for (header_name, header_value) in res
                .headers()
                .iter()
                .filter(|(h, _)| !FORWARD_RESPONSE_HEADERS_TO_REMOVE.contains(&h))
            {
                client_resp.header(header_name.clone(), header_value.clone());
            }

            client_resp.streaming(res)
        })
}

async fn fetch(
    req: HttpRequest,
    payload: web::Payload,
    client: web::Data<Client>,
    config: web::Data<Config>,
) -> Result<HttpResponse, Error> {
    let get_url = config.create_url(&req.uri());

    let mut fetch_req = client
        .request_from(get_url.as_str(), req.head())
        .timeout(TIMEOUT_DURATION);

    for header in &FETCH_REQUEST_HEADERS_TO_REMOVE {
        fetch_req.headers_mut().remove(header);
    }

    fetch_req
        .send_stream(payload)
        .await
        .map_err(Error::from)
        .map(move |res| {
            if res.status().is_client_error() || res.status().is_server_error() {
                error!("fetch error {:?} {:?}", req, res);
            }

            let mut client_resp = HttpResponse::build(res.status());

            for (header_name, header_value) in res
                .headers()
                .iter()
                .filter(|(h, _)| !FETCH_RESPONSE_HEADERS_TO_REMOVE.contains(&h))
            {
                client_resp.header(header_name.clone(), header_value.clone());
            }

            if config.noop {
                client_resp.streaming(res)
            } else {
                let decoder = Decoder::new(config.key.clone(), Box::new(res));
                client_resp.streaming(decoder)
            }
        })
}

async fn simple_proxy(
    req: HttpRequest,
    payload: web::Payload,
    client: web::Data<Client>,
    config: web::Data<Config>,
) -> Result<HttpResponse, Error> {
    let url = config.create_url(&req.uri());

    let mut proxied_req = client
        .request_from(url.as_str(), req.head())
        .timeout(TIMEOUT_DURATION);

    for header in &FETCH_REQUEST_HEADERS_TO_REMOVE {
        proxied_req.headers_mut().remove(header);
    }

    proxied_req
        .send_stream(payload)
        .await
        .map_err(Error::from)
        .map(|res| {
            if res.status().is_client_error() || res.status().is_server_error() {
                error!("simple proxy error {:?} {:?}", req, res);
            }

            let mut client_resp = HttpResponse::build(res.status());

            for (header_name, header_value) in res
                .headers()
                .iter()
                .filter(|(h, _)| !FETCH_RESPONSE_HEADERS_TO_REMOVE.contains(&h))
            {
                client_resp.header(header_name.clone(), header_value.clone());
            }

            client_resp.streaming(res)
        })
}

#[actix_rt::main]
pub async fn main(config: Config) -> std::io::Result<()> {
    let address = config.address.unwrap();
    let max_conn = config.max_connections;

    HttpServer::new(move || {
        App::new()
            .data(actix_web::client::Client::new())
            .data(config.clone())
            .wrap(middleware::Logger::default())
            .service(web::resource("/ping").guard(guard::Get()).to(ping))
            .service(web::resource(".*").guard(guard::Get()).to(fetch))
            .service(web::resource(".*").guard(guard::Put()).to(forward))
            .default_service(web::route().to(simple_proxy))
    })
    .max_connections(max_conn)
    .bind_uds("/tmp/actix-uds.socket")?
    .bind(address)?
    .run()
    .await
}

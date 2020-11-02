use super::config::Config;
use super::decoder::*;
use super::encoder::*;
use actix_web::client::Client;
use actix_web::guard;
use actix_web::{middleware, web, App, Error, HttpRequest, HttpResponse, HttpServer};
use futures_core::stream::Stream;
use log::error;
use std::time::Duration;

const TIMEOUT_DURATION: Duration = Duration::from_secs(60 * 60);

// Encryption changes the value of those headers
static HEADERS_TO_REMOVE: [actix_web::http::header::HeaderName; 3] = [
    actix_web::http::header::CONTENT_LENGTH,
    actix_web::http::header::CONTENT_TYPE,
    actix_web::http::header::ETAG,
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
        .set_header(actix_web::http::header::CONTENT_TYPE, "application/json")
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

    for header in &HEADERS_TO_REMOVE {
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
            for (header_name, header_value) in
                res.headers().iter().filter(|(h, _)| *h != "connection")
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

    client
        .request_from(get_url.as_str(), req.head())
        .timeout(TIMEOUT_DURATION)
        .send_stream(payload)
        .await
        .map_err(Error::from)
        .map(move |res| {
            if res.status().is_client_error() || res.status().is_server_error() {
                error!("fetch error {:?} {:?}", req, res);
            }

            let mut client_resp = HttpResponse::build(res.status());

            if let Some(original_length)  = res.headers().get(actix_web::http::header::CONTENT_LENGTH) {
                let content_length = original_length.to_str().unwrap().parse().unwrap();
                client_resp.no_chunking(decrypted_content_length(content_length, config.chunk_size) as u64);
            }

            for (header_name, header_value) in
                res.headers().iter().filter(|(h, _)| !(*h == "connection" || *h == "content-length"))
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
    let options_url = config.create_url(&req.uri());

    client
        .request_from(options_url.as_str(), req.head())
        .timeout(TIMEOUT_DURATION)
        .send_stream(payload)
        .await
        .map_err(Error::from)
        .map(|res| {
            if res.status().is_client_error() || res.status().is_server_error() {
                error!("simple proxy error {:?} {:?}", req, res);
            }

            let mut client_resp = HttpResponse::build(res.status());
            for (header_name, header_value) in
                res.headers().iter().filter(|(h, _)| *h != "connection")
            {
                client_resp.header(header_name.clone(), header_value.clone());
            }
            client_resp.streaming(res)
        })
}


fn decrypted_content_length(encrypted_length: usize, chunk_length: usize) -> usize {
    use super::header::HEADER_SIZE;
    use sodiumoxide::crypto::secretstream::xchacha20poly1305::{HEADERBYTES, ABYTES};

    // encrypted = HEADER_DS + HEADER_CRYPTO + n * ( ABYTES + CHUNK ) + (ABYTES + REMAIN)

    let nb_chunk = ((encrypted_length - HEADER_SIZE - HEADERBYTES) as f64 / (ABYTES + chunk_length) as f64).ceil() as usize;

    encrypted_length - HEADER_SIZE - HEADERBYTES - nb_chunk * ABYTES
}

#[actix_rt::main]
pub async fn main(config: Config) -> std::io::Result<()> {
    let address = config.address.unwrap();

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
    .bind_uds("/tmp/actix-uds.socket")?
    .bind(address)?
    .run()
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decrypt_content_length() {
        assert_eq!(1233793024, decrypted_content_length(1235073281, 16 * 1024));
        assert_eq!(5882, decrypted_content_length(6345, 256));
    }
}

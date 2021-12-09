use super::config::Config;
use super::decipher_type::DecipherType;
use super::decoder::*;
use super::encoder::*;
use super::header::HEADER_SIZE;
use super::header_decoder::*;
use super::partial_extractor::*;
use actix_files::HttpRange;
use actix_web::body::SizedStream;
use actix_web::client::Client;
use actix_web::guard;
use actix_web::http::{header, HeaderMap};
use actix_web::web::Bytes;
use actix_web::{middleware, web, App, Error, HttpRequest, HttpResponse, HttpServer};
use futures::TryStreamExt;
use futures_core::stream::Stream;
use log::error;
use sodiumoxide::crypto::secretstream::xchacha20poly1305::{ABYTES, HEADERBYTES};
use std::time::Duration;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(1);
const RESPONSE_TIMEOUT: Duration = Duration::from_secs(30);
const UPLOAD_TIMEOUT: Duration = Duration::from_secs(60 * 60);

static FORWARD_REQUEST_HEADERS_TO_REMOVE: [header::HeaderName; 4] = [
    // Connection settings (keepalived) must not be resend
    header::CONNECTION,
    // Encryption changes the length of the content
    header::CONTENT_LENGTH,
    // Openstack checks the ETAG header as a md5 checksum of the data
    // the encryption change the data and thus the etag
    header::ETAG,
    // The awc client does not handle expect header
    // https://github.com/actix/actix-web/issues/1775
    header::EXPECT,
];

static FORWARD_RESPONSE_HEADERS_TO_REMOVE: [header::HeaderName; 1] = [
    // Connection settings (keepalived) must not be resend
    header::CONNECTION,
];

static FETCH_REQUEST_HEADERS_TO_REMOVE: [header::HeaderName; 2] = [
    // Connection settings (keepalived) must not be resend
    header::CONNECTION,
    header::RANGE,
];

static FETCH_RESPONSE_HEADERS_TO_REMOVE: [header::HeaderName; 2] = [
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
        .force_close()
        .timeout(UPLOAD_TIMEOUT);

    let forward_length: Option<usize> = content_length(req.headers()).map(|content_length| {
        if config.noop {
            content_length
        } else {
            encrypted_content_length(content_length, config.chunk_size)
        }
    });

    for header in &FORWARD_REQUEST_HEADERS_TO_REMOVE {
        forwarded_req.headers_mut().remove(header);
    }

    let stream: Box<dyn Stream<Item = _> + Unpin> = if config.noop {
        Box::new(payload)
    } else {
        Box::new(Encoder::new(
            config.key.clone(),
            config.chunk_size,
            Box::new(payload),
        ))
    };

    let req_copy = req.clone();
    let stream_to_send = stream.map_err(move |e| {
        error!("forward error with stream {:?}, {:?}", e, req_copy);
        Error::from(e)
    });

    let res_e = if let Some(length) = forward_length {
        forwarded_req
            .send_body(SizedStream::new(length as u64, stream_to_send))
            .await
    } else {
        forwarded_req.send_stream(stream_to_send).await
    };

    let mut res = res_e.map_err(|e| {
        error!("forward fwk error {:?}, {:?}", e, req);
        Error::from(e)
    })?;

    if res.status().is_client_error() || res.status().is_server_error() {
        error!("forward status error {:?} {:?}", req, res);
    }

    let mut client_resp = HttpResponse::build(res.status());

    for (header_name, header_value) in res
        .headers()
        .iter()
        .filter(|(h, _)| !FORWARD_RESPONSE_HEADERS_TO_REMOVE.contains(&h))
    {
        client_resp.header(header_name.clone(), header_value.clone());
    }

    Ok(client_resp.body(res.body().await?))
}

async fn fetch(
    req: HttpRequest,
    body: web::Bytes,
    client: web::Data<Client>,
    config: web::Data<Config>,
) -> Result<HttpResponse, Error> {
    let get_url = config.create_url(&req.uri());

    let mut fetch_req = client
        .request_from(get_url.as_str(), req.head())
        .force_close();

    let raw_range = req
        .headers()
        .get(header::RANGE)
        .and_then(|l| l.to_str().ok());

    for header in &FETCH_REQUEST_HEADERS_TO_REMOVE {
        fetch_req.headers_mut().remove(header);
    }

    let res = fetch_req.send_body(body).await.map_err(|e| {
        error!("fetch fwk error {:?}, {:?}", e, req);
        Error::from(e)
    })?;

    if res.status().is_client_error() || res.status().is_server_error() {
        error!("fetch status error {:?} {:?}", req, res);
    }

    let mut client_resp = HttpResponse::build(res.status());

    for (header_name, header_value) in res
        .headers()
        .iter()
        .filter(|(h, _)| !FETCH_RESPONSE_HEADERS_TO_REMOVE.contains(&h))
    {
        client_resp.header(header_name.clone(), header_value.clone());
    }

    let original_length = content_length(res.headers());

    if config.noop {
        if let Some(length) = original_length {
            Ok(client_resp.no_chunking(length as u64).streaming(res))
        } else {
            Ok(client_resp.streaming(res))
        }
    } else {
        let mut boxy: Box<dyn Stream<Item = Result<Bytes, _>> + Unpin> = Box::new(res);
        let header_decoder = HeaderDecoder::new(&mut boxy);
        let (cypher_type, buff) = header_decoder.await;
        let fetch_length = original_length
            .map(|content_length| decrypted_content_length(content_length, cypher_type));

        let decoder =
            Decoder::new_from_cypher_and_buffer(config.key.clone(), boxy, cypher_type, buff);

        if let Some(length) = fetch_length {
            use std::convert::TryInto;

            let range = match raw_range {
                Some(r) => Some(HttpRange::parse(r, length.try_into().unwrap())),
                _ => None,
            };

            match range {
                Some(Ok(v)) => {
                    let r = v.first().unwrap();

                    let range_start = r.start.try_into().unwrap();
                    let range_end = (r.start + r.length - 1).try_into().unwrap();

                    let pe = PartialExtractor::new(Box::new(decoder), range_start, range_end);

                    client_resp.header(
                        header::CONTENT_RANGE,
                        format!("bytes {}-{}/{}", range_start, range_end, length),
                    );

                    return Ok(client_resp.no_chunking(r.length as u64).streaming(pe));
                }
                _ => {
                    return Ok(client_resp.no_chunking(length as u64).streaming(decoder));
                }
            }
        } else {
            Ok(client_resp.streaming(decoder))
        }
    }
}

async fn simple_proxy(
    req: HttpRequest,
    payload: web::Payload,
    client: web::Data<Client>,
    config: web::Data<Config>,
) -> Result<HttpResponse, Error> {
    let url = config.create_url(&req.uri());

    let mut proxied_req = client.request_from(url.as_str(), req.head()).force_close();

    for header in &FETCH_REQUEST_HEADERS_TO_REMOVE {
        proxied_req.headers_mut().remove(header);
    }

    proxied_req
        .send_stream(payload)
        .await
        .map_err(|e| {
            error!("simple proxy fwk error {:?}, {:?}", e, req);
            Error::from(e)
        })
        .map(|res| {
            if res.status().is_client_error() || res.status().is_server_error() {
                error!("simple proxy status error {:?} {:?}", req, res);
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

fn content_length(headers: &HeaderMap) -> Option<usize> {
    headers
        .get(header::CONTENT_LENGTH)
        .and_then(|l| l.to_str().ok())
        .and_then(|s| s.parse::<usize>().ok())
}

fn encrypted_content_length(clear_length: usize, chunk_size: usize) -> usize {
    if clear_length == 0 {
        return 0;
    }

    let nb_chunk = clear_length / chunk_size;
    let remainder = clear_length % chunk_size;

    if remainder == 0 {
        HEADER_SIZE + HEADERBYTES + nb_chunk * (ABYTES + chunk_size)
    } else {
        HEADER_SIZE + HEADERBYTES + nb_chunk * (ABYTES + chunk_size) + ABYTES + remainder
    }
}

fn decrypted_content_length(encrypted_length: usize, decipher: DecipherType) -> usize {
    if encrypted_length == 0 {
        return 0;
    }

    match decipher {
        DecipherType::Encrypted { chunk_size } => {
            // encrypted = header_ds + header_crypto + n ( abytes + chunk ) + a (abytes + remainder)
            // with remainder < chunk and a = 0 if remainder = 0, a = 1 otherwise
            //
            //  encrypted - header_ds - header_crypto = n ( abytes + chunk ) + a (abytes + remainder)
            //
            //  integer_part ((encrypted - header_ds - header_crypto) / ( abytes + chunk ))
            //    = integer_part ( n + a (abytes + remainder) / (abytes + chunk) )
            //    = n

            let nb_chunk = (encrypted_length - HEADER_SIZE - HEADERBYTES) / (ABYTES + chunk_size);
            let remainder_exists =
                (encrypted_length - HEADER_SIZE - HEADERBYTES) % (ABYTES + chunk_size) != 0;

            if remainder_exists {
                encrypted_length - HEADER_SIZE - HEADERBYTES - (nb_chunk + 1) * ABYTES
            } else {
                encrypted_length - HEADER_SIZE - HEADERBYTES - nb_chunk * ABYTES
            }
        }

        DecipherType::Plaintext => encrypted_length,
    }
}

#[actix_web::main]
pub async fn main(config: Config) -> std::io::Result<()> {
    let address = config.address.unwrap();
    let max_conn = config.max_connections;

    use actix_http;

    HttpServer::new(move || {
        App::new()
            .data(
                actix_web::client::ClientBuilder::new()
                    .connector(
                        actix_web::client::Connector::new()
                            .timeout(CONNECT_TIMEOUT) // max time to connect to remote host including dns name resolution
                            .finish(),
                    )
                    .timeout(RESPONSE_TIMEOUT) // the total time before a response must be received
                    .finish(),
            )
            .data(config.clone())
            .wrap(middleware::Logger::default())
            .service(web::resource("/ping").guard(guard::Get()).to(ping))
            .service(web::resource(".*").guard(guard::Get()).to(fetch))
            .service(web::resource(".*").guard(guard::Put()).to(forward))
            .default_service(web::route().to(simple_proxy))
    })
    .max_connections(max_conn)
    .keep_alive(actix_http::KeepAlive::Disabled)
    .bind_uds("/tmp/actix-uds.socket")?
    .bind(address)?
    .run()
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decrypt_content_length_from_0() {
        let original_length = 0;
        let chunk_size = 16;
        let encrypted_length = 0;

        let decrypted_length = decrypted_content_length(
            encrypted_length,
            DecipherType::Encrypted {
                chunk_size: chunk_size,
            },
        );

        assert_eq!(original_length, decrypted_length);
    }

    #[test]
    fn test_decrypt_content_length_without_remainder() {
        let original_length = 32;
        let chunk_size = 16;
        let nb_chunk = 32 / 16;
        let encrypted_length = HEADER_SIZE + HEADERBYTES + nb_chunk * (ABYTES + chunk_size);

        let decrypted_length = decrypted_content_length(
            encrypted_length,
            DecipherType::Encrypted {
                chunk_size: chunk_size,
            },
        );

        assert_eq!(original_length, decrypted_length);
    }

    #[test]
    fn test_decrypt_content_length_with_remainder() {
        let original_length = 33;
        let chunk_size = 16;
        let nb_chunk = 32 / 16;
        let encrypted_length =
            HEADER_SIZE + HEADERBYTES + nb_chunk * (ABYTES + chunk_size) + (ABYTES + 1);

        let decrypted_length = decrypted_content_length(
            encrypted_length,
            DecipherType::Encrypted {
                chunk_size: chunk_size,
            },
        );

        assert_eq!(original_length, decrypted_length);
    }

    #[test]
    fn test_decrypt_content_length_with_another_exemple() {
        let original_length = 5882;
        let encrypted_length = 6345;

        let decrypted_length = decrypted_content_length(
            encrypted_length,
            DecipherType::Encrypted { chunk_size: 256 },
        );

        assert_eq!(original_length, decrypted_length);
    }

    #[test]
    fn test_encrypted_content_length_from_0() {
        let original_length = 0;
        let chunk_size = 16;
        let encrypted_length = 0;

        assert_eq!(
            encrypted_length,
            encrypted_content_length(original_length, chunk_size)
        );
    }

    #[test]
    fn test_encrypted_content_length_without_remainder() {
        let original_length = 32;
        let chunk_size = 16;
        let nb_chunk = 32 / 16;
        let encrypted_length = HEADER_SIZE + HEADERBYTES + nb_chunk * (ABYTES + chunk_size);

        assert_eq!(
            encrypted_length,
            encrypted_content_length(original_length, chunk_size)
        );
    }

    #[test]
    fn test_encrypted_content_length_with_remainder() {
        let original_length = 33;
        let chunk_size = 16;
        let nb_chunk = 32 / 16;
        let encrypted_length =
            HEADER_SIZE + HEADERBYTES + nb_chunk * (ABYTES + chunk_size) + (ABYTES + 1);

        assert_eq!(
            encrypted_length,
            encrypted_content_length(original_length, chunk_size)
        );
    }

    #[test]
    fn test_encrypted_content_length_with_another_exemple() {
        let original_length = 5882;
        let encrypted_length = 6345;
        let chunk_size = 256;

        assert_eq!(
            encrypted_length,
            encrypted_content_length(original_length, chunk_size)
        );
    }
}

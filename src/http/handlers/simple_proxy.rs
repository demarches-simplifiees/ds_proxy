use data_encoding::HEXLOWER;
use sha2::{Digest, Sha256};

use crate::http::utils::aws_helper::sign_request;

use super::*;

pub async fn simple_proxy(
    req: HttpRequest,
    payload: web::Payload,
    client: web::Data<Client>,
    config: web::Data<HttpConfig>,
) -> Result<HttpResponse, Error> {
    let url = config.create_upstream_url(&req);

    if url.is_none() {
        return not_found();
    }

    let mut proxied_req = client.request_from(url.unwrap(), req.head()).force_close();

    for header in &FETCH_REQUEST_HEADERS_TO_REMOVE {
        proxied_req.headers_mut().remove(header);
    }

    let req_to_send = if config.aws_access_key.is_some() {
        let checksum = HEXLOWER.encode(&Sha256::digest(b""));

        sign_request(
            proxied_req,
            &config.aws_access_key.clone().unwrap(),
            &config.aws_secret_key.clone().unwrap(),
            &config.aws_region.clone().unwrap(),
            &checksum,
        )
    } else {
        proxied_req
    };

    log::info!("simple proxy forwarding request {:?}", req_to_send);

    req_to_send
        .send_stream(payload)
        .await
        .map_err(|e| {
            error!("simple proxy fwk error {:?}, {:?}", e, req);
            actix_web::error::ErrorBadGateway(e)
        })
        .map(|res| {
            log::info!("simple proxy received response {:?}", res);

            if res.status().is_client_error() || res.status().is_server_error() {
                error!("simple proxy status error {:?} {:?}", req, res);
            }

            let mut client_resp = HttpResponse::build(res.status());

            for header in res.headers().iter()
            // .filter(|(h, _)| !FETCH_RESPONSE_HEADERS_TO_REMOVE.contains(h))
            {
                client_resp.append_header(header);
            }

            // client_resp.streaming(res)
            client_resp.finish()
        })
}

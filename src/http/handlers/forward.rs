use super::*;
use actix_web::body::SizedStream;
use std::time::Duration;

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

pub async fn forward(
    req: HttpRequest,
    payload: web::Payload,
    client: web::Data<Client>,
    config: web::Data<HttpConfig>,
) -> Result<HttpResponse, Error> {
    let put_url = config.create_upstream_url(&req);

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
        let (key_id, key) = config
            .keyring
            .get_last_key()
            .expect("no key avalaible for encryption");

        Box::new(Encoder::new(
            key,
            key_id,
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
        actix_web::error::ErrorBadGateway(e)
    })?;

    if res.status().is_client_error() || res.status().is_server_error() {
        error!("forward status error {:?} {:?}", req, res);
    }

    let mut client_resp = HttpResponse::build(res.status());

    for header in res
        .headers()
        .iter()
        .filter(|(h, _)| !FORWARD_RESPONSE_HEADERS_TO_REMOVE.contains(h))
    {
        client_resp.append_header(header);
    }

    Ok(client_resp.body(res.body().await?))
}

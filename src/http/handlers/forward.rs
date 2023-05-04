use crate::http::utils::{aws_helper::sign_request, memory_or_file_buffer::MemoryOrFileBuffer};

use super::*;
use actix_web::body::SizedStream;
use futures::StreamExt;
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

    if put_url.is_none() {
        return not_found();
    }

    let mut forwarded_req = client
        .request_from(put_url.unwrap(), req.head())
        .force_close()
        .timeout(UPLOAD_TIMEOUT);

    let forward_length: Option<usize> = content_length(req.headers())
        .map(|content_length| encrypted_content_length(content_length, config.chunk_size));

    for header in &FORWARD_REQUEST_HEADERS_TO_REMOVE {
        forwarded_req.headers_mut().remove(header);
    }

    let (key_id, key) = config
        .keyring
        .get_last_key()
        .expect("no key avalaible for encryption");

    let mut encrypted_stream = Encoder::new(key, key_id, config.chunk_size, Box::new(payload));

    let cloned_req = req.clone();

    let mut input_etag: Option<String> = None;

    let res_e = if config.aws_access_key.is_some() {
        let filepath = config.local_encryption_path_for(&req);
        let mut buffer = MemoryOrFileBuffer::new(filepath);

        while let Ok(Some(v)) = encrypted_stream.try_next().await {
            buffer.append(v).await;
        }

        let (output_sha256, length) = buffer.sha256_and_len();
        input_etag = Some(encrypted_stream.input_md5());

        let stream_to_send = buffer.to_stream().await;

        sign_request(
            forwarded_req,
            &config.aws_access_key.clone().unwrap(),
            &config.aws_secret_key.clone().unwrap(),
            &config.aws_region.clone().unwrap(),
            &output_sha256,
        )
        .send_body(SizedStream::new(length, stream_to_send))
        .await
    } else {
        let stream_to_send = encrypted_stream
            .map_err(move |e| {
                error!("forward error with stream {:?}, {:?}", e, cloned_req);
                Error::from(e)
            })
            .boxed_local();

        if let Some(length) = forward_length {
            forwarded_req
                .send_body(SizedStream::new(length as u64, stream_to_send))
                .await
        } else {
            forwarded_req.send_stream(stream_to_send).await
        }
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

    if let Some(etag) = input_etag {
        client_resp.insert_header(("etag", format!("\"{}\"", etag)));
    }

    Ok(client_resp.body(res.body().await?))
}

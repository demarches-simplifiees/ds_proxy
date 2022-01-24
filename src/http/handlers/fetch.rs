use super::*;
use crate::http::utils::partial_extractor::*;
use actix_files::HttpRange;
use actix_web::web::Bytes;

pub async fn fetch(
    req: HttpRequest,
    body: web::Bytes,
    client: web::Data<Client>,
    config: web::Data<Config>,
) -> Result<HttpResponse, Error> {
    let get_url = config.create_url(req.uri());

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
        error!("fetch error {:?}, {:?}", e, req);
        match e {
            awc::error::SendRequestError::Timeout => actix_web::error::ErrorGatewayTimeout(e),
            _ => actix_web::error::ErrorBadGateway(e),
        }
    })?;

    if res.status().is_client_error() || res.status().is_server_error() {
        error!("fetch status error {:?} {:?}", req, res);
    }

    let mut client_resp = HttpResponse::build(res.status());

    for header in res
        .headers()
        .iter()
        .filter(|(h, _)| !FETCH_RESPONSE_HEADERS_TO_REMOVE.contains(h))
    {
        client_resp.append_header(header);
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

            let range = raw_range.map(|r| HttpRange::parse(r, length.try_into().unwrap()));

            match range {
                Some(Ok(v)) => {
                    let r = v.first().unwrap();

                    let range_start = r.start.try_into().unwrap();
                    let range_end = (r.start + r.length - 1).try_into().unwrap();

                    let pe = PartialExtractor::new(Box::new(decoder), range_start, range_end);

                    client_resp.append_header((
                        header::CONTENT_RANGE,
                        format!("bytes {}-{}/{}", range_start, range_end, length),
                    ));

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

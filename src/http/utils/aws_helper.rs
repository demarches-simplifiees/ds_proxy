use crate::http::utils::sign::*;
use actix_http::header::{HeaderName, HeaderValue};
use awc::ClientRequest;

pub fn sign_request(
    mut req: ClientRequest,
    aws_access_key: &str,
    aws_secret_key: &str,
    aws_region: &str,
    checksum: &str,
) -> ClientRequest {
    let datetime = chrono::Utc::now();

    let host = req.get_uri().host().unwrap();
    let amz_date = datetime.format("%Y%m%dT%H%M%SZ").to_string();

    let amz_headers: Vec<(&HeaderName, &HeaderValue)> = req
        .headers()
        .iter()
        .filter(|(key, _)| key.to_string().starts_with("x-amz-"))
        .collect();

    log::info!("voila les amz: {:?}", amz_headers);

    let mut map = http::HeaderMap::new();

    for (header_name, header_value) in amz_headers {
        map.insert::<http::HeaderName>(header_name.into(), header_value.into());
    }
    map.insert("x-amz-date", amz_date.parse().unwrap());
    map.insert("x-amz-content-sha256", checksum.parse().unwrap());
    map.insert("host", host.parse().unwrap());

    let authorization = AwsSign::new(
        req.get_method().as_str(),
        &req.get_uri().to_string(),
        &datetime,
        &map,
        aws_region,
        aws_access_key,
        aws_secret_key,
        "s3",
        checksum,
    )
    .sign();

    for (key, value) in map {
        req = req.insert_header((key.unwrap().to_string(), value.to_str().unwrap()));
    }

    req.insert_header(("Authorization", authorization))
}

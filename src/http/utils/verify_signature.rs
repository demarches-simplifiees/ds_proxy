use actix_web::HttpRequest;
use aws_sdk_s3::config::Credentials;
use aws_sigv4::http_request::{sign, SignableRequest, SignatureLocation, SigningSettings};
use aws_sigv4::http_request::{SignableBody, SigningParams};
use aws_sigv4::sign::v4;
use chrono::{DateTime, NaiveDateTime, Utc};
use std::collections::HashMap;
use std::time::{Duration, SystemTime};

pub fn is_signature_valid(
    request: &HttpRequest,
    aws_access_key: &str,
    aws_secret_key: &str,
    aws_region: &str,
) -> bool {
    let identity = Credentials::new(
        aws_access_key,
        aws_secret_key,
        None,
        None,
        "hardcoded-credentials",
    )
    .into();

    let mut url = request.full_url();
    let http_method = request.method().as_str();
    let headers = request.headers();

    let (_, mut aws_params) = remove_aws_query_params(&url);

    let aws_headers = headers.iter()
        .filter(|(key, _)| {
            let key_lower = key.as_str().to_lowercase();
            key_lower.starts_with("x-amz-") || key_lower == "authorization"
        })
        .map(|(key, value)| {
            (
                key.as_str().to_lowercase(),
                value.to_str().unwrap_or("").to_string(),
            )
        });

    aws_params.extend(aws_headers);

    let url = clean_url(&full_url);

    let mut settings = SigningSettings::default();
    settings.expires_in = aws_req_params
        .get("x-amz-expires")
        .and_then(|s| s.parse::<u64>().ok())
        .map(Duration::from_secs);
    settings.signature_location = SignatureLocation::QueryParams;

    let time = parse_amz_date(
        aws_req_params
            .get("x-amz-date")
            .expect("Missing x-amz-date"),
    );

    let signing_params: SigningParams = v4::SigningParams::builder()
        .identity(&identity)
        .region(aws_region)
        .name("s3")
        .time(time)
        .settings(settings)
        .build()
        .unwrap()
        .into();

    let signed_headers: Vec<(String, String)> = aws_req_params
        .get("x-amz-signedheaders")
        .map(|s| s.split(';'))
        .into_iter()
        .flatten()
        .filter(|&h| h != "host")
        .filter_map(|h| aws_req_params.get(h).map(|v| (h.to_string(), v.clone())))
        .collect();

    let mut url = request.full_url();
    url.set_query(None);
    url.set_fragment(None);
    let url_string = url.to_string();

    let signable = SignableRequest::new(
        http_method,
        &url,
        signed_headers.iter().map(|(k, v)| (k.as_str(), v.as_str())),
        SignableBody::UnsignedPayload,
    )
    .unwrap();

    let (_, expected_signature) = sign(signable, &params).unwrap().into_parts();

    let signature = aws_req_params
        .get("x-amz-signature")
        .expect("Missing x-amz-signature");

    signature == &expected_signature
}

fn parse_amz_date(date_str: &str) -> SystemTime {
    let naive = NaiveDateTime::parse_from_str(date_str, "%Y%m%dT%H%M%SZ")
        .expect("Invalid x-amz-date format");
    DateTime::<Utc>::from_naive_utc_and_offset(naive, Utc).into()
}

fn clean_url(full_url: &str) -> String {
    let mut parsed = Url::parse(full_url).expect("Invalid URL");
    parsed.set_query(None);
    parsed.set_fragment(None);
    parsed.to_string()
}

#[cfg(test)]
mod tests {
    use actix_web::test::TestRequest;

    use super::*;
    #[test]
    fn test_verify_signature() {
        let access_key = "an_access_key";
        let secret_key = "a_secret_key";
        let aws_region = "eu-west-1";

        let uri = "/upstream/drive-media-storage/item/2b5a76ad-4bfb-4f32-9b6d-ebdd999d3711/test.txt?q=p&x-amz-algorithm=AWS4-HMAC-SHA256&x-amz-signature=1695606b1548dc5e8819c3a0276951ac12fb3ef58861d3f31d05c8359a06b1ef&x-amz-credential=an_access_key%2F20251113%2Feu-west-1%2Fs3%2Faws4_request&x-amz-date=20251113T155445Z&x-amz-expires=60&x-amz-signedheaders=host%3Bx-amz-acl";

        let request = TestRequest::put()
            .uri(uri)
            .insert_header(("host", "localhost:4444"))
            .insert_header(("x-amz-acl", "private"))
            .to_http_request();

        let is_valid = is_signature_valid(&request, access_key, secret_key, aws_region);

        assert!(is_valid);
    }
}

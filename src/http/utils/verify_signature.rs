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

    let mut params: HashMap<String, String> = request
        .full_url()
        .query_pairs()
        .map(|(k, v)| (k.to_lowercase(), v.to_string()))
        .collect();

    request.headers().iter().for_each(|(k, v)| {
        params.insert(
            k.as_str().to_lowercase(),
            v.to_str().unwrap_or("").to_string(),
        );
    });

    let settings = if presigned_url(&params) {
        let expires_in = params
            .get("x-amz-expires")
            .and_then(|s| s.parse::<u64>().ok())
            .map(Duration::from_secs);

        let mut settings = SigningSettings::default();
        settings.signature_location = SignatureLocation::QueryParams;
        settings.expires_in = expires_in;

        settings
    } else {
        let mut settings = SigningSettings::default();
        settings.signature_location = SignatureLocation::Headers;

        settings
    };

    let amz_date = params.get("x-amz-date").expect("Missing x-amz-date");
    let time = parse_amz_date(amz_date);

    let signing_params: SigningParams = v4::SigningParams::builder()
        .identity(&identity)
        .region(aws_region)
        .name("s3")
        .time(time)
        .settings(settings)
        .build()
        .unwrap()
        .into();

    let signed = signed_headers(&params);
    log::trace!("Signed headers: {:?}", signed);

    let body = if let Some(sha) = params.get("x-amz-content-sha256") {
        SignableBody::Precomputed(sha.to_string())
    } else {
        SignableBody::UnsignedPayload
    };

    let mut url = request.full_url();
    url.set_query(None);
    url.set_fragment(None);
    let url_string = url.to_string();

    let signable = SignableRequest::new(
        request.method().as_str(),
        &url_string,
        signed.iter().map(|(k, v)| (k.as_str(), v.as_str())),
        body,
    )
    .unwrap();

    let (_, expected_signature) = sign(signable, &signing_params).unwrap().into_parts();

    expected_signature == extract_signature(&params)
}

fn parse_amz_date(date_str: &str) -> SystemTime {
    let naive = NaiveDateTime::parse_from_str(date_str, "%Y%m%dT%H%M%SZ")
        .expect("Invalid x-amz-date format");
    DateTime::<Utc>::from_naive_utc_and_offset(naive, Utc).into()
}

fn presigned_url(aws_params: &HashMap<String, String>) -> bool {
    aws_params.contains_key("x-amz-signature")
}

fn signed_headers(aws_params: &HashMap<String, String>) -> Vec<(String, String)> {
    let header_list = if presigned_url(aws_params) {
        aws_params
            .get("x-amz-signedheaders")
            .expect("Missing x-amz-signedheaders")
            .as_str()
    } else {
        aws_params
            .get("authorization")
            .expect("Missing Authorization header")
            .split(',')
            .find(|part| part.trim().starts_with("SignedHeaders="))
            .expect("Missing SignedHeaders in Authorization header")
            .trim()
            .trim_start_matches("SignedHeaders=")
    };

    header_list
        .split(';')
        .filter_map(|h| aws_params.get(h).map(|v| (h.to_string(), v.clone())))
        .collect()
}

fn extract_signature(aws_params: &HashMap<String, String>) -> String {
    if presigned_url(aws_params) {
        aws_params
            .get("x-amz-signature")
            .expect("Missing x-amz-signature")
            .to_string()
    } else {
        let authorization = aws_params
            .get("authorization")
            .expect("Missing Authorization header");
        authorization
            .split(',')
            .map(|part| part.trim())
            .find(|part| part.starts_with("Signature="))
            .map(|sig| sig.trim_start_matches("Signature=").to_string())
            .expect("Missing Signature in Authorization header")
    }
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

    #[test]
    fn test_header_signature() {
        let access_key = "an_access_key";
        let secret_key = "a_secret_key";
        let aws_region = "eu-west-1";

        let uri = "/upstream/drive-media-storage/item/29f00a79-b2ff-49a4-b0d5-814863d21ea8/18-11-2025-a-18h35.ics";

        let request = TestRequest::get()
            .uri(uri)
            .insert_header(("host", "937d7186e461.ngrok-free.app"))
            .insert_header(("authorization", "AWS4-HMAC-SHA256 Credential=an_access_key/20251117/eu-west-1/s3/aws4_request, SignedHeaders=host;x-amz-content-sha256;x-amz-date, Signature=e09656ef6781f03e8eacd0c5a98c18c4a884254982b8a0043201aa6838e8792c"))
            .insert_header(("x-amz-content-sha256", "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"))
            .insert_header(("x-amz-date", "20251117T151958Z"))
            .to_http_request();

        let is_valid = is_signature_valid(&request, access_key, secret_key, aws_region);

        assert!(is_valid);
    }

    #[test]
    fn another_test_header_signature() {
        let _ = env_logger::try_init();
        let access_key = "an_access_key";
        let secret_key = "a_secret_key";
        let aws_region = "eu-west-1";

        let uri =
            "/upstream/drive-media-storage/item/969fd250-d647-48d7-a0b9-705f2cf4069c/test.txt";

        let request = TestRequest::get()
            .uri(uri)
            .insert_header(("authorization", "AWS4-HMAC-SHA256 Credential=an_access_key/20251118/eu-west-1/s3/aws4_request, SignedHeaders=host;range;x-amz-checksum-mode;x-amz-content-sha256;x-amz-date, Signature=df8a2df04aea3cec93826f42a38e55a13f74b63680fada05d5203cb05df9fbef"))
            .insert_header(("x-amz-content-sha256", "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"))
            .insert_header(("x-amz-checksum-mode", "ENABLED"))
            .insert_header(("range", "bytes=0-2047"))
            .insert_header(("x-amz-date", "20251118T135750Z"))
            .insert_header(("host", "c0f16bdf2fc8.ngrok-free.app"))
            .to_http_request();

        let is_valid = is_signature_valid(&request, access_key, secret_key, aws_region);

        assert!(is_valid);
    }
}

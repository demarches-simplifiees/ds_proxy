use crate::http::utils::sign::*;
use actix_http::header::{HeaderName, HeaderValue};
use awc::ClientRequest;
use std::{collections::HashMap, str::FromStr};
use url::Url;

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
        let header_name = http::HeaderName::from_str(header_name.as_str()).unwrap();
        let header_value = http::HeaderValue::from_str(header_value.to_str().unwrap()).unwrap();
        map.insert(header_name, header_value);
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

pub fn remove_aws_query_params(url: &Url) -> (String, HashMap<String, String>) {
    let mut aws_params = HashMap::new();
    let mut other_params = HashMap::new();

    url.query_pairs().for_each(|(key, value)| {
        if key.to_lowercase().starts_with("x-amz-") {
            aws_params.insert(key.to_lowercase(), value.to_string());
        } else {
            other_params.insert(key.to_string(), value.to_string());
        }
    });

    let mut parsed_url = url.clone();
    parsed_url.set_query(None);

    other_params.iter().for_each(|(k, v)| {
        parsed_url.query_pairs_mut().append_pair(k, v);
    });

    (parsed_url.to_string(), aws_params)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_remove_aws_query_params() {
        let url = Url::parse("https://example.com/path/to/resource?X-Amz-Algorithm=algo&X-Amz-Credential=credential&other_param=value").unwrap();
        let (cleaned_url, aws_params) = remove_aws_query_params(&url);

        assert_eq!(
            cleaned_url,
            "https://example.com/path/to/resource?other_param=value"
        );

        let expected_params = HashMap::from([
            ("x-amz-algorithm".to_string(), "algo".to_string()),
            ("x-amz-credential".to_string(), "credential".to_string()),
        ]);

        assert_eq!(aws_params, expected_params);
    }
}

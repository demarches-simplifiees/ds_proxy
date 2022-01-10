use actix_web::http::{header, header::HeaderMap};

pub mod partial_extractor;

pub fn content_length(headers: &HeaderMap) -> Option<usize> {
    headers
        .get(header::CONTENT_LENGTH)
        .and_then(|l| l.to_str().ok())
        .and_then(|s| s.parse::<usize>().ok())
}

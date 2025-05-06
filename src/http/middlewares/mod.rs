use super::super::config::HttpConfig;
use crate::write_once_service::WriteOnceService;
use actix_web::{
    body::MessageBody,
    dev::{ServiceRequest, ServiceResponse},
    error::ErrorForbidden,
    middleware::Next,
    web, Error,
};
use std::path::Path;

pub async fn ensure_write_once(
    req: ServiceRequest,
    next: Next<impl MessageBody>,
) -> Result<ServiceResponse<impl MessageBody>, Error> {
    let uri_string = req.uri().to_string();
    let uri: &str = uri_string.as_str();

    let write_once_service = req
        .app_data::<web::Data<WriteOnceService>>()
        .unwrap()
        .clone();

    // key was set before, early return and deny access because we only write once
    match write_once_service.is_locked(uri).await {
        Ok(true) => {
            log::warn!("Access denied: Redis key already exists: {}", uri);
            return Err(ErrorForbidden("Access denied"));
        }
        Ok(false) => {} // Key does not exist, proceed
        Err(_) => {}    // don't mind about redis errors
    }

    // proceed with the request
    let result = next.call(req).await;
    if let Ok(ref response) = result {
        if response.status().is_success() {
            if let Err(err) = write_once_service.mark_as_locked(uri).await {
                log::error!(
                    "Failed to mark as locked with expiration: {}. Error: {}",
                    uri,
                    err
                );
            }
        }
    }

    result
}

pub fn erase_file(res: Result<ServiceResponse, Error>) -> Result<ServiceResponse, Error> {
    let response = res.unwrap();
    let request = response.request();

    let filepath = request
        .app_data::<web::Data<HttpConfig>>()
        .unwrap()
        .local_encryption_path_for(request);

    if Path::new(&filepath).exists() {
        std::fs::remove_file(filepath).unwrap();
    }

    Ok(response)
}

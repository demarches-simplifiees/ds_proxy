use super::super::config::HttpConfig;
use crate::redis_service::RedisService;
use actix_web::{
    body::MessageBody,
    dev::{ServiceRequest, ServiceResponse},
    error::ErrorForbidden,
    middleware::Next,
    web, Error,
};
use deadpool_redis::Pool;
use sha2::{Digest, Sha256};
use std::path::Path;

const REDIS_KEY_EXPIRATION: u64 = 3600; // 1 hour

pub fn hash_key(input: &str) -> String {
    format!("{:x}", Sha256::digest(input))
}

pub async fn ensure_write_once(
    req: ServiceRequest,
    next: Next<impl MessageBody>,
) -> Result<ServiceResponse<impl MessageBody>, Error> {
    let path = req.path().to_string();
    let redis_service = RedisService::new(req.app_data::<web::Data<Pool>>().cloned(), path);

    // key was set before, early return and deny access because we only write once
    match redis_service.check_key().await {
        Ok(true) => {
            log::warn!(
                "Access denied: Redis key already exists: {}",
                redis_service.path
            );
            return Err(ErrorForbidden("Access denied"));
        }
        Ok(false) => {} // Key does not exist, proceed
        Err(_) => {}    // don't mind about redis errors
    }

    // proceed with the request
    let result = next.call(req).await;

    // set key key
    if let Err(err) = redis_service.set_temp_key(REDIS_KEY_EXPIRATION).await {
        log::error!(
            "Failed to set Redis key with expiration: {}. Error: {}",
            redis_service.path,
            err
        );
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

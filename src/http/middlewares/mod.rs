use super::super::config::HttpConfig;
use crate::redis_utils::{check_redis_key, get_redis_connection, set_redis_key_with_expiration};
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


fn hash_key(input: &str) -> String {
    format!("{:x}", Sha256::digest(input))
}

pub async fn ensure_write_once(
    req: ServiceRequest,
    next: Next<impl MessageBody>,
) -> Result<ServiceResponse<impl MessageBody>, Error> {
    let redis_key = hash_key(req.path());

    // pool not configured, proceed with request
    let redis_pool = match req.app_data::<web::Data<Pool>>() {
        Some(pool) => pool,
        None => return next.call(req).await,
    };

    // connection not available, proceed with request
    let mut conn = match get_redis_connection(redis_pool).await {
        Some(conn) => conn,
        None => return next.call(req).await,
    };

    // key was set before, early return and deny access because we only write once
    match check_redis_key(&mut conn, &redis_key).await {
        Ok(true) => {
            log::warn!("Access denied: Redis key already exists: {}", redis_key);
            return Err(ErrorForbidden("Access denied"));
        }
        Ok(false) => {} // Key does not exist, proceed
        Err(_) => {}    // don't mind about redis errors
    }

    // proceed with the request
    let result = next.call(req).await;

    // set key key
    if let Err(err) =
        set_redis_key_with_expiration(&mut conn, &redis_key, REDIS_KEY_EXPIRATION).await
    {
        log::error!(
            "Failed to set Redis key with expiration: {}. Error: {}",
            redis_key,
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

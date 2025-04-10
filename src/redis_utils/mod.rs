use crate::config::HttpConfig;
use actix_web::web;
use deadpool_redis::{Config as RedisConfig, Pool as RedisPool, Pool, Runtime};
use log::{info, warn};
use redis::AsyncCommands;
use url::Url;

pub async fn configure_redis_pool(config: &HttpConfig) -> Option<RedisPool> {
    if config.write_once.is_some() {
        if let Some(ref redis_url) = config.redis_url {
            log::info!("Redis URL provided: {:?}", redis_url);

            match create_redis_pool(config.redis_url.clone()).await {
                Some(pool) => {
                    return Some(pool);
                }
                None => {
                    panic!("An accessibl Redis URL is required when write-once is enabled.");
                }
            }
        }
        panic!("Redis URL is required when write-once is enabled.");
    } else {
        log::info!("Write-once is not enabled, skipping Redis pool creation.");
    }
    None
}

pub async fn create_redis_pool(url: Option<Url>) -> Option<RedisPool> {
    match url {
        Some(url) => {
            let cfg = RedisConfig::from_url(url.to_string());

            match cfg.create_pool(Some(Runtime::Tokio1)) {
                Ok(redis_pool) => Some(redis_pool),
                Err(err) => {
                    warn!("Invalid Redis URL : {}", err);
                    None
                }
            }
        }
        None => {
            info!("No Redis URL provided, skipping Redis pool creation.");
            None
        }
    }
}

pub async fn get_redis_connection(
    redis_pool: &web::Data<Pool>,
) -> Option<deadpool_redis::Connection> {
    match redis_pool.get().await {
        Ok(conn) => Some(conn),
        Err(_) => {
            log::warn!("Failed to get Redis connection.");
            None
        }
    }
}

pub async fn check_redis_key(
    conn: &mut deadpool_redis::Connection,
    redis_key: &str,
) -> Result<bool, String> {
    conn.exists(redis_key).await.map_err(|e| e.to_string())
}

pub async fn set_redis_key_with_expiration(
    conn: &mut deadpool_redis::Connection,
    redis_key: &str,
    expire: u64,
) -> Result<(), String> {
    conn.set_ex(redis_key, true, expire)
        .await
        .map_err(|e| e.to_string())
}

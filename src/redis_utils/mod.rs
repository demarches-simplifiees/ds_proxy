use crate::config::HttpConfig;
use deadpool_redis::{Config as RedisConfig, Pool as RedisPool, Pool, Runtime};
use log::{info, warn};
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

use crate::config::RedisConfig;
use deadpool::managed::{QueueMode, Timeouts};
use deadpool_redis::{Config, Pool, PoolConfig, Runtime};
use log::{info, warn};

pub fn configure_redis_pool(redis_config: &RedisConfig) -> Pool {
    if let Some(ref redis_url) = redis_config.redis_url {
        log::info!("Redis URL provided: {:?}", redis_url);
        match create_redis_pool(redis_config) {
            Some(pool) => {
                return pool;
            }
            None => {
                panic!("An accessibl Redis URL is required when write-once is enabled.");
            }
        }
    }
    panic!("Redis URL is required when write-once is enabled.");
}

fn get_redis_pool_config(config: &RedisConfig) -> PoolConfig {
    PoolConfig {
        max_size: config.redis_pool_max_size.unwrap_or(16),
        timeouts: Timeouts {
            wait: config
                .redis_timeout_wait
                .or(Some(std::time::Duration::from_secs(5))),
            create: config
                .redis_timeout_create
                .or(Some(std::time::Duration::from_secs(3))),
            recycle: config
                .redis_timeout_recycle
                .or(Some(std::time::Duration::from_secs(1))),
        },
        queue_mode: QueueMode::Fifo, // default queue mode
    }
}

pub fn create_redis_pool(redis_config: &RedisConfig) -> Option<Pool> {
    match redis_config.redis_url.as_ref() {
        Some(url) => {
            let pool_config = get_redis_pool_config(redis_config);

            let mut cfg = Config::from_url(url.to_string());
            cfg.pool = Some(pool_config);

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

use crate::config::RedisConfig;
use deadpool_redis::{Config, Pool, Runtime};

pub fn configure_redis_pool(redis_config: RedisConfig) -> Pool {
    log::info!("Redis URL provided: {:?}", redis_config.redis_url);

    let mut cfg = Config::from_url(redis_config.redis_url.clone());
    cfg.pool = Some(redis_config.pool_config);

    cfg.create_pool(Some(Runtime::Tokio1))
        .unwrap_or_else(|err| panic!("Failed to create Redis pool: {}", err))
}

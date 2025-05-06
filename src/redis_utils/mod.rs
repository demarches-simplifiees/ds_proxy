use crate::config::RedisConfig;
use deadpool::managed::{QueueMode, Timeouts};
use deadpool_redis::{Config, Pool, PoolConfig, Runtime};

pub fn configure_redis_pool(redis_config: &RedisConfig) -> Pool {
    log::info!("Redis URL provided: {:?}", redis_config.redis_url);
    let pool_config = get_redis_pool_config(redis_config);

    let mut cfg = Config::from_url(redis_config.redis_url.clone());
    cfg.pool = Some(pool_config);

    cfg.create_pool(Some(Runtime::Tokio1))
        .unwrap_or_else(|err| panic!("Failed to create Redis pool: {}", err))
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

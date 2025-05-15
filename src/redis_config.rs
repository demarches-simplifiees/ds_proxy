use deadpool_redis::{PoolConfig, Timeouts};
use std::env;
use std::time::Duration;
use url::Url;

use super::args;

#[derive(Debug, Clone)]
pub struct RedisConfig {
    pub url: Url,
    pub pool_config: PoolConfig,
}

impl Default for RedisConfig {
    fn default() -> Self {
        Self {
            url: Url::parse("redis://127.0.0.1").unwrap(),
            pool_config: PoolConfig {
                timeouts: Timeouts {
                    wait: Some(Duration::from_millis(200)),
                    create: Some(Duration::from_millis(200)),
                    recycle: Some(Duration::from_millis(200)),
                },
                ..PoolConfig::default()
            },
        }
    }
}

impl RedisConfig {
    pub fn create_redis_config(args: &args::Args) -> RedisConfig {
        let default_config = RedisConfig::default();

        RedisConfig {
            url: match &args.flag_redis_url {
                Some(redis_url) => redis_url.clone(),
                None => match env::var("REDIS_URL") {
                    Ok(redis_url_string) => Url::parse(&redis_url_string)
                        .expect("Invalid Redis URL from environment variable"),
                    _ => default_config.url,
                },
            },
            pool_config: PoolConfig {
                max_size: match &args.flag_redis_pool_max_size {
                    Some(max_size) => *max_size,
                    None => match env::var("REDIS_POOL_MAX_SIZE") {
                        Ok(max_size_string) => max_size_string
                            .parse::<usize>()
                            .expect("REDIS_POOL_MAX_SIZE is not a valid usize"),
                        _ => default_config.pool_config.max_size,
                    },
                },
                queue_mode: default_config.pool_config.queue_mode,
                timeouts: Timeouts {
                    wait: match &args.flag_redis_timeout_wait {
                        Some(timeout) => Some(Duration::from_secs(*timeout)),
                        None => match env::var("REDIS_TIMEOUT_WAIT") {
                            Ok(timeout_string) => Some(Duration::from_secs(
                                timeout_string
                                    .parse::<u64>()
                                    .expect("REDIS_TIMEOUT_WAIT is not a valid u64"),
                            )),
                            _ => default_config.pool_config.timeouts.wait,
                        },
                    },
                    create: match &args.flag_redis_timeout_create {
                        Some(timeout) => Some(Duration::from_secs(*timeout)),
                        None => match env::var("REDIS_TIMEOUT_CREATE") {
                            Ok(timeout_string) => Some(Duration::from_secs(
                                timeout_string
                                    .parse::<u64>()
                                    .expect("REDIS_TIMEOUT_CREATE is not a valid u64"),
                            )),
                            _ => default_config.pool_config.timeouts.create,
                        },
                    },
                    recycle: match &args.flag_redis_timeout_recycle {
                        Some(timeout) => Some(Duration::from_secs(*timeout)),
                        None => match env::var("REDIS_TIMEOUT_RECYCLE") {
                            Ok(timeout_string) => Some(Duration::from_secs(
                                timeout_string
                                    .parse::<u64>()
                                    .expect("REDIS_TIMEOUT_RECYCLE is not a valid u64"),
                            )),
                            _ => default_config.pool_config.timeouts.recycle,
                        },
                    },
                },
            },
        }
    }
}

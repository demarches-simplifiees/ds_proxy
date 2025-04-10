use actix_web::web;
use deadpool_redis::Pool;
use md5::Digest;
use redis::AsyncCommands;

pub struct RedisService {
    pool: Option<web::Data<Pool>>,
    pub path: String,
}

pub fn hash_key(input: &str) -> String {
    format!("{:x}", sha2::Sha256::digest(input))
}

impl RedisService {
    pub fn new(pool: Option<web::Data<Pool>>, path: String) -> Self {
        RedisService { pool, path }
    }

    pub async fn check_key(&self) -> Result<bool, String> {
        self.get_redis_connection()
            .await?
            .exists(hash_key(&self.path))
            .await
            .map_err(|e| e.to_string())
            .map(|exists: i32| exists > 0)
    }

    pub async fn set_temp_key(&self, expire: u64) -> Result<(), String> {
        self.get_redis_connection()
            .await?
            .set_ex(hash_key(&self.path), true, expire)
            .await
            .map_err(|e| e.to_string())
    }

    async fn get_redis_connection(&self) -> Result<deadpool_redis::Connection, String> {
        let redis_pool = match &self.pool {
            Some(pool) => pool.clone(),
            None => return Err("Redis pool is not initialized.".to_string()),
        };
        let conn = match redis_pool.get().await {
            Ok(conn) => conn,
            Err(e) => return Err(format!("Failed to get Redis connection: {}", e)),
        };
        Ok(conn)
    }
}

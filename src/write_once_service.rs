use actix_web::web;
use deadpool_redis::Pool;
use redis::AsyncCommands;

const LOCK_DURATION: u64 = 3600; // 1 hour

use sha2::{Digest, Sha256};


// Service that implements the "write once" functionality
// This service uses Redis to track resource paths that have been successfully
// accessed, preventing multiple accesses to the same resource. This is especially
// useful for temporary URLs that should only be valid for a single use.

pub struct WriteOnceService {
    pool: Option<web::Data<Pool>>,
    pub path: String,
}

impl WriteOnceService {
    pub fn new(pool: Option<web::Data<Pool>>, path: String) -> Self {
        WriteOnceService { pool, path }
    }

    pub fn hash_key(path: &str) -> String {
        format!("{:x}", Sha256::digest(path.as_bytes()))
    }

    pub async fn is_locked(&self) -> Result<bool, String> {
        self.get_redis_connection()
            .await?
            .exists(Self::hash_key(&self.path))
            .await
            .map_err(|e| e.to_string())
            .map(|exists: i32| exists > 0)
    }

    pub async fn mark_as_locked(&self) -> Result<(), String> {
        self.get_redis_connection()
            .await?
            .set_ex(Self::hash_key(&self.path), true, LOCK_DURATION)
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

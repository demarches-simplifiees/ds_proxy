use deadpool_redis::Pool;
use redis::AsyncCommands;

const LOCK_DURATION: u64 = 3600; // 1 hour

use sha2::{Digest, Sha256};

// Service that implements the "write once" functionality
// This service uses Redis to track resource uris that have been successfully
// accessed, preventing multiple accesses to the same resource. This is especially
// useful for temporary URLs that should only be valid for a single use.

#[derive(Clone)]
pub struct WriteOnceService {
    pool: Pool,
}

impl WriteOnceService {
    pub fn new(pool: Pool) -> Self {
        WriteOnceService { pool }
    }

    pub fn hash_key(uri: &str) -> String {
        format!("{:x}", Sha256::digest(uri.as_bytes()))
    }

    pub async fn is_locked(&self, uri: &str) -> Result<bool, String> {
        self.get_redis_connection()
            .await?
            .exists(Self::hash_key(uri))
            .await
            .map_err(|e| e.to_string())
            .map(|exists: i32| exists > 0)
    }

    pub async fn mark_as_locked(&self, uri: &str) -> Result<(), String> {
        self.get_redis_connection()
            .await?
            .set_ex(Self::hash_key(uri), true, LOCK_DURATION)
            .await
            .map_err(|e| e.to_string())
    }

    async fn get_redis_connection(&self) -> Result<deadpool_redis::Connection, String> {
        self.pool
            .get()
            .await
            .map_err(|e| format!("Failed to get Redis connection: {}", e))
    }
}

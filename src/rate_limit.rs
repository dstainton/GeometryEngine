use redis::AsyncCommands;
use crate::error::ApiError;
use crate::cache::Cache;

pub struct RateLimiter {
    cache: Cache,
    requests_per_minute: u32,
}

impl RateLimiter {
    pub fn new(cache: Cache, requests_per_minute: u32) -> Self {
        Self {
            cache,
            requests_per_minute,
        }
    }

    pub async fn check_rate_limit(&self, api_key: &str) -> Result<(), ApiError> {
        let mut conn = self.cache.get_connection().await?;
        let key = format!("rate_limit:{}", api_key);

        let count: Option<u32> = conn.get(&key).await.map_err(|e| ApiError::Cache(e.to_string()))?;
        let current = count.unwrap_or(0);

        if current >= self.requests_per_minute {
            return Err(ApiError::RateLimitExceeded);
        }

        // Increment counter and set expiry
        let _: () = conn.incr(&key, 1).await.map_err(|e| ApiError::Cache(e.to_string()))?;
        let _: () = conn.expire(&key, 60).await.map_err(|e| ApiError::Cache(e.to_string()))?;

        Ok(())
    }
} 
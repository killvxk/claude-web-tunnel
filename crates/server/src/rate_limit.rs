//! Rate limiting module using Redis

use anyhow::Result;
use deadpool_redis::{Pool, Connection};
use redis::AsyncCommands;

/// Rate limiter configuration
#[derive(Clone)]
pub struct RateLimiter {
    pool: Pool,
    /// Maximum requests per minute
    limit_per_minute: u32,
    /// Window size in seconds
    window_seconds: u64,
}

impl RateLimiter {
    /// Create a new rate limiter
    pub fn new(pool: Pool, limit_per_minute: u32) -> Self {
        Self {
            pool,
            limit_per_minute,
            window_seconds: 60,
        }
    }

    /// Check if the request is within rate limit
    /// Returns Ok(true) if allowed, Ok(false) if rate limited
    pub async fn check_limit(&self, key: &str) -> Result<bool> {
        let mut conn = self.get_connection().await?;
        let redis_key = format!("rate_limit:{}", key);

        // Increment counter
        let count: u32 = conn.incr(&redis_key, 1).await?;

        // Set expiry on first request
        if count == 1 {
            conn.expire::<_, ()>(&redis_key, self.window_seconds as i64).await?;
        }

        Ok(count <= self.limit_per_minute)
    }

    /// Get current request count for a key
    #[allow(dead_code)]
    pub async fn get_count(&self, key: &str) -> Result<u32> {
        let mut conn = self.get_connection().await?;
        let redis_key = format!("rate_limit:{}", key);

        let count: Option<u32> = conn.get(&redis_key).await?;
        Ok(count.unwrap_or(0))
    }

    /// Get remaining requests for a key
    #[allow(dead_code)]
    pub async fn get_remaining(&self, key: &str) -> Result<u32> {
        let count = self.get_count(key).await?;
        Ok(self.limit_per_minute.saturating_sub(count))
    }

    /// Reset rate limit for a key (useful for testing)
    #[allow(dead_code)]
    pub async fn reset(&self, key: &str) -> Result<()> {
        let mut conn = self.get_connection().await?;
        let redis_key = format!("rate_limit:{}", key);
        conn.del::<_, ()>(&redis_key).await?;
        Ok(())
    }

    async fn get_connection(&self) -> Result<Connection> {
        self.pool.get().await.map_err(|e| anyhow::anyhow!("Failed to get Redis connection: {}", e))
    }
}

/// Initialize Redis connection pool
pub async fn init_redis(url: &str) -> Result<Pool> {
    let cfg = deadpool_redis::Config::from_url(url);
    let pool = cfg.create_pool(Some(deadpool_redis::Runtime::Tokio1))?;

    // Test connection
    let mut conn = pool.get().await?;
    let _: String = redis::cmd("PING").query_async(&mut conn).await?;

    tracing::info!("Connected to Redis");
    Ok(pool)
}

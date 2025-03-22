use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use deadpool_redis::{Config, Pool, Runtime};
use redis::{AsyncCommands, RedisError};
use serde::{de::DeserializeOwned, Serialize};
use std::time::Duration;
use tracing::{debug, error, info};

pub struct RedisManager {
    pool: Pool,
    default_ttl: Duration,
}

impl RedisManager {
    pub async fn new(
        url: &str,
        default_ttl_seconds: u64,
        max_connections: usize,
    ) -> Result<Self> {
        let cfg = Config::from_url(url);
        let pool = cfg
            .create_pool(Some(Runtime::Tokio1))
            .context("Failed to create Redis connection pool")?;
        
        // Test connection
        let mut conn = pool.get().await?;
        redis::cmd("PING").query_async(&mut conn).await?;
        
        info!("Connected to Redis successfully");
        
        Ok(Self {
            pool,
            default_ttl: Duration::from_secs(default_ttl_seconds),
        })
    }
    
    // Set a key with serialized value and TTL
    pub async fn set<T: Serialize>(&self, key: &str, value: &T, ttl: Option<Duration>) -> Result<()> {
        let serialized = serde_json::to_string(value)?;
        let mut conn = self.pool.get().await?;
        
        match ttl {
            Some(ttl) => {
                conn.set_ex(key, serialized, ttl.as_secs() as usize).await?;
            },
            None => {
                conn.set_ex(key, serialized, self.default_ttl.as_secs() as usize).await?;
            }
        }
        
        Ok(())
    }
    
    // Get and deserialize a value by key
    pub async fn get<T: DeserializeOwned>(&self, key: &str) -> Result<Option<T>> {
        let mut conn = self.pool.get().await?;
        let result: Option<String> = conn.get(key).await?;
        
        match result {
            Some(val) => {
                let deserialized = serde_json::from_str(&val)?;
                Ok(Some(deserialized))
            },
            None => Ok(None),
        }
    }
    
    // Check if a key exists
    pub async fn exists(&self, key: &str) -> Result<bool> {
        let mut conn = self.pool.get().await?;
        let result: i32 = conn.exists(key).await?;
        Ok(result == 1)
    }
    
    // Delete a key
    pub async fn delete(&self, key: &str) -> Result<bool> {
        let mut conn = self.pool.get().await?;
        let result: i32 = conn.del(key).await?;
        Ok(result == 1)
    }
    
    // Set expiration time on key
    pub async fn expire(&self, key: &str, ttl: Duration) -> Result<bool> {
        let mut conn = self.pool.get().await?;
        let result: bool = conn.expire(key, ttl.as_secs() as usize).await?;
        Ok(result)
    }
    
    // Get array data with caching
    pub async fn get_or_set_array<T, F>(
        &self,
        key: &str,
        ttl: Option<Duration>,
        fetch_fn: F,
    ) -> Result<Vec<T>>
    where
        T: Serialize + DeserializeOwned,
        F: FnOnce() -> Result<Vec<T>>,
    {
        // Try to get from cache first
        if let Some(cached) = self.get::<Vec<T>>(key).await? {
            debug!("Cache hit for key: {}", key);
            return Ok(cached);
        }
        
        // If not in cache, fetch the data
        debug!("Cache miss for key: {}", key);
        let data = fetch_fn()?;
        
        // Store in cache
        if !data.is_empty() {
            self.set(key, &data, ttl).await?;
        }
        
        Ok(data)
    }
    
    // Cache key for candle data
    pub fn candle_data_key(symbol: &str, interval: &str) -> String {
        format!("candles:{}:{}", symbol, interval)
    }
    
    // Cache key for calculated indicator
    pub fn indicator_key(
        symbol: &str,
        interval: &str,
        indicator_name: &str,
        parameters_json: &str,
    ) -> String {
        format!(
            "indicator:{}:{}:{}:{}",
            symbol, interval, indicator_name, parameters_json
        )
    }
    
    // Cache key for intermediate calculation results
    pub fn intermediate_key(
        symbol: &str,
        interval: &str,
        calculation_type: &str,
        parameters_json: &str,
    ) -> String {
        format!(
            "intermediate:{}:{}:{}:{}",
            symbol, interval, calculation_type, parameters_json
        )
    }
}

// src/worker.rs
use crate::cache::redis::RedisManager;
use crate::database::postgres::PostgresManager;
use crate::processor::worker::{Worker, WorkerConfig};
use crate::talib_bindings::TaLibAbstract;
use anyhow::Result;
use num_cpus;
use std::env;
use std::sync::Arc;
use tracing::{info, error};

/// Start the worker process
pub async fn start_worker(concurrency: Option<usize>) -> Result<()> {
    info!("Starting Technical Indicator Calculator with TA-Lib Direct Functions and Completeness Caching");
    
    // Initialize TA-Lib
    match TaLibAbstract::initialize() {
        Ok(_) => info!("TA-Lib successfully initialized"),
        Err(e) => {
            error!("Failed to initialize TA-Lib: {}", e);
            return Err(anyhow::anyhow!("TA-Lib initialization failed"));
        }
    }
    
    // Check if TA-Lib functions are available
    if !TaLibAbstract::is_function_available("RSI") {
        error!("TA-Lib is not properly configured. RSI function not found.");
        error!("Please ensure TA-Lib is installed on your system.");
        return Err(anyhow::anyhow!("TA-Lib functions not available"));
    }
    
    info!("TA-Lib library found and initialized successfully");
    
    // Get database configuration
    let db_host = env::var("DB_HOST").unwrap_or_else(|_| "localhost".to_string());
    let db_port = env::var("DB_PORT").unwrap_or_else(|_| "5432".to_string()).parse::<u16>()?;
    let db_user = env::var("DB_USER").unwrap_or_else(|_| "binanceuser".to_string());
    let db_password = env::var("DB_PASSWORD").unwrap_or_else(|_| "binancepass".to_string());
    let db_name = env::var("DB_NAME").unwrap_or_else(|_| "binancedb".to_string());
    
    // Get Redis configuration
    let redis_url = env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379".to_string());
    
    // Determine concurrency
    let concurrency = concurrency.unwrap_or_else(|| {
        env::var("CONCURRENCY")
            .ok()
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or_else(|| num_cpus::get())
    });
    
    info!("Using concurrency level: {}", concurrency);
    
    // Create PostgreSQL connection
    let pg: Arc<PostgresManager> = Arc::new(
        PostgresManager::new(
            &db_host,
            db_port,
            &db_user,
            &db_password,
            &db_name,
            concurrency * 2, // Max connections in the pool
        )
        .await?
    );
    
    // Initialize database tables
    pg.init_tables().await?;
    
    // Create Redis connection
    let cache_ttl = env::var("CACHE_TTL_SECONDS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(3600); // Default 1 hour
    
    // Get completeness cache TTL
    let completeness_ttl = env::var("COMPLETENESS_CACHE_MINUTES")
        .ok()
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(30); // Default 30 minutes
    
    info!("Using completeness cache TTL: {} minutes", completeness_ttl);
    
    // Add explicit type annotation for RedisManager
    let redis: Arc<RedisManager> = Arc::new(
        RedisManager::new(
            &redis_url,
            cache_ttl,
            concurrency * 2, // Max connections in the pool
        )
        .await?
    );
    
    // Create worker configuration
    let worker_config = WorkerConfig {
        cache_ttl_seconds: cache_ttl,
        completeness_cache_minutes: completeness_ttl,
        batch_size: 1000,
        retry_max: 3,
        retry_delay_ms: 500,
    };
    
    // Create and start worker
    let worker = Worker::new(
        pg.clone(),
        redis.clone(),
        worker_config,
        concurrency,
    );
    
    // Start processing (this will block until the application is terminated)
    info!("Starting worker process with TA-Lib integration and completeness caching");
    worker.start().await?;
    
    info!("Technical Indicator Calculator shutting down");
    Ok(())
}

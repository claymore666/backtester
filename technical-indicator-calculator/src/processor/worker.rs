use crate::cache::redis::RedisManager;
use crate::database::models::{CalculatedIndicatorBatch, CandleData};
use crate::database::postgres::PostgresManager;
use crate::indicators::calculator::IndicatorCalculator;
use crate::processor::job::{CalculationJob, IndicatorType};
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, Semaphore};
use tracing::{debug, error, info, instrument, warn};

// Worker configuration
#[derive(Debug, Clone)]
pub struct WorkerConfig {
    pub cache_ttl_seconds: u64,
    pub batch_size: usize,
    pub retry_max: usize,
    pub retry_delay_ms: u64,
}

impl Default for WorkerConfig {
    fn default() -> Self {
        Self {
            cache_ttl_seconds: 3600, // 1 hour cache TTL
            batch_size: 1000,        // Number of indicators to batch insert
            retry_max: 3,            // Maximum retries
            retry_delay_ms: 500,     // Delay between retries
        }
    }
}

pub struct Worker {
    pg: Arc<PostgresManager>,
    redis: Arc<RedisManager>,
    config: WorkerConfig,
    concurrency_limit: usize,
}

impl Worker {
    pub fn new(
        pg: Arc<PostgresManager>,
        redis: Arc<RedisManager>,
        config: WorkerConfig,
        concurrency_limit: usize,
    ) -> Self {
        Self {
            pg,
            redis,
            config,
            concurrency_limit,
        }
    }

    pub async fn start(self) -> Result<()> {
        info!("Starting indicator calculation worker with concurrency limit: {}", self.concurrency_limit);
        
        // Set up channels
        let (job_tx, job_rx) = mpsc::channel(1000);
        
        // Spawn job producer
        tokio::spawn(self.clone().job_producer(job_tx));
        
        // Create a semaphore to limit concurrent processing
        let semaphore = Arc::new(Semaphore::new(self.concurrency_limit));
        
        // Process jobs in the main thread
        self.job_consumer(0, job_rx, semaphore).await?;
        
        info!("Technical Indicator Calculator shutting down");
        Ok(())
    }
    
    #[instrument(skip(self, job_tx))]
    async fn job_producer(self, job_tx: mpsc::Sender<CalculationJob>) -> Result<()> {
        info!("Started job producer");
        
        loop {
            // Get all enabled indicator configurations
            let configs = match self.pg.get_enabled_indicator_configs().await {
                Ok(configs) => configs,
                Err(e) => {
                    error!("Failed to get indicator configurations: {}", e);
                    tokio::time::sleep(Duration::from_secs(5)).await;
                    continue;
                }
            };
            
            info!("Found {} enabled indicator configurations", configs.len());
            
            // Process each configuration
            for config in configs {
                let indicator_type = IndicatorType::from(config.indicator_type.as_str());
                
                let job = CalculationJob::new(
                    config.symbol,
                    config.interval,
                    indicator_type,
                    config.indicator_name,
                    config.parameters,
                );
                
                // Get the last calculated time for this indicator
                let _last_time = match self.pg
                    .get_last_calculated_time(
                        &job.symbol,
                        &job.interval,
                        &job.indicator_name,
                        &job.parameters,
                    )
                    .await
                {
                    Ok(Some(time)) => Some(time),
                    Ok(None) => None,
                    Err(e) => {
                        error!("Failed to get last calculated time: {}", e);
                        continue;
                    }
                };
                
                // Check if job is already in cache (being processed)
                let job_key = job.cache_key();
                if let Ok(exists) = self.redis.exists(&job_key).await {
                    if exists {
                        debug!("Job already in progress, skipping: {}", job_key);
                        continue;
                    }
                }
                
                // Send job to workers
                if let Err(e) = job_tx.send(job.clone()).await {
                    error!("Failed to send job to workers: {}", e);
                }
                
                // Add job to cache to prevent duplicate processing
                if let Err(e) = self.redis
                    .set(
                        &job_key,
                        &json!({"status": "processing", "queued_at": Utc::now()}),
                        Some(Duration::from_secs(600)), // 10 minute TTL
                    )
                    .await
                {
                    warn!("Failed to cache job status: {}", e);
                }
            }
            
            // Sleep for a while before checking for new configurations
            tokio::time::sleep(Duration::from_secs(60)).await;
        }
    }
    
    #[instrument(skip(self, job_rx, semaphore), fields(worker_id = %worker_id))]
    async fn job_consumer(
        self,
        worker_id: usize,
        mut job_rx: mpsc::Receiver<CalculationJob>,
        semaphore: Arc<Semaphore>,
    ) -> Result<()> {
        info!("Started worker {}", worker_id);
        
        while let Some(job) = job_rx.recv().await {
            // Acquire permit from semaphore
            let _permit = semaphore.acquire().await?;
            
            info!("Worker {} processing job: {}:{}:{}", 
                  worker_id, job.symbol, job.interval, job.indicator_name);
            
            // Process the job
            if let Err(e) = self.process_job(&job).await {
                error!("Failed to process job: {}", e);
                
                // Release the job from cache so it can be retried
                let job_key = job.cache_key();
                if let Err(e) = self.redis.delete(&job_key).await {
                    warn!("Failed to remove failed job from cache: {}", e);
                }
            }
        }
        
        Ok(())
    }
    
    #[instrument(skip(self))]
    async fn process_job(&self, job: &CalculationJob) -> Result<()> {
        // Get candle data
        let data = self.pg.get_candle_data(&job.symbol, &job.interval).await?;
        
        if data.close.is_empty() {
            warn!("No candle data available for {}:{}", job.symbol, job.interval);
            return Ok(());
        }
        
        // Calculate the indicator using the TA-Lib abstract interface
        debug!("Calculating indicator {}:{}:{} using TA-Lib abstract interface", 
               job.symbol, job.interval, job.indicator_name);
        let results = self.calculate_indicator(job, &data).await?;
        
        if results.is_empty() {
            info!("No new indicator values calculated for {}:{}:{}", 
                 job.symbol, job.interval, job.indicator_name);
            return Ok(());
        }
        
        // Prepare batch for database insertion
        let mut batch = Vec::with_capacity(results.len());
        
        for (time, value) in results {
            batch.push(CalculatedIndicatorBatch {
                symbol: job.symbol.clone(),
                interval: job.interval.clone(),
                indicator_type: job.indicator_type.to_string(),
                indicator_name: job.indicator_name.clone(),
                parameters: job.parameters.clone(),
                time,
                value,
            });
            
            // Insert in batches
            if batch.len() >= self.config.batch_size {
                self.pg.insert_calculated_indicators_batch(batch.clone()).await?;
                batch.clear();
            }
        }
        
        // Insert any remaining indicators
        if !batch.is_empty() {
            self.pg.insert_calculated_indicators_batch(batch).await?;
        }
        
        // Remove job from cache
        let job_key = job.cache_key();
        if let Err(e) = self.redis.delete(&job_key).await {
            warn!("Failed to remove completed job from cache: {}", e);
        }
        
        info!("Successfully processed indicator {}:{}:{}", 
             job.symbol, job.interval, job.indicator_name);
        
        Ok(())
    }
    
    async fn calculate_indicator(
        &self,
        job: &CalculationJob,
        candle_data: &CandleData,
    ) -> Result<Vec<(DateTime<Utc>, Value)>> {
        // Get the TA-Lib function name for this indicator
        let ta_function_name = IndicatorCalculator::get_ta_function_name(&job.indicator_name);
        
        // Special handling for multi-output indicators that need extra processing
        match job.indicator_name.as_str() {
            "MACD" => {
                let fast_period = job.parameters["fast_period"].as_u64().unwrap_or(12) as usize;
                let slow_period = job.parameters["slow_period"].as_u64().unwrap_or(26) as usize;
                let signal_period = job.parameters["signal_period"].as_u64().unwrap_or(9) as usize;
                
                IndicatorCalculator::calculate_macd(
                    candle_data,
                    fast_period,
                    slow_period,
                    signal_period,
                )
            },
            "BBANDS" => {
                // Use the generic calculator for Bollinger Bands but process the results
                // to provide upper/middle/lower bands in the expected format
                let period = job.parameters["period"].as_u64().unwrap_or(20) as usize;
                let dev_up = job.parameters["deviation_up"].as_f64().unwrap_or(2.0);
                let dev_down = job.parameters["deviation_down"].as_f64().unwrap_or(2.0);
                
                // Customize the parameters for TA-Lib
                let params = json!({
                    "optInTimePeriod": period,
                    "optInNbDevUp": dev_up,
                    "optInNbDevDown": dev_down,
                    "optInMAType": 0  // Simple Moving Average
                });
                
                IndicatorCalculator::calculate_indicator(
                    candle_data,
                    "BBANDS",
                    &params,
                )
            },
            "STOCH" => {
                let k_period = job.parameters["k_period"].as_u64().unwrap_or(14) as usize;
                let d_period = job.parameters["d_period"].as_u64().unwrap_or(3) as usize;
                let slowing = job.parameters["slowing"].as_u64().unwrap_or(3) as usize;
                
                let params = json!({
                    "optInFastK_Period": k_period,
                    "optInSlowK_Period": slowing,
                    "optInSlowK_MAType": 0,
                    "optInSlowD_Period": d_period,
                    "optInSlowD_MAType": 0
                });
                
                IndicatorCalculator::calculate_indicator(
                    candle_data,
                    "STOCH",
                    &params,
                )
            },
            // Add other special cases as needed
            _ => {
                // Use the generic calculator for most indicators
                IndicatorCalculator::calculate_indicator(
                    candle_data,
                    &ta_function_name,
                    &job.parameters,
                )
            }
        }
    }
}

impl Clone for Worker {
    fn clone(&self) -> Self {
        Self {
            pg: Arc::clone(&self.pg),
            redis: Arc::clone(&self.redis),
            config: self.config.clone(),
            concurrency_limit: self.concurrency_limit,
        }
    }
}

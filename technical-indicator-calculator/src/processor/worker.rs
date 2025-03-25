use crate::cache::completeness::{CompletenessCache, CompletenessInfo, SharedCompletenessCache};
use crate::cache::completeness_controller::CompletenessController;
use crate::cache::redis::RedisManager;
use crate::database::models::{CalculatedIndicatorBatch, CandleData};
use crate::database::postgres::PostgresManager;
use crate::indicators::calculator::IndicatorCalculator;
use crate::processor::job::{CalculationJob, IndicatorType};
use crate::utils::log_utils::log_to_file;
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, Semaphore};
use tokio::time::Instant;
use tracing::{debug, error, info, instrument, warn};

// Worker configuration
#[derive(Debug, Clone)]
pub struct WorkerConfig {
    pub cache_ttl_seconds: u64,
    pub completeness_cache_minutes: i64,
    pub batch_size: usize,
    pub retry_max: usize,
    pub retry_delay_ms: u64,
}

impl Default for WorkerConfig {
    fn default() -> Self {
        Self {
            cache_ttl_seconds: 3600,           // 1 hour cache TTL
            completeness_cache_minutes: 30,    // 30 minute completeness cache TTL
            batch_size: 1000,                  // Number of indicators to batch insert
            retry_max: 3,                      // Maximum retries
            retry_delay_ms: 500,               // Delay between retries
        }
    }
}

// Add Clone implementation for Worker
#[derive(Clone)]
pub struct Worker {
    pg: Arc<PostgresManager>,
    redis: Arc<RedisManager>,
    completeness_cache: SharedCompletenessCache,
    completeness_controller: CompletenessController,
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
        // Create the completeness cache with the configured TTL
        let completeness_cache = Arc::new(CompletenessCache::new(config.completeness_cache_minutes));
        
        // Create the completeness controller
        let completeness_controller = CompletenessController::new(
            completeness_cache.clone(),
            pg.clone(),
        );
        
        Self {
            pg,
            redis,
            completeness_cache,
            completeness_controller,
            config,
            concurrency_limit,
        }
    }

    pub async fn start(self) -> Result<()> {
        info!("Starting indicator calculation worker with concurrency limit: {}", self.concurrency_limit);
        
        // Initialize completeness cache
        info!("Initializing completeness cache");
        if let Err(e) = self.completeness_controller.initialize_cache().await {
            error!("Failed to initialize completeness cache: {}", e);
        }
        
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
        let _ = log_to_file("Started job producer").await;
        
        // Track when we last initialized the completeness cache
        let mut last_cache_refresh = Instant::now();
        
        loop {
            // Periodically refresh the completeness cache
            if last_cache_refresh.elapsed() >= Duration::from_secs(
                (self.config.completeness_cache_minutes * 60) as u64
            ) {
                info!("Refreshing completeness cache");
                if let Err(e) = self.completeness_controller.initialize_cache().await {
                    error!("Failed to refresh completeness cache: {}", e);
                }
                last_cache_refresh = Instant::now();
            }
            
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
            let _ = log_to_file(&format!("Found {} enabled indicator configurations", configs.len())).await;
            
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
                
                // Check if job is already complete according to our cache
                if self.completeness_controller.is_job_complete(&job) {
                    debug!("Skipping complete job: {}:{}:{} with parameters: {:?}", 
                           job.symbol, job.interval, job.indicator_name, job.parameters);
                    continue;
                }
                
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
        let _ = log_to_file(&format!("Started worker {}", worker_id)).await;
        
        while let Some(job) = job_rx.recv().await {
            // Acquire permit from semaphore
            let _permit = semaphore.acquire().await?;
            
            info!("Worker {} processing job: {}:{}:{}", 
                  worker_id, job.symbol, job.interval, job.indicator_name);
            let _ = log_to_file(&format!("Worker {} processing job: {}:{}:{} with parameters: {:?}", 
                worker_id, job.symbol, job.interval, job.indicator_name, job.parameters)).await;
            
            // Process the job
            match self.process_job(&job).await {
                Ok(success) => {
                    if success {
                        // Update the completeness cache with new information
                        if let Ok((last_time, count)) = self.pg.get_indicator_completeness(
                            &job.symbol,
                            &job.interval,
                            &job.indicator_name,
                            &job.parameters,
                        ).await {
                            // Get candle data range
                            if let Ok((first_candle, last_candle)) = self.pg.get_candle_data_range(
                                &job.symbol,
                                &job.interval,
                            ).await {
                                // Create updated completeness info
                                let mut info = CompletenessInfo::from_job(&job);
                                info.last_calculated_time = last_time;
                                info.first_candle_time = Some(first_candle);
                                info.last_candle_time = Some(last_candle);
                                info.data_count = count;
                                
                                // Calculate coverage percentage
                                if let Some(last_calc) = last_time {
                                    let candle_span = last_candle.signed_duration_since(first_candle).num_seconds();
                                    if candle_span > 0 {
                                        let calc_span = last_calc.signed_duration_since(first_candle).num_seconds();
                                        let coverage = (calc_span as f64 / candle_span as f64) * 100.0;
                                        info.coverage_percent = coverage.min(100.0) as i32;
                                        
                                        // Determine if complete
                                        let freshness = last_candle.signed_duration_since(last_calc).num_hours();
                                        info.is_complete = freshness <= 24 && info.coverage_percent >= 95;
                                    }
                                }
                                
                                // Update cache
                                self.completeness_cache.update(info);
                            }
                        }
                    }
                },
                Err(e) => {
                    error!("Failed to process job: {}", e);
                    let _ = log_to_file(&format!("Failed to process job: {}", e)).await;
                    
                    // Release the job from cache so it can be retried
                    let job_key = job.cache_key();
                    if let Err(e) = self.redis.delete(&job_key).await {
                        warn!("Failed to remove failed job from cache: {}", e);
                    }
                }
            }
        }
        
        Ok(())
    }
    
    #[instrument(skip(self))]
    async fn process_job(&self, job: &CalculationJob) -> Result<bool> {
        // Get candle data
        let data = self.pg.get_candle_data(&job.symbol, &job.interval).await?;
        
        if data.close.is_empty() {
            warn!("No candle data available for {}:{}", job.symbol, job.interval);
            let _ = log_to_file(&format!("No candle data available for {}:{}", job.symbol, job.interval)).await;
            return Ok(false);
        }
        
        // Log data information
        let data_info = format!(
            "Found {} candle data points for {}:{} from {} to {}",
            data.close.len(),
            job.symbol, job.interval,
            data.open_time.first().unwrap().to_rfc3339(),
            data.open_time.last().unwrap().to_rfc3339()
        );
        debug!("{}", data_info);
        let _ = log_to_file(&data_info).await;
        
        // Log sample data (last 5 points)
        if data.close.len() >= 5 {
            let sample_idx = data.close.len() - 5;
            let sample_data = format!(
                "Sample data (last 5 points) for {}:{}: Open: {:?}, High: {:?}, Low: {:?}, Close: {:?}, Volume: {:?}",
                job.symbol, job.interval,
                &data.open[sample_idx..],
                &data.high[sample_idx..],
                &data.low[sample_idx..],
                &data.close[sample_idx..],
                &data.volume[sample_idx..]
            );
            debug!("{}", sample_data);
            let _ = log_to_file(&sample_data).await;
        }
        
        // Calculate the indicator using the TA-Lib abstract interface
        debug!("Calculating indicator {}:{}:{} using TA-Lib abstract interface", 
               job.symbol, job.interval, job.indicator_name);
        let results = self.calculate_indicator(job, &data).await?;
        let results_len = results.len(); // Store length before moving
        
        if results.is_empty() {
            info!("No new indicator values calculated for {}:{}:{}", 
                 job.symbol, job.interval, job.indicator_name);
            return Ok(false);
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
        let _ = log_to_file(&format!("Successfully processed indicator {}:{}:{} - Generated {} data points", 
             job.symbol, job.interval, job.indicator_name, results_len));
        
        Ok(true)
    }
    
    async fn calculate_indicator(
        &self,
        job: &CalculationJob,
        candle_data: &CandleData,
    ) -> Result<Vec<(DateTime<Utc>, Value)>> {
        // Get the TA-Lib function name for this indicator
        let ta_function_name = IndicatorCalculator::get_ta_function_name(&job.indicator_name);
        
        // Log function parameters
        let params_info = format!(
            "Calculating {} using TA-Lib function '{}' with parameters: {:?}",
            job.indicator_name, ta_function_name, job.parameters
        );
        debug!("{}", params_info);
        let _ = log_to_file(&params_info).await;
        
        // Special handling for multi-output indicators that need extra processing
        let result = match job.indicator_name.as_str() {
            "MACD" => {
                let fast_period = job.parameters["fast_period"].as_u64().unwrap_or(12) as usize;
                let slow_period = job.parameters["slow_period"].as_u64().unwrap_or(26) as usize;
                let signal_period = job.parameters["signal_period"].as_u64().unwrap_or(9) as usize;
                
                let _ = log_to_file(&format!(
                    "MACD configuration: fast_period={}, slow_period={}, signal_period={}",
                    fast_period, slow_period, signal_period
                )).await;
                
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
                
                let _ = log_to_file(&format!(
                    "BBANDS configuration: period={}, dev_up={}, dev_down={}",
                    period, dev_up, dev_down
                )).await;
                
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
                
                let _ = log_to_file(&format!(
                    "STOCH configuration: k_period={}, d_period={}, slowing={}",
                    k_period, d_period, slowing
                )).await;
                
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
        };
        
        // Log result
        match &result {
            Ok(values) => {
                let success_msg = format!(
                    "Successfully calculated {} - Got {} data points", 
                    job.indicator_name, values.len()
                );
                debug!("{}", success_msg);
                let _ = log_to_file(&success_msg).await;
                
                // Log sample of output values
                if !values.is_empty() && values.len() > 3 {
                    let sample_idx = values.len() - 3;
                    let sample_values = format!(
                        "Sample output values (last 3 points): {:?}", 
                        &values[sample_idx..]
                    );
                    debug!("{}", sample_values);
                    let _ = log_to_file(&sample_values).await;
                }
            },
            Err(e) => {
                let error_msg = format!("Failed to calculate {}: {}", job.indicator_name, e);
                error!("{}", error_msg);
                let _ = log_to_file(&error_msg).await;
            },
        }
        
        result
    }
}

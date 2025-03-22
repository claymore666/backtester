use crate::cache::redis::RedisManager;
use crate::database::models::{CalculatedIndicatorBatch, CandleData};
use crate::database::postgres::PostgresManager;
use crate::indicators::oscillators::OscillatorCalculator;
use crate::indicators::overlaps::OverlapCalculator;
use crate::indicators::patterns::PatternRecognizer;
use crate::indicators::volatility::VolatilityCalculator;
use crate::indicators::volume::VolumeCalculator;
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
        
        // Just process jobs in this main thread instead of trying to clone the receiver
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
                let last_time = match self.pg
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
        let candle_data_key = RedisManager::candle_data_key(&job.symbol, &job.interval);
        
        let candle_data = match self.redis.get::<CandleData>(&candle_data_key).await? {
            Some(data) => {
                debug!("Found candle data in cache for {}:{}", job.symbol, job.interval);
                data
            },
            None => {
                info!("Fetching candle data from database for {}:{}", job.symbol, job.interval);
                let data = self.pg.get_candle_data(&job.symbol, &job.interval).await?;
                
                // Cache the data for future use
                if let Err(e) = self.redis
                    .set(
                        &candle_data_key,
                        &data,
                        Some(Duration::from_secs(self.config.cache_ttl_seconds)),
                    )
                    .await
                {
                    warn!("Failed to cache candle data: {}", e);
                }
                
                data
            }
        };
        
        if candle_data.close.is_empty() {
            warn!("No candle data available for {}:{}", job.symbol, job.interval);
            return Ok(());
        }
        
        // Calculate the indicator
        let results = self.calculate_indicator(job, &candle_data).await?;
        
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
        match job.indicator_type {
            IndicatorType::Oscillator => self.calculate_oscillator(job, candle_data),
            IndicatorType::Overlap => self.calculate_overlap(job, candle_data),
            IndicatorType::Volume => self.calculate_volume(job, candle_data),
            IndicatorType::Volatility => self.calculate_volatility(job, candle_data),
            IndicatorType::Pattern => self.calculate_pattern(job, candle_data),
        }
    }
    
    fn calculate_oscillator(
        &self,
        job: &CalculationJob,
        candle_data: &CandleData,
    ) -> Result<Vec<(DateTime<Utc>, Value)>> {
        let params = &job.parameters;
        
        match job.indicator_name.as_str() {
            "RSI" => {
                let period = params["period"].as_u64().unwrap_or(14) as usize;
                
                let results = OscillatorCalculator::calculate_rsi(candle_data, period)?;
                
                // Convert f64 results to Value
                let value_results = results
                    .into_iter()
                    .map(|(time, value)| (time, json!(value)))
                    .collect();
                
                Ok(value_results)
            },
            "MACD" => {
                let fast_period = params["fast_period"].as_u64().unwrap_or(12) as usize;
                let slow_period = params["slow_period"].as_u64().unwrap_or(26) as usize;
                let signal_period = params["signal_period"].as_u64().unwrap_or(9) as usize;
                
                OscillatorCalculator::calculate_macd(
                    candle_data,
                    fast_period,
                    slow_period,
                    signal_period,
                )
            },
            "STOCH" => {
                let k_period = params["k_period"].as_u64().unwrap_or(14) as usize;
                let k_slowing = params["k_slowing"].as_u64().unwrap_or(3) as usize;
                let d_period = params["d_period"].as_u64().unwrap_or(3) as usize;
                
                OscillatorCalculator::calculate_stochastic(
                    candle_data,
                    k_period,
                    k_slowing,
                    d_period,
                )
            },
            "STOCHRSI" => {
                let period = params["period"].as_u64().unwrap_or(14) as usize;
                let k_period = params["k_period"].as_u64().unwrap_or(9) as usize;
                let d_period = params["d_period"].as_u64().unwrap_or(3) as usize;
                
                OscillatorCalculator::calculate_stoch_rsi(
                    candle_data,
                    period,
                    k_period,
                    d_period,
                )
            },
            "CCI" => {
                let period = params["period"].as_u64().unwrap_or(20) as usize;
                
                let results = OscillatorCalculator::calculate_cci(candle_data, period)?;
                
                // Convert f64 results to Value
                let value_results = results
                    .into_iter()
                    .map(|(time, value)| (time, json!(value)))
                    .collect();
                
                Ok(value_results)
            },
            "MFI" => {
                let period = params["period"].as_u64().unwrap_or(14) as usize;
                
                let results = OscillatorCalculator::calculate_mfi(candle_data, period)?;
                
                // Convert f64 results to Value
                let value_results = results
                    .into_iter()
                    .map(|(time, value)| (time, json!(value)))
                    .collect();
                
                Ok(value_results)
            },
            "ULTOSC" => {
                let short_period = params["short_period"].as_u64().unwrap_or(7) as usize;
                let medium_period = params["medium_period"].as_u64().unwrap_or(14) as usize;
                let long_period = params["long_period"].as_u64().unwrap_or(28) as usize;
                
                let results = OscillatorCalculator::calculate_ultimate_oscillator(
                    candle_data,
                    short_period,
                    medium_period,
                    long_period,
                )?;
                
                // Convert f64 results to Value
                let value_results = results
                    .into_iter()
                    .map(|(time, value)| (time, json!(value)))
                    .collect();
                
                Ok(value_results)
            },
            "WILLR" => {
                let period = params["period"].as_u64().unwrap_or(14) as usize;
                
                let results = OscillatorCalculator::calculate_williams_r(candle_data, period)?;
                
                // Convert f64 results to Value
                let value_results = results
                    .into_iter()
                    .map(|(time, value)| (time, json!(value)))
                    .collect();
                
                Ok(value_results)
            },
            "MOM" => {
                let period = params["period"].as_u64().unwrap_or(10) as usize;
                
                let results = OscillatorCalculator::calculate_momentum(candle_data, period)?;
                
                // Convert f64 results to Value
                let value_results = results
                    .into_iter()
                    .map(|(time, value)| (time, json!(value)))
                    .collect();
                
                Ok(value_results)
            },
            "ROC" => {
                let period = params["period"].as_u64().unwrap_or(10) as usize;
                
                let results = OscillatorCalculator::calculate_roc(candle_data, period)?;
                
                // Convert f64 results to Value
                let value_results = results
                    .into_iter()
                    .map(|(time, value)| (time, json!(value)))
                    .collect();
                
                Ok(value_results)
            },
            "PPO" => {
                let fast_period = params["fast_period"].as_u64().unwrap_or(12) as usize;
                let slow_period = params["slow_period"].as_u64().unwrap_or(26) as usize;
                let signal_period = params["signal_period"].as_u64().unwrap_or(9) as usize;
                
                OscillatorCalculator::calculate_ppo(
                    candle_data,
                    fast_period,
                    slow_period,
                    signal_period,
                )
            },
            _ => {
                Err(anyhow::anyhow!("Unsupported oscillator indicator: {}", job.indicator_name))
            }
        }
    }
    
    fn calculate_overlap(
        &self,
        job: &CalculationJob,
        candle_data: &CandleData,
    ) -> Result<Vec<(DateTime<Utc>, Value)>> {
        let params = &job.parameters;
        
        match job.indicator_name.as_str() {
            "SMA" => {
                let period = params["period"].as_u64().unwrap_or(20) as usize;
                
                let results = OverlapCalculator::calculate_sma(candle_data, period)?;
                
                // Convert f64 results to Value
                let value_results = results
                    .into_iter()
                    .map(|(time, value)| (time, json!(value)))
                    .collect();
                
                Ok(value_results)
            },
            "EMA" => {
                let period = params["period"].as_u64().unwrap_or(20) as usize;
                
                let results = OverlapCalculator::calculate_ema(candle_data, period)?;
                
                // Convert f64 results to Value
                let value_results = results
                    .into_iter()
                    .map(|(time, value)| (time, json!(value)))
                    .collect();
                
                Ok(value_results)
            },
            "WMA" => {
                let period = params["period"].as_u64().unwrap_or(20) as usize;
                
                let results = OverlapCalculator::calculate_wma(candle_data, period)?;
                
                // Convert f64 results to Value
                let value_results = results
                    .into_iter()
                    .map(|(time, value)| (time, json!(value)))
                    .collect();
                
                Ok(value_results)
            },
            "DEMA" => {
                let period = params["period"].as_u64().unwrap_or(20) as usize;
                
                let results = OverlapCalculator::calculate_dema(candle_data, period)?;
                
                // Convert f64 results to Value
                let value_results = results
                    .into_iter()
                    .map(|(time, value)| (time, json!(value)))
                    .collect();
                
                Ok(value_results)
            },
            "TEMA" => {
                let period = params["period"].as_u64().unwrap_or(20) as usize;
                
                let results = OverlapCalculator::calculate_tema(candle_data, period)?;
                
                // Convert f64 results to Value
                let value_results = results
                    .into_iter()
                    .map(|(time, value)| (time, json!(value)))
                    .collect();
                
                Ok(value_results)
            },
            "TRIMA" => {
                let period = params["period"].as_u64().unwrap_or(20) as usize;
                
                let results = OverlapCalculator::calculate_trima(candle_data, period)?;
                
                // Convert f64 results to Value
                let value_results = results
                    .into_iter()
                    .map(|(time, value)| (time, json!(value)))
                    .collect();
                
                Ok(value_results)
            },
            "KAMA" => {
                let period = params["period"].as_u64().unwrap_or(20) as usize;
                let fast_ema = params["fast_ema"].as_u64().unwrap_or(2) as usize;
                let slow_ema = params["slow_ema"].as_u64().unwrap_or(30) as usize;
                
                let results = OverlapCalculator::calculate_kama(
                    candle_data,
                    period,
                    fast_ema,
                    slow_ema,
                )?;
                
                // Convert f64 results to Value
                let value_results = results
                    .into_iter()
                    .map(|(time, value)| (time, json!(value)))
                    .collect();
                
                Ok(value_results)
            },
            "BBANDS" => {
                let period = params["period"].as_u64().unwrap_or(20) as usize;
                let deviation = params["deviation"].as_f64().unwrap_or(2.0);
                
                OverlapCalculator::calculate_bollinger_bands(candle_data, period, deviation)
            },
            "SAR" => {
                let acceleration = params["acceleration"].as_f64().unwrap_or(0.02);
                let maximum = params["maximum"].as_f64().unwrap_or(0.2);
                
                let results = OverlapCalculator::calculate_parabolic_sar(
                    candle_data,
                    acceleration,
                    maximum,
                )?;
                
                // Convert f64 results to Value
                let value_results = results
                    .into_iter()
                    .map(|(time, value)| (time, json!(value)))
                    .collect();
                
                Ok(value_results)
            },
            _ => {
                Err(anyhow::anyhow!("Unsupported overlap indicator: {}", job.indicator_name))
            }
        }
    }
    
    fn calculate_volume(
        &self,
        job: &CalculationJob,
        candle_data: &CandleData,
    ) -> Result<Vec<(DateTime<Utc>, Value)>> {
        let params = &job.parameters;
        
        match job.indicator_name.as_str() {
            "OBV" => {
                let results = VolumeCalculator::calculate_obv(candle_data)?;
                
                // Convert f64 results to Value
      		let value_results = results
                    .into_iter()
                    .map(|(time, value)| (time, json!(value)))
                    .collect();
                
                Ok(value_results)
            },
            "AD" => {
                let results = VolumeCalculator::calculate_ad_line(candle_data)?;
                
                // Convert f64 results to Value
                let value_results = results
                    .into_iter()
                    .map(|(time, value)| (time, json!(value)))
                    .collect();
                
                Ok(value_results)
            },
            "ADOSC" => {
                let fast_period = params["fast_period"].as_u64().unwrap_or(3) as usize;
                let slow_period = params["slow_period"].as_u64().unwrap_or(10) as usize;
                
                let results = VolumeCalculator::calculate_chaikin_oscillator(
                    candle_data,
                    fast_period,
                    slow_period,
                )?;
                
                // Convert f64 results to Value
                let value_results = results
                    .into_iter()
                    .map(|(time, value)| (time, json!(value)))
                    .collect();
                
                Ok(value_results)
            },
            "VROC" => {
                let period = params["period"].as_u64().unwrap_or(25) as usize;
                
                let results = VolumeCalculator::calculate_volume_roc(candle_data, period)?;
                
                // Convert f64 results to Value
                let value_results = results
                    .into_iter()
                    .map(|(time, value)| (time, json!(value)))
                    .collect();
                
                Ok(value_results)
            },
            "PVT" => {
                let results = VolumeCalculator::calculate_price_volume_trend(candle_data)?;
                
                // Convert f64 results to Value
                let value_results = results
                    .into_iter()
                    .map(|(time, value)| (time, json!(value)))
                    .collect();
                
                Ok(value_results)
            },
            _ => {
                Err(anyhow::anyhow!("Unsupported volume indicator: {}", job.indicator_name))
            }
        }
    }
    
    fn calculate_volatility(
        &self,
        job: &CalculationJob,
        candle_data: &CandleData,
    ) -> Result<Vec<(DateTime<Utc>, Value)>> {
        let params = &job.parameters;
        
        match job.indicator_name.as_str() {
            "ATR" => {
                let period = params["period"].as_u64().unwrap_or(14) as usize;
                
                let results = VolatilityCalculator::calculate_atr(candle_data, period)?;
                
                // Convert f64 results to Value
                let value_results = results
                    .into_iter()
                    .map(|(time, value)| (time, json!(value)))
                    .collect();
                
                Ok(value_results)
            },
            "NATR" => {
                let period = params["period"].as_u64().unwrap_or(14) as usize;
                
                let results = VolatilityCalculator::calculate_natr(candle_data, period)?;
                
                // Convert f64 results to Value
                let value_results = results
                    .into_iter()
                    .map(|(time, value)| (time, json!(value)))
                    .collect();
                
                Ok(value_results)
            },
            "TRANGE" => {
                let results = VolatilityCalculator::calculate_true_range(candle_data)?;
                
                // Convert f64 results to Value
                let value_results = results
                    .into_iter()
                    .map(|(time, value)| (time, json!(value)))
                    .collect();
                
                Ok(value_results)
            },
            "STDDEV" => {
                let period = params["period"].as_u64().unwrap_or(5) as usize;
                
                let results = VolatilityCalculator::calculate_standard_deviation(candle_data, period)?;
                
                // Convert f64 results to Value
                let value_results = results
                    .into_iter()
                    .map(|(time, value)| (time, json!(value)))
                    .collect();
                
                Ok(value_results)
            },
            _ => {
                Err(anyhow::anyhow!("Unsupported volatility indicator: {}", job.indicator_name))
            }
        }
    }
    
    fn calculate_pattern(
        &self,
        job: &CalculationJob,
        candle_data: &CandleData,
    ) -> Result<Vec<(DateTime<Utc>, Value)>> {
        let params = &job.parameters;
        
        // For candlestick patterns, we use a common penetration parameter
        let penetration = params["penetration"].as_f64().unwrap_or(0.5);
        
        // Calculate all patterns at once
        PatternRecognizer::calculate_all_patterns(candle_data, penetration)
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

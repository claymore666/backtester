use crate::cache::completeness::{CompletenessInfo, SharedCompletenessCache};
use crate::database::postgres::PostgresManager;
use crate::processor::job::{CalculationJob, IndicatorType};
use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{info, warn};

/// Controller for managing the completeness cache
#[derive(Clone)]
pub struct CompletenessController {
    cache: SharedCompletenessCache,
    pg: Arc<PostgresManager>,
}

impl CompletenessController {
    /// Create a new completeness controller
    pub fn new(cache: SharedCompletenessCache, pg: Arc<PostgresManager>) -> Self {
        Self { cache, pg }
    }
    
    /// Initialize the completeness cache by loading all enabled configurations
    pub async fn initialize_cache(&self) -> Result<()> {
        info!("Initializing completeness cache with all enabled configurations");
        
        // Get all enabled indicator configurations
        let configs = self.pg.get_enabled_indicator_configs().await?;
        info!("Found {} enabled indicator configurations", configs.len());
        
        // First, get all symbol/interval pairs to fetch candle data ranges
        let mut symbol_intervals = Vec::new();
        for config in &configs {
            let pair = (config.symbol.clone(), config.interval.clone());
            if !symbol_intervals.contains(&pair) {
                symbol_intervals.push(pair);
            }
        }
        
        // Fetch candle data ranges for all symbol/interval pairs
        let mut candle_ranges = HashMap::new();
        for (symbol, interval) in &symbol_intervals {
            match self.pg.get_candle_data_range(symbol, interval).await {
                Ok((first, last)) => {
                    candle_ranges.insert((symbol.clone(), interval.clone()), (first, last));
                }
                Err(e) => {
                    warn!("Failed to get candle data range for {}:{}: {}", symbol, interval, e);
                }
            }
        }
        
        // Process each configuration
        let mut cache_updates = Vec::new();
        
        for config in configs {
            let indicator_type = IndicatorType::from(config.indicator_type.as_str());
            
            let job = CalculationJob::new(
                config.symbol.clone(),
                config.interval.clone(),
                indicator_type,
                config.indicator_name.clone(),
                config.parameters.clone(),
            );
            
            // Get candle data range
            let candle_range = candle_ranges.get(&(job.symbol.clone(), job.interval.clone()));
            
            // Create initial completeness info
            let mut info = CompletenessInfo::from_job(&job);
            
            if let Some((first, last)) = candle_range {
                info.first_candle_time = Some(*first);
                info.last_candle_time = Some(*last);
            }
            
            // Get last calculated time and data count
            match self.pg.get_indicator_completeness(
                &job.symbol,
                &job.interval,
                &job.indicator_name,
                &job.parameters,
            ).await {
                Ok((last_time, count)) => {
                    info.last_calculated_time = last_time;
                    info.data_count = count;
                    
                    // Calculate coverage percentage
                    if let (Some(first_candle), Some(last_candle), Some(last_calc)) = 
                        (info.first_candle_time, info.last_candle_time, last_time) {
                        
                        let candle_span = last_candle.signed_duration_since(first_candle).num_seconds();
                        if candle_span > 0 {
                            let calc_span = last_calc.signed_duration_since(first_candle).num_seconds();
                            let coverage = (calc_span as f64 / candle_span as f64) * 100.0;
                            info.coverage_percent = coverage.min(100.0) as i32;
                            
                            // Determine if complete (last calculation within 24 hours of last candle)
                            let freshness = last_candle.signed_duration_since(last_calc).num_hours();
                            info.is_complete = freshness <= 24 && info.coverage_percent >= 95;
                        }
                    }
                }
                Err(e) => {
                    warn!("Failed to get indicator completeness for {}:{}:{}: {}", 
                          job.symbol, job.interval, job.indicator_name, e);
                }
            }
            
            // Add to batch updates
            cache_updates.push(info);
        }
        
        // Update cache with all completeness information
        for info in cache_updates {
            self.cache.update(info);
        }
        
        // Log cache statistics
        self.cache.log_stats();
        
        Ok(())
    }
    
    /// Check if a job is complete based on cached information
    pub fn is_job_complete(&self, job: &CalculationJob) -> bool {
        if let Some(info) = self.cache.get(job) {
            return info.is_complete;
        }
        
        // If not in cache, assume not complete
        false
    }
}

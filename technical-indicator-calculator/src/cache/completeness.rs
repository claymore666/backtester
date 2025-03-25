use crate::processor::job::CalculationJob;
use chrono::{DateTime, Duration, Utc};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::info;

/// Represents the completeness status of an indicator
#[derive(Debug, Clone)]
pub struct CompletenessInfo {
    /// Symbol for the indicator (e.g., "BTCUSDT")
    pub symbol: String,
    /// Interval for the indicator (e.g., "1h")
    pub interval: String,
    /// Name of the indicator (e.g., "RSI")
    pub indicator_name: String,
    /// Parameters used for the indicator
    pub parameters: serde_json::Value,
    /// Last calculated time for this indicator
    pub last_calculated_time: Option<DateTime<Utc>>,
    /// First candle time available for this symbol/interval
    pub first_candle_time: Option<DateTime<Utc>>,
    /// Last candle time available for this symbol/interval
    pub last_candle_time: Option<DateTime<Utc>>,
    /// Number of calculated data points
    pub data_count: i64,
    /// Coverage percentage (0-100)
    pub coverage_percent: i32,
    /// Whether the indicator is complete (up to date with available data)
    pub is_complete: bool,
    /// When this status was last updated
    pub updated_at: DateTime<Utc>,
}

impl CompletenessInfo {
    /// Create a key for this completeness info
    pub fn cache_key(&self) -> String {
        format!(
            "{}:{}:{}:{}",
            self.symbol,
            self.interval,
            self.indicator_name,
            self.parameters
        )
    }
    
    /// Create a new completeness info from a job
    pub fn from_job(job: &CalculationJob) -> Self {
        Self {
            symbol: job.symbol.clone(),
            interval: job.interval.clone(),
            indicator_name: job.indicator_name.clone(),
            parameters: job.parameters.clone(),
            last_calculated_time: None,
            first_candle_time: None,
            last_candle_time: None,
            data_count: 0,
            coverage_percent: 0,
            is_complete: false,
            updated_at: Utc::now(),
        }
    }
    
    /// Check if the completeness info is still valid
    pub fn is_valid(&self, ttl_minutes: i64) -> bool {
        let now = Utc::now();
        let age = now.signed_duration_since(self.updated_at);
        age < Duration::minutes(ttl_minutes)
    }
}

/// Cache for completeness information
pub struct CompletenessCache {
    cache: RwLock<HashMap<String, CompletenessInfo>>,
    ttl_minutes: i64,
}

impl CompletenessCache {
    /// Create a new completeness cache
    pub fn new(ttl_minutes: i64) -> Self {
        Self {
            cache: RwLock::new(HashMap::new()),
            ttl_minutes,
        }
    }
    
    /// Get completeness info for a job
    pub fn get(&self, job: &CalculationJob) -> Option<CompletenessInfo> {
        let key = format!(
            "{}:{}:{}:{}",
            job.symbol,
            job.interval,
            job.indicator_name,
            job.parameters
        );
        
        let cache = self.cache.read();
        let info = cache.get(&key).cloned();
        
        // Return info only if it's still valid
        info.filter(|i| i.is_valid(self.ttl_minutes))
    }
    
    /// Update completeness info in the cache
    pub fn update(&self, info: CompletenessInfo) {
        let key = info.cache_key();
        let mut cache = self.cache.write();
        cache.insert(key, info);
    }
    
    /// Remove a job from the cache
    pub fn remove(&self, job: &CalculationJob) {
        let key = format!(
            "{}:{}:{}:{}",
            job.symbol,
            job.interval,
            job.indicator_name,
            job.parameters
        );
        
        let mut cache = self.cache.write();
        cache.remove(&key);
    }
    
    /// Clear the entire cache
    pub fn clear(&self) {
        let mut cache = self.cache.write();
        cache.clear();
    }
    
    /// Get all incomplete jobs from the cache
    pub fn get_incomplete_jobs(&self) -> Vec<CalculationJob> {
        let cache = self.cache.read();
        let mut jobs = Vec::new();
        
        for (_, info) in cache.iter() {
            // Skip if complete or not valid
            if info.is_complete || !info.is_valid(self.ttl_minutes) {
                continue;
            }
            
            // Create a job from the completeness info
            let job = CalculationJob::new(
                info.symbol.clone(),
                info.interval.clone(),
                info.indicator_name.clone().as_str().into(),
                info.indicator_name.clone(),
                info.parameters.clone(),
            );
            
            jobs.push(job);
        }
        
        jobs
    }
    
    /// Get cache statistics
    pub fn get_stats(&self) -> (usize, usize, usize) {
        let cache = self.cache.read();
        let total = cache.len();
        let complete = cache.values().filter(|i| i.is_complete).count();
        let incomplete = total - complete;
        
        (total, complete, incomplete)
    }
    
    /// Log cache statistics
    pub fn log_stats(&self) {
        let (total, complete, incomplete) = self.get_stats();
        info!(
            "Completeness cache stats: total={}, complete={}, incomplete={}",
            total, complete, incomplete
        );
    }
}

impl Default for CompletenessCache {
    fn default() -> Self {
        // Default TTL: 30 minutes
        Self::new(30)
    }
}

// Shared, thread-safe cache that can be passed around
pub type SharedCompletenessCache = Arc<CompletenessCache>;

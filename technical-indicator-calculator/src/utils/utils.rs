use anyhow::Result;
use chrono::{DateTime, Utc};
use std::time::{Instant};
use tracing::{debug};

// Utility function to measure execution time of operations
#[allow(dead_code)]
pub async fn measure_time<F, T>(operation_name: &str, f: F) -> Result<T>
where
    F: std::future::Future<Output = Result<T>>,
{
    let start = Instant::now();
    let result = f.await;
    let elapsed = start.elapsed();
    
    debug!(
        "{} completed in {:.2?}",
        operation_name,
        elapsed
    );
    
    result
}

// Format a timestamp for logging
#[allow(dead_code)]
pub fn format_time(time: &DateTime<Utc>) -> String {
    time.format("%Y-%m-%d %H:%M:%S UTC").to_string()
}

// Get current timestamp as string
#[allow(dead_code)]
pub fn now_string() -> String {
    format_time(&Utc::now())
}

// Function to convert a timestamp to a readable format
#[allow(dead_code)]
pub fn format_timestamp(timestamp_ms: i64) -> String {
    let dt = DateTime::<Utc>::from_timestamp(timestamp_ms / 1000, 0)
        .unwrap_or_else(|| Utc::now());
    format_time(&dt)
}

// Utility to truncate long strings for logging
#[allow(dead_code)]
pub fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[0..max_len])
    }
}

// Helper to handle type annotations for Arc in main.rs
#[allow(dead_code)]
pub fn specify_type<T>(value: T) -> T {
    value
}

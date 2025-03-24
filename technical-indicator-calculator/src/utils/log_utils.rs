// src/utils/log_utils.rs
use anyhow::Result;
use chrono::Local;
use std::fs::{create_dir_all, OpenOptions};
use std::io::Write;
use std::path::Path;

pub fn log_to_file(message: &str) -> Result<()> {
    let log_dir = Path::new("logs");
    if !log_dir.exists() {
        create_dir_all(log_dir)?;
    }
    
    let log_file = log_dir.join("indicator_calculations.log");
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_file)?;
        
    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
    writeln!(file, "[{}] {}", timestamp, message)?;
    
    Ok(())
}

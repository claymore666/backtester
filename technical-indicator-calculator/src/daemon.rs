// src/daemon.rs
use anyhow::{Result, Context};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use tracing::{info, error, warn};
use std::os::unix::fs::PermissionsExt;

const PID_FILE: &str = "/tmp/indicator-calculator.pid";
const LOG_FILE: &str = "/tmp/indicator-calculator.log";

/// Start the indicator calculator as a daemon process
pub async fn start_daemon(concurrency: Option<usize>) -> Result<()> {
    // Check if daemon is already running
    if is_daemon_running()? {
        println!("Indicator calculator is already running.");
        return Ok(());
    }
    
    // Build the command to start the daemon
    let mut cmd = Command::new(std::env::current_exe()?);
    
    // Add arguments
    cmd.arg("start");
    
    if let Some(conc) = concurrency {
        cmd.arg("--concurrency").arg(conc.to_string());
    }
    
    // Redirect stdout and stderr to log file
    let log_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(LOG_FILE)?;
    
    // Set executable permissions
    let metadata = fs::metadata(LOG_FILE)?;
    let mut permissions = metadata.permissions();
    permissions.set_mode(0o644);  // User read/write, group read, others read
    fs::set_permissions(LOG_FILE, permissions)?;
    
    cmd.stdout(Stdio::from(log_file.try_clone()?))
       .stderr(Stdio::from(log_file))
       .stdin(Stdio::null());
    
    // Spawn the daemon process
    let child = cmd.spawn()
        .context("Failed to spawn daemon process")?;
    
    // Write PID to file
    let pid = child.id();
    fs::write(PID_FILE, pid.to_string())?;
    
    // Set permissions on the PID file
    let metadata = fs::metadata(PID_FILE)?;
    let mut permissions = metadata.permissions();
    permissions.set_mode(0o644);  // User read/write, group read, others read
    fs::set_permissions(PID_FILE, permissions)?;
    
    println!("Indicator calculator daemon started with PID {}.", pid);
    println!("Logs are being written to {}", LOG_FILE);
    
    Ok(())
}

/// Stop the indicator calculator daemon
pub async fn stop_daemon() -> Result<()> {
    // Check if PID file exists
    if !Path::new(PID_FILE).exists() {
        println!("Indicator calculator is not running.");
        return Ok(());
    }
    
    // Read PID from file
    let mut file = File::open(PID_FILE)?;
    let mut pid_str = String::new();
    file.read_to_string(&mut pid_str)?;
    
    let pid = pid_str.trim().parse::<u32>()
        .context("Invalid PID in PID file")?;
    
    // Check if process is running
    let status = Command::new("ps")
        .arg("-p")
        .arg(pid.to_string())
        .arg("-o")
        .arg("comm=")
        .output()?;
    
    if !status.status.success() {
        warn!("Process with PID {} is not running, removing stale PID file", pid);
        fs::remove_file(PID_FILE)?;
        println!("Removed stale PID file.");
        return Ok(());
    }
    
    // Send SIGTERM to the process
    let kill_status = Command::new("kill")
        .arg(pid.to_string())
        .status()?;
    
    if kill_status.success() {
        println!("Sent termination signal to indicator calculator daemon (PID {}).", pid);
        
        // Wait a bit to ensure the process has time to shut down
        std::thread::sleep(std::time::Duration::from_secs(1));
        
        // Remove PID file
        if Path::new(PID_FILE).exists() {
            fs::remove_file(PID_FILE)?;
        }
    } else {
        error!("Failed to terminate process with PID {}", pid);
        
        // Try a more forceful approach with SIGKILL
        println!("Trying force kill...");
        let force_kill_status = Command::new("kill")
            .arg("-9")
            .arg(pid.to_string())
            .status()?;
            
        if force_kill_status.success() {
            println!("Force killed indicator calculator daemon (PID {}).", pid);
            
            // Remove PID file
            if Path::new(PID_FILE).exists() {
                fs::remove_file(PID_FILE)?;
            }
        } else {
            return Err(anyhow::anyhow!("Failed to terminate process with PID {}", pid));
        }
    }
    
    Ok(())
}

/// Check if the daemon is running
fn is_daemon_running() -> Result<bool> {
    // Check if PID file exists
    if !Path::new(PID_FILE).exists() {
        return Ok(false);
    }
    
    // Read PID from file
    let mut file = File::open(PID_FILE)?;
    let mut pid_str = String::new();
    file.read_to_string(&mut pid_str)?;
    
    let pid = pid_str.trim().parse::<u32>()
        .context("Invalid PID in PID file")?;
    
    // Check if process is running
    let status = Command::new("ps")
        .arg("-p")
        .arg(pid.to_string())
        .arg("-o")
        .arg("comm=")
        .output()?;
    
    Ok(status.status.success())
}

/// Get the last few lines of the log file
fn get_recent_logs(lines: usize) -> Result<String> {
    if !Path::new(LOG_FILE).exists() {
        return Ok("No log file found.".to_string());
    }
    
    let output = Command::new("tail")
        .arg("-n")
        .arg(lines.to_string())
        .arg(LOG_FILE)
        .output()?;
    
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Check the status of the daemon
pub async fn check_daemon_status() -> Result<()> {
    // Check if daemon is running
    let running = is_daemon_running()?;
    
    if running {
        // Read PID from file
        let pid_str = fs::read_to_string(PID_FILE)?;
        let pid = pid_str.trim();
        
        println!("Indicator calculator daemon is running (PID {}).", pid);
        
        // Show uptime information
        let uptime = Command::new("ps")
            .arg("-p")
            .arg(pid)
            .arg("-o")
            .arg("etime=")
            .output()?;
        
        let uptime_str = String::from_utf8_lossy(&uptime.stdout).trim().to_string();
        println!("Uptime: {}", uptime_str);
        
        // Show recent logs
        println!("\nRecent logs:");
        println!("{}", get_recent_logs(10)?);
    } else {
        println!("Indicator calculator daemon is not running.");
    }
    
    Ok(())
}

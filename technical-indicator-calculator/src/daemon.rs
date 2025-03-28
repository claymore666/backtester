// src/daemon.rs
use anyhow::{Result, Context};
use std::fs::{self, File};
use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};
use tracing::{error, warn};

const PID_FILE: &str = "/tmp/indicator-calculator.pid";
const LOG_FILE: &str = "/tmp/indicator-calculator.log";

/// Start the indicator calculator as a daemon process
pub async fn start_daemon(concurrency: Option<usize>) -> Result<()> {
    // Check if daemon is already running
    if is_daemon_running()? {
        println!("Indicator calculator is already running.");
        return Ok(());
    }
    
    // On Unix systems, use the nohup command which is specifically designed for this purpose
    #[cfg(unix)]
    {
        // Build the command using nohup
        let mut args = vec!["start"];
        
        // Create a string holder for concurrency value if provided
        let conc_string;
        
        if let Some(conc) = concurrency {
            conc_string = conc.to_string();
            args.push("--concurrency");
            args.push(&conc_string);
        }
        
        // Prepare the nohup command
        let mut cmd = Command::new("nohup");
        let exec_path = std::env::current_exe()?;
        
        cmd.arg(exec_path)
            .args(args)
            .stdout(Stdio::from(File::create("/tmp/indicator-calculator.log")?))
            .stderr(Stdio::from(File::create("/tmp/indicator-calculator.err")?))
            .stdin(Stdio::null())
            .env("RUST_LOG", "info")  // Ensure logging is set
            .spawn()?;
            
        // Wait a moment for the process to start
        std::thread::sleep(std::time::Duration::from_millis(500));
        
        // Find the PID by looking for the process
        let output = Command::new("pgrep")
            .arg("-f")
            .arg("technical-indicator-calculator start")
            .output()?;
            
        if output.status.success() {
            let pid_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
            // We might have multiple matches, take the last one which is likely our daemon
            let pid = pid_str.lines().last().unwrap_or("").trim();
            
            if !pid.is_empty() {
                // Write the PID to the file
                fs::write(PID_FILE, pid)?;
                
                println!("Indicator calculator daemon started with PID {}.", pid);
                println!("Logs are being written to {}", LOG_FILE);
                return Ok(());
            }
        }
        
        println!("Daemon started, but could not determine PID. Check process list manually.");
    }
    
    // For non-Unix systems, we use a simpler approach
    #[cfg(not(unix))]
    {
        warn!("Running on a non-Unix system. Daemon functionality may be limited.");
        
        // Build the command
        let mut cmd = Command::new(std::env::current_exe()?);
        cmd.arg("start");
        
        if let Some(conc) = concurrency {
            cmd.arg("--concurrency").arg(conc.to_string());
        }
        
        // Redirect stdio
        let log_file = File::create(LOG_FILE)?;
        
        cmd.stdout(Stdio::from(log_file.try_clone()?))
           .stderr(Stdio::from(log_file))
           .stdin(Stdio::null())
           .spawn()?;
           
        println!("Indicator calculator started in background mode.");
        println!("Note: On Windows, the process may terminate when you log out.");
    }
    
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
    #[cfg(unix)]
    {
        let status = Command::new("ps")
            .arg("-p")
            .arg(pid.to_string())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()?;
        
        if !status.success() {
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
            std::thread::sleep(std::time::Duration::from_secs(2));
            
            // Check if process is still running
            let check_status = Command::new("ps")
                .arg("-p")
                .arg(pid.to_string())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()?;
                
            if check_status.success() {
                // Process is still running, try SIGKILL
                println!("Process still running, attempting force kill...");
                let force_kill_status = Command::new("kill")
                    .arg("-9")
                    .arg(pid.to_string())
                    .status()?;
                    
                if force_kill_status.success() {
                    println!("Force killed indicator calculator daemon (PID {}).", pid);
                } else {
                    return Err(anyhow::anyhow!("Failed to terminate process with PID {}", pid));
                }
            }
            
            // Remove PID file
            if Path::new(PID_FILE).exists() {
                fs::remove_file(PID_FILE)?;
            }
            
            println!("Indicator calculator daemon stopped.");
        } else {
            error!("Failed to terminate process with PID {}", pid);
            return Err(anyhow::anyhow!("Failed to terminate process with PID {}", pid));
        }
    }
    
    #[cfg(not(unix))]
    {
        // On Windows, we use taskkill
        let status = Command::new("taskkill")
            .arg("/PID")
            .arg(pid.to_string())
            .arg("/F")  // Force termination
            .status()?;
            
        if status.success() {
            println!("Successfully terminated indicator calculator (PID {}).", pid);
            
            // Remove PID file
            if Path::new(PID_FILE).exists() {
                fs::remove_file(PID_FILE)?;
            }
        } else {
            println!("Failed to terminate process. The process may have already exited.");
            // Remove stale PID file
            if Path::new(PID_FILE).exists() {
                fs::remove_file(PID_FILE)?;
            }
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
    
    // Check if process is running (platform-specific)
    #[cfg(unix)]
    {
        let status = Command::new("ps")
            .arg("-p")
            .arg(pid.to_string())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()?;
        
        return Ok(status.success());
    }
    
    #[cfg(not(unix))]
    {
        // On Windows, use tasklist to check if process exists
        let output = Command::new("tasklist")
            .arg("/FI")
            .arg(format!("PID eq {}", pid))
            .arg("/NH")
            .output()?;
            
        let output_str = String::from_utf8_lossy(&output.stdout);
        return Ok(output_str.contains(&format!("{}", pid)));
    }
}

/// Get the last few lines of the log file
fn get_recent_logs(lines: usize) -> Result<String> {
    if !Path::new(LOG_FILE).exists() {
        return Ok("No log file found. The daemon may have just started or no logs have been written yet.".to_string());
    }
    
    let output = Command::new("tail")
        .arg("-n")
        .arg(lines.to_string())
        .arg(LOG_FILE)
        .output()?;
    
    if output.stdout.is_empty() {
        return Ok("Log file exists but is empty.".to_string());
    }
    
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
        #[cfg(unix)]
        {
            let uptime = Command::new("ps")
                .arg("-p")
                .arg(pid)
                .arg("-o")
                .arg("etime=")
                .output()?;
            
            let uptime_str = String::from_utf8_lossy(&uptime.stdout).trim().to_string();
            if !uptime_str.is_empty() {
                println!("Uptime: {}", uptime_str);
            }
        }
        
        // Show process details
        #[cfg(unix)]
        {
            let proc_info = Command::new("ps")
                .arg("-p")
                .arg(pid)
                .arg("-o")
                .arg("pid,ppid,%cpu,%mem,command")
                .output()?;
                
            let info_str = String::from_utf8_lossy(&proc_info.stdout).to_string();
            println!("\nProcess information:");
            println!("{}", info_str);
        }
        
        #[cfg(not(unix))]
        {
            println!("Process is running. Status details not available on this platform.");
        }
        
        // Show recent logs
        println!("\nRecent logs:");
        println!("{}", get_recent_logs(10)?);
    } else {
        println!("Indicator calculator daemon is not running.");
    }
    
    Ok(())
}

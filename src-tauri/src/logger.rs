use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::sync::Mutex;

/// Log entry with timestamp, level, and message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: String,
    pub level: String,
    pub message: String,
    pub module: Option<String>,
    pub source: String,
}

/// Global log storage
static LOGS: Mutex<Option<Vec<LogEntry>>> = Mutex::new(None);

/// Initialize the logging system
pub fn init_logger() {
    // Initialize the log storage
    if let Ok(mut logs) = LOGS.lock() {
        *logs = Some(Vec::new());
    }

    // Log to internal storage only (no terminal output)
    log_entry("INFO", "Logging system initialized", None);
}

/// Get all log entries
pub fn get_logs() -> Vec<LogEntry> {
    if let Ok(logs) = LOGS.lock() {
        logs.clone().unwrap_or_default()
    } else {
        Vec::new()
    }
}

/// Clear all log entries
pub fn clear_logs() {
    if let Ok(mut logs) = LOGS.lock() {
        if let Some(log_vec) = logs.as_mut() {
            log_vec.clear();
        }
    }
}

/// Add a log entry
pub fn log_entry(level: &str, message: &str, module: Option<String>) {
    let entry = LogEntry {
        timestamp: chrono::Local::now().format("%m-%d | %H:%M:%S%.3f").to_string(),
        level: level.to_string(),
        message: message.to_string(),
        module,
        source: "🦀".to_string(),
    };

    if let Ok(mut logs) = LOGS.lock() {
        if let Some(log_vec) = logs.as_mut() {
            log_vec.push(entry.clone());
            // Keep only the last 1000 entries
            if log_vec.len() > 1000 {
                log_vec.drain(0..log_vec.len() - 1000);
            }
        }
    }
}

/// Log info message
pub fn log_info(message: &str) {
    log_entry("INFO", message, None);
}

/// Log warning message
#[allow(dead_code)]
pub fn log_warn(message: &str) {
    log_entry("WARN", message, None);
}

/// Log error message
pub fn log_error(message: &str) {
    log_entry("ERROR", message, None);
}

/// Log debug message
pub fn log_debug(message: &str) {
    log_entry("DEBUG", message, None);
}

/// Export logs to a file
pub fn export_logs(path: &str) -> std::io::Result<()> {
    let logs = get_logs();
    let mut file = fs::File::create(path)?;

    // Write header
    writeln!(file, "Mehu PLOT3D Viewer - Log Export")?;
    writeln!(file, "================================")?;
    writeln!(
        file,
        "Exported: {}",
        chrono::Local::now().format("%Y-%m-%d %H:%M:%S")
    )?;
    writeln!(file, "Total entries: {}", logs.len())?;
    writeln!(file, "================================\n")?;

    // Write logs in a formatted table
    for log in logs {
        let module_str = log.module.map(|m| format!(" [{}]", m)).unwrap_or_default();
        writeln!(
            file,
            "[{}] {}{} {}",
            log.timestamp, log.level, module_str, log.message
        )?;
    }

    Ok(())
}

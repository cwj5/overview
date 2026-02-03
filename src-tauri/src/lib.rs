mod logger;
mod plot3d;

use logger::{clear_logs, export_logs, get_logs, log_debug, log_error, log_info, LogEntry};
use plot3d::{
    read_plot3d_function, read_plot3d_grid, read_plot3d_grid_ascii, read_plot3d_solution,
    read_plot3d_solution_ascii, Plot3DFunction, Plot3DGrid, Plot3DSolution,
};
use std::path::Path;
use tauri_plugin_dialog::DialogExt;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

/// Load PLOT3D grid file (auto-detects binary format)
#[tauri::command]
fn load_plot3d_file(path: String) -> Result<Vec<Plot3DGrid>, String> {
    log_debug(&format!("Loading PLOT3D grid file: {}", path));
    match read_plot3d_grid(&path) {
        Ok(grids) => {
            log_info(&format!(
                "Successfully loaded {} grid(s) from {}",
                grids.len(),
                path
            ));
            Ok(grids)
        }
        Err(e) => {
            let error_msg = format!("Error loading PLOT3D file: {}", e);
            log_error(&error_msg);
            Err(error_msg)
        }
    }
}

/// Load PLOT3D grid file in ASCII format
#[tauri::command]
fn load_plot3d_file_ascii(path: String) -> Result<Vec<Plot3DGrid>, String> {
    log_debug(&format!("Loading ASCII PLOT3D grid file: {}", path));
    match read_plot3d_grid_ascii(&path) {
        Ok(grids) => {
            log_info(&format!(
                "Successfully loaded {} ASCII grid(s) from {}",
                grids.len(),
                path
            ));
            Ok(grids)
        }
        Err(e) => {
            let error_msg = format!("Error loading ASCII PLOT3D file: {}", e);
            log_error(&error_msg);
            Err(error_msg)
        }
    }
}

/// Load PLOT3D solution file (Q file) in binary format
#[tauri::command]
fn load_plot3d_solution(path: String) -> Result<Vec<Plot3DSolution>, String> {
    log_debug(&format!("Loading PLOT3D solution file: {}", path));
    match read_plot3d_solution(&path) {
        Ok(solutions) => {
            log_info(&format!(
                "Successfully loaded {} solution(s) from {}",
                solutions.len(),
                path
            ));
            Ok(solutions)
        }
        Err(e) => {
            let error_msg = format!("Error loading PLOT3D solution file: {}", e);
            log_error(&error_msg);
            Err(error_msg)
        }
    }
}

/// Load PLOT3D solution file (Q file) in ASCII format
#[tauri::command]
fn load_plot3d_solution_ascii(path: String) -> Result<Vec<Plot3DSolution>, String> {
    log_debug(&format!("Loading ASCII PLOT3D solution file: {}", path));
    match read_plot3d_solution_ascii(&path) {
        Ok(solutions) => {
            log_info(&format!(
                "Successfully loaded {} ASCII solution(s) from {}",
                solutions.len(),
                path
            ));
            Ok(solutions)
        }
        Err(e) => {
            let error_msg = format!("Error loading ASCII PLOT3D solution file: {}", e);
            log_error(&error_msg);
            Err(error_msg)
        }
    }
}

/// Load PLOT3D function file (F file) in binary format
#[tauri::command]
fn load_plot3d_function(path: String) -> Result<Vec<Plot3DFunction>, String> {
    log_debug(&format!("Loading PLOT3D function file: {}", path));
    match read_plot3d_function(&path) {
        Ok(functions) => {
            log_info(&format!(
                "Successfully loaded {} function file(s) from {}",
                functions.len(),
                path
            ));
            Ok(functions)
        }
        Err(e) => {
            let error_msg = format!("Error loading PLOT3D function file: {}", e);
            log_error(&error_msg);
            Err(error_msg)
        }
    }
}

#[tauri::command]
async fn open_file_dialog(app: tauri::AppHandle) -> Result<Option<String>, String> {
    let file_path = app
        .dialog()
        .file()
        .add_filter("PLOT3D Grid Files", &["grid", "xyz"])
        .add_filter("PLOT3D Solution Files", &["q"])
        .add_filter("PLOT3D Function Files", &["f"])
        .add_filter("PLOT3D Files", &["grid", "xyz", "q", "f", "dat", "in"])
        .add_filter("All Files", &["*"])
        .blocking_pick_file();

    Ok(file_path.map(|f| f.to_string()))
}

#[tauri::command]
async fn open_multiple_files_dialog(app: tauri::AppHandle) -> Result<Vec<String>, String> {
    let file_paths = app
        .dialog()
        .file()
        .add_filter("PLOT3D Grid Files", &["grid", "xyz"])
        .add_filter("PLOT3D Solution Files", &["q"])
        .add_filter("PLOT3D Function Files", &["f"])
        .add_filter("PLOT3D Files", &["grid", "xyz", "q", "f", "dat", "in"])
        .add_filter("All Files", &["*"])
        .blocking_pick_files();

    Ok(file_paths
        .map(|files| files.iter().map(|f| f.to_string()).collect())
        .unwrap_or_default())
}

/// Detect if file is ASCII or binary format
#[tauri::command]
fn detect_file_format(path: String) -> Result<String, String> {
    let p = Path::new(&path);

    match p.extension().and_then(|e| e.to_str()) {
        Some("q") | Some("f") => {
            // Try to detect by reading first few bytes
            std::fs::read(&path)
                .map_err(|e| e.to_string())
                .and_then(|data| {
                    if data.len() < 4 {
                        return Ok("unknown".to_string());
                    }

                    // Check if file looks like ASCII (text)
                    let first_chars = &data[..data.len().min(100)];
                    if first_chars
                        .iter()
                        .all(|&b| b.is_ascii_graphic() || b.is_ascii_whitespace())
                    {
                        Ok("ascii".to_string())
                    } else {
                        Ok("binary".to_string())
                    }
                })
        }
        _ => Ok("unknown".to_string()),
    }
}

/// Get all log entries
#[tauri::command]
fn get_log_entries() -> Result<Vec<LogEntry>, String> {
    Ok(get_logs())
}

/// Clear all log entries
#[tauri::command]
fn clear_log_entries() -> Result<(), String> {
    clear_logs();
    Ok(())
}

/// Export logs to a file
#[tauri::command]
fn export_logs_to_file(path: String) -> Result<(), String> {
    export_logs(&path).map_err(|e| {
        let error_msg = format!("Failed to export logs: {}", e);
        log_error(&error_msg);
        error_msg
    })?;
    log_info(&format!("Logs exported to {}", path));
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Initialize logging
    logger::init_logger();
    log_info("Application started");

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            greet,
            load_plot3d_file,
            load_plot3d_file_ascii,
            load_plot3d_solution,
            load_plot3d_solution_ascii,
            load_plot3d_function,
            open_file_dialog,
            open_multiple_files_dialog,
            detect_file_format,
            get_log_entries,
            clear_log_entries,
            export_logs_to_file,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

mod logger;
mod plot3d;

#[cfg(test)]
mod logger_tests;

use logger::{clear_logs, export_logs, get_logs, log_debug, log_error, log_info, LogEntry};
use plot3d::{
    read_plot3d_function, read_plot3d_grid_ascii, read_plot3d_grid_with_metadata,
    read_plot3d_solution, read_plot3d_solution_ascii, MeshGeometry, Plot3DFunction, Plot3DGrid,
    Plot3DSolution,
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
    match read_plot3d_grid_with_metadata(&path) {
        Ok((grids, metadata)) => {
            let dims_str = metadata
                .grid_dimensions
                .iter()
                .enumerate()
                .map(|(idx, d)| format!("Grid {} ({}×{}×{})", idx, d.i, d.j, d.k))
                .collect::<Vec<_>>()
                .join(", ");

            log_info(&format!(
                "Detected byte order: {} (auto-detected)",
                metadata.byte_order
            ));
            log_info(&format!(
                "Loaded {} grid(s): {}",
                metadata.num_grids, dims_str
            ));

            // Debug: Validate grid data
            for (idx, grid) in grids.iter().enumerate() {
                let expected_points = grid.total_points();
                log_debug(&format!(
                    "Grid {}: expected {} points, got x:{}, y:{}, z:{}",
                    idx,
                    expected_points,
                    grid.x_coords.len(),
                    grid.y_coords.len(),
                    grid.z_coords.len()
                ));

                if let Some(ref iblank) = grid.iblank {
                    let blanked_count = iblank.iter().filter(|&&v| v == 0).count();
                    log_info(&format!(
                        "Grid {} has iblank array: {} blanked points ({:.1}%)",
                        idx,
                        blanked_count,
                        (blanked_count as f32 / expected_points as f32) * 100.0
                    ));
                }

                if grid.x_coords.len() > 0 {
                    log_debug(&format!(
                        "Grid {} sample: x[0]={}, y[0]={}, z[0]={}",
                        idx, grid.x_coords[0], grid.y_coords[0], grid.z_coords[0]
                    ));
                }
            }

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

/// Convert PLOT3D grid to Three.js mesh geometry
#[tauri::command]
fn convert_grid_to_mesh(grid: Plot3DGrid) -> Result<MeshGeometry, String> {
    log_debug(&format!(
        "Converting grid ({}x{}x{}) to mesh geometry",
        grid.dimensions.i, grid.dimensions.j, grid.dimensions.k
    ));

    // Validate grid data
    let total_points = grid.total_points();
    if grid.x_coords.len() != total_points {
        let error_msg = format!(
            "Invalid grid: x_coords length {} != expected {} ({}x{}x{})",
            grid.x_coords.len(),
            total_points,
            grid.dimensions.i,
            grid.dimensions.j,
            grid.dimensions.k
        );
        log_error(&error_msg);
        return Err(error_msg);
    }
    if grid.y_coords.len() != total_points {
        let error_msg = format!(
            "Invalid grid: y_coords length {} != expected {}",
            grid.y_coords.len(),
            total_points
        );
        log_error(&error_msg);
        return Err(error_msg);
    }
    if grid.z_coords.len() != total_points {
        let error_msg = format!(
            "Invalid grid: z_coords length {} != expected {}",
            grid.z_coords.len(),
            total_points
        );
        log_error(&error_msg);
        return Err(error_msg);
    }

    let mesh = grid.to_mesh_geometry();

    log_info(&format!(
        "Generated mesh with {} vertices and {} faces",
        mesh.vertex_count, mesh.face_count
    ));

    Ok(mesh)
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

/// Open save file dialog for log export
#[tauri::command]
async fn save_log_file_dialog(app: tauri::AppHandle) -> Result<Option<String>, String> {
    let file_path = app
        .dialog()
        .file()
        .add_filter("Text Files", &["txt"])
        .add_filter("All Files", &["*"])
        .set_file_name("mehu-logs.txt")
        .blocking_save_file();

    Ok(file_path.map(|f| f.to_string()))
}

/// Write text content to a file
#[tauri::command]
fn write_text_file(path: String, contents: String) -> Result<(), String> {
    use std::fs;
    use std::io::Write;

    let mut file =
        fs::File::create(&path).map_err(|e| format!("Failed to create file {}: {}", path, e))?;

    file.write_all(contents.as_bytes())
        .map_err(|e| format!("Failed to write to file {}: {}", path, e))?;

    log_info(&format!(
        "Successfully wrote {} bytes to {}",
        contents.len(),
        path
    ));
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
            convert_grid_to_mesh,
            open_file_dialog,
            open_multiple_files_dialog,
            detect_file_format,
            get_log_entries,
            clear_log_entries,
            export_logs_to_file,
            save_log_file_dialog,
            write_text_file,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

// Copyright 2026 Charles W Jackson
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

mod logger;
mod plot3d;
mod solution;

#[cfg(test)]
mod logger_tests;

use logger::{clear_logs, export_logs, get_logs, log_debug, log_error, log_info, LogEntry};
use plot3d::{
    get_last_solution_metadata, read_plot3d_function, read_plot3d_grid_ascii,
    read_plot3d_grid_with_metadata, read_plot3d_solution, read_plot3d_solution_ascii, MeshGeometry,
    Plot3DFunction, Plot3DGrid, Plot3DSolution, SolutionFileMetadata,
};
use std::cell::RefCell;
use std::path::Path;
use tauri::webview::WebviewWindow;
use tauri::Manager;
use tauri_plugin_dialog::DialogExt;

// Thread-local storage for solution file metadata
thread_local! {
    static SOLUTION_METADATA: RefCell<Option<SolutionFileMetadata>> = RefCell::new(None);
}

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

/// Load PLOT3D grid file (auto-detects binary format)
#[tauri::command]
fn load_plot3d_file(path: String) -> Result<Vec<Plot3DGrid>, String> {
    match read_plot3d_grid_with_metadata(&path) {
        Ok((grids, metadata)) => {
            let dims_str = metadata
                .grid_dimensions
                .iter()
                .enumerate()
                .map(|(idx, d)| format!("Grid {} ({}×{}×{})", idx + 1, d.i, d.j, d.k))
                .collect::<Vec<_>>()
                .join(", ");

            log_info(&format!(
                "Loaded grid file {} (endianness: {}, precision: {}, iblank: {})",
                path,
                metadata.byte_order,
                metadata.precision,
                if metadata.has_iblank { "yes" } else { "no" }
            ));
            log_info(&format!("Grids: {}", dims_str));

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
    match read_plot3d_grid_ascii(&path) {
        Ok(grids) => {
            let dims_str = grids
                .iter()
                .enumerate()
                .map(|(idx, d)| {
                    format!(
                        "Grid {} ({}×{}×{})",
                        idx, d.dimensions.i, d.dimensions.j, d.dimensions.k
                    )
                })
                .collect::<Vec<_>>()
                .join(", ");

            log_info(&format!(
                "Loaded ASCII grid file {} (endianness: ASCII, precision: f32, iblank: no)",
                path
            ));
            log_info(&format!("Grids: {}", dims_str));
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
            // Get the metadata that was set by the reader
            if let Some(metadata) = get_last_solution_metadata() {
                log_info(&format!(
                    "Loaded solution file {} ({} format, {} precision, endianness: {})",
                    path, metadata.format, metadata.precision, metadata.byte_order
                ));
            } else {
                log_info(&format!(
                    "Successfully loaded {} solution(s) from {} (binary format)",
                    solutions.len(),
                    path
                ));
            }
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
            // Get the metadata that was set by the reader
            if let Some(metadata) = get_last_solution_metadata() {
                log_info(&format!(
                    "Loaded solution file {} ({} format, {} precision)",
                    path, metadata.format, metadata.precision
                ));
            } else {
                log_info(&format!(
                    "Successfully loaded {} solution(s) from {} (ASCII format)",
                    solutions.len(),
                    path
                ));
            }
            Ok(solutions)
        }
        Err(e) => {
            let error_msg = format!("Error loading ASCII PLOT3D solution file: {}", e);
            log_error(&error_msg);
            Err(error_msg)
        }
    }
}

/// Load PLOT3D solution file (Q file) - auto-detects binary or ASCII format
#[tauri::command]
fn load_plot3d_solution_auto(path: String) -> Result<Vec<Plot3DSolution>, String> {
    log_debug(&format!(
        "Loading PLOT3D solution file (auto-detect): {}",
        path
    ));

    // First, check file size and basic properties
    use std::fs;
    let metadata = fs::metadata(&path).map_err(|e| format!("Failed to read file: {}", e))?;

    log_debug(&format!("File size: {} bytes", metadata.len()));

    if metadata.len() == 0 {
        return Err("Solution file is empty".to_string());
    }

    // Try to detect file type by reading first few bytes
    let file_bytes = fs::read(&path).map_err(|e| format!("Failed to read file: {}", e))?;

    let is_likely_text = file_bytes
        .iter()
        .take(500)
        .all(|&b| b == b'\n' || b == b'\r' || b == b'\t' || (b >= 32 && b < 127));

    log_debug(&format!(
        "File appears to be: {}",
        if is_likely_text {
            "text (ASCII)"
        } else {
            "binary"
        }
    ));

    // Try binary format first (more specific format)
    match read_plot3d_solution(&path) {
        Ok(solutions) => {
            // Get the metadata that was set by the reader
            if let Some(metadata) = get_last_solution_metadata() {
                log_info(&format!(
                    "Loaded solution file {} ({} format, {} precision, endianness: {})",
                    path, metadata.format, metadata.precision, metadata.byte_order
                ));
            } else {
                log_info(&format!(
                    "Successfully loaded {} solution(s) from {} (binary format)",
                    solutions.len(),
                    path
                ));
            }
            Ok(solutions)
        }
        Err(binary_err) => {
            // Binary failed, try ASCII format
            log_debug(&format!("Binary format failed: {}", binary_err));
            match read_plot3d_solution_ascii(&path) {
                Ok(solutions) => {
                    // Get the metadata that was set by the reader
                    if let Some(metadata) = get_last_solution_metadata() {
                        log_info(&format!(
                            "Loaded solution file {} ({} format, {} precision)",
                            path, metadata.format, metadata.precision
                        ));
                    } else {
                        log_info(&format!(
                            "Successfully loaded {} solution(s) from {} (ASCII format)",
                            solutions.len(),
                            path
                        ));
                    }
                    Ok(solutions)
                }
                Err(ascii_err) => {
                    log_debug(&format!("ASCII format failed: {}", ascii_err));
                    let file_type = if is_likely_text {
                        "text file"
                    } else {
                        "binary file"
                    };
                    let error_msg = format!(
                        "Failed to load solution file (detected as {}). Binary reader: {}. ASCII reader: {}",
                        file_type, binary_err, ascii_err
                    );
                    log_error(&error_msg);
                    Err(error_msg)
                }
            }
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

    Ok(mesh)
}

/// Compute scalar field colors for a grid with solution data
#[tauri::command]
fn compute_solution_colors(
    grid: Plot3DGrid,
    solution: Plot3DSolution,
    field: String,
) -> Result<MeshGeometry, String> {
    use solution::{compute_colors, compute_scalar_field, ScalarField};

    // Parse field type
    let field_enum =
        ScalarField::from_str(&field).ok_or_else(|| format!("Unknown scalar field: {}", field))?;

    // Validate solution matches grid
    let grid_points = grid.total_points();
    if solution.rho.len() != grid_points {
        return Err(format!(
            "Solution points {} != grid points {}",
            solution.rho.len(),
            grid_points
        ));
    }

    // Compute the scalar field values
    let values = compute_scalar_field(&solution, field_enum);

    // Generate colors from scalar values
    let colors = compute_colors(&values);

    // Create mesh geometry
    let mut mesh = grid.to_mesh_geometry();
    mesh.colors = Some(colors);

    log_info(&format!(
        "Computed {} colors for {} vertices",
        mesh.colors.as_ref().unwrap_or(&Vec::new()).len() / 3,
        grid_points
    ));

    Ok(mesh)
}

#[tauri::command]
async fn open_file_dialog(app: tauri::AppHandle) -> Result<Option<String>, String> {
    let file_path = app.dialog().file().blocking_pick_file();

    Ok(file_path.map(|f| f.to_string()))
}

#[tauri::command]
async fn open_multiple_files_dialog(app: tauri::AppHandle) -> Result<Vec<String>, String> {
    let file_paths = app.dialog().file().blocking_pick_files();

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

/// Open the About window
#[tauri::command]
async fn open_about_window(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("about") {
        let _ = window.set_focus();
        Ok(())
    } else {
        WebviewWindow::builder(&app, "about", tauri::WebviewUrl::App("/about.html".into()))
            .title("About Mehu")
            .inner_size(600.0, 700.0)
            .resizable(true)
            .minimizable(true)
            .maximizable(false)
            .build()
            .map_err(|e| format!("Failed to create About window: {}", e))?;
        Ok(())
    }
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
            load_plot3d_solution_auto,
            load_plot3d_function,
            convert_grid_to_mesh,
            compute_solution_colors,
            open_file_dialog,
            open_multiple_files_dialog,
            detect_file_format,
            get_log_entries,
            clear_log_entries,
            export_logs_to_file,
            save_log_file_dialog,
            write_text_file,
            open_about_window,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

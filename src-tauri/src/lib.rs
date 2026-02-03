mod plot3d;

use plot3d::{read_plot3d_grid, Plot3DGrid};
use tauri_plugin_dialog::DialogExt;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
fn load_plot3d_file(path: String) -> Result<Vec<Plot3DGrid>, String> {
    read_plot3d_grid(path).map_err(|e| e.to_string())
}

#[tauri::command]
async fn open_file_dialog(app: tauri::AppHandle) -> Result<Option<String>, String> {
    let file_path = app
        .dialog()
        .file()
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
        .add_filter("PLOT3D Files", &["grid", "xyz", "q", "f", "dat", "in"])
        .add_filter("All Files", &["*"])
        .blocking_pick_files();

    Ok(file_paths
        .map(|files| files.iter().map(|f| f.to_string()).collect())
        .unwrap_or_default())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            greet,
            load_plot3d_file,
            open_file_dialog,
            open_multiple_files_dialog
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

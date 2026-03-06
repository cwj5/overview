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
use once_cell::sync::Lazy;
use plot3d::{
    get_last_solution_metadata, read_plot3d_function, read_plot3d_grid_ascii,
    read_plot3d_grid_with_metadata, read_plot3d_solution, read_plot3d_solution_ascii,
    GridDimensions, MeshGeometry, Plot3DFunction, Plot3DGrid, Plot3DSolution, SolutionFileMetadata,
};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};
use tauri::webview::WebviewWindow;
use tauri::Emitter;
use tauri::Manager;
use tauri_plugin_dialog::DialogExt;

// Thread-local storage for solution file metadata
thread_local! {
    static SOLUTION_METADATA: RefCell<Option<SolutionFileMetadata>> = RefCell::new(None);
}

// Deprecated: Legacy solution cache - keeping for backward compatibility during migration
static SOLUTION_CACHE: Lazy<Mutex<Vec<Arc<Plot3DSolution>>>> = Lazy::new(|| Mutex::new(Vec::new()));

fn cache_solutions(solutions: &[Plot3DSolution]) {
    let cached: Vec<Arc<Plot3DSolution>> = solutions
        .iter()
        .map(|solution| Arc::new(solution.clone()))
        .collect();
    if let Ok(mut store) = SOLUTION_CACHE.lock() {
        *store = cached;
    }
}

// ============================================================================
// NEW: Grid and Solution Cache Architecture
// ============================================================================

/// Cached grid entry with metadata
#[derive(Clone, Debug, Serialize)]
struct CachedGrid {
    id: String,
    grid: Arc<Plot3DGrid>,
    file_path: String,
    file_name: String,
    grid_index: usize,
    has_iblank: bool,
}

/// Cached solution entry with metadata
#[derive(Clone, Debug, Serialize)]
struct CachedSolution {
    id: String,
    solution: Arc<Plot3DSolution>,
    file_path: String,
    file_name: String,
    grid_index: usize,
}

/// Metadata about a cached grid (no coordinate arrays)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GridMetadata {
    pub id: String,
    pub file_path: String,
    pub file_name: String,
    pub grid_index: usize,
    pub dimensions: GridDimensions,
    pub has_iblank: bool,
    pub has_solution: bool,
}

/// Metadata about a cached solution (no arrays)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SolutionMetadata {
    pub id: String,
    pub file_path: String,
    pub file_name: String,
    pub grid_index: usize,
    pub dimensions: GridDimensions,
}

/// Global grid cache: grid_id -> CachedGrid
static GRID_CACHE: Lazy<Mutex<HashMap<String, CachedGrid>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

/// Global solution cache: solution_id -> CachedSolution
static SOLUTION_CACHE_V2: Lazy<Mutex<HashMap<String, CachedSolution>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

/// Counter for generating unique grid IDs
static GRID_ID_COUNTER: Lazy<Mutex<u64>> = Lazy::new(|| Mutex::new(0));

/// Generate a unique grid ID
fn generate_grid_id(file_path: &str, grid_index: usize) -> String {
    let mut counter = GRID_ID_COUNTER.lock().unwrap();
    *counter += 1;
    format!(
        "grid_{}_{}_idx{}_{}",
        *counter,
        Path::new(file_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown"),
        grid_index,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis()
            % 100000
    )
}

/// Generate a unique solution ID
fn generate_solution_id(file_path: &str, grid_index: usize) -> String {
    let mut counter = GRID_ID_COUNTER.lock().unwrap();
    *counter += 1;
    format!(
        "solution_{}_{}_idx{}_{}",
        *counter,
        Path::new(file_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown"),
        grid_index,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis()
            % 100000
    )
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
            cache_solutions(&solutions);
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
            cache_solutions(&solutions);
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
            cache_solutions(&solutions);
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
                    cache_solutions(&solutions);
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

// ============================================================================
// NEW: V2 Load Commands that Cache and Return Metadata
// ============================================================================

/// Load PLOT3D grid file (caches grids and returns metadata)
#[tauri::command]
fn load_plot3d_file_cached(path: String) -> Result<Vec<GridMetadata>, String> {
    // Load grids using existing reader
    let (grids, file_metadata) = read_plot3d_grid_with_metadata(&path).map_err(|e| {
        let error_msg = format!("Error loading PLOT3D file: {}", e);
        log_error(&error_msg);
        error_msg
    })?;

    let file_name = Path::new(&path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();

    let dims_str = file_metadata
        .grid_dimensions
        .iter()
        .enumerate()
        .map(|(idx, d)| format!("Grid {} ({}×{}×{})", idx + 1, d.i, d.j, d.k))
        .collect::<Vec<_>>()
        .join(", ");

    log_info(&format!(
        "Loaded grid file {} (endianness: {}, precision: {}, iblank: {})",
        path,
        file_metadata.byte_order,
        file_metadata.precision,
        if file_metadata.has_iblank {
            "yes"
        } else {
            "no"
        }
    ));
    log_info(&format!("Grids: {}", dims_str));

    // Cache grids and generate metadata
    let mut cache = GRID_CACHE
        .lock()
        .map_err(|_| "Grid cache lock poisoned".to_string())?;

    let mut metadata_list = Vec::new();

    for (grid_index, grid) in grids.into_iter().enumerate() {
        let grid_id = generate_grid_id(&path, grid_index);
        let has_iblank = grid.iblank.is_some();

        let cached_grid = CachedGrid {
            id: grid_id.clone(),
            grid: Arc::new(grid.clone()),
            file_path: path.clone(),
            file_name: file_name.clone(),
            grid_index,
            has_iblank,
        };

        cache.insert(grid_id.clone(), cached_grid);

        metadata_list.push(GridMetadata {
            id: grid_id,
            file_path: path.clone(),
            file_name: file_name.clone(),
            grid_index,
            dimensions: grid.dimensions,
            has_iblank,
            has_solution: false, // Will be updated when solution is loaded
        });
    }

    log_info(&format!("Cached {} grids", metadata_list.len()));

    Ok(metadata_list)
}

/// Load PLOT3D solution file (caches solutions and returns metadata)
#[tauri::command]
fn load_plot3d_solution_cached(path: String) -> Result<Vec<SolutionMetadata>, String> {
    log_debug(&format!("Loading PLOT3D solution file (v2): {}", path));

    // Load solutions using existing reader (auto-detects format)
    let (solutions, _) = {
        // Try binary first
        match read_plot3d_solution(&path) {
            Ok(solutions) => {
                let metadata = get_last_solution_metadata();
                (solutions, metadata)
            }
            Err(binary_err) => {
                // Try ASCII
                match read_plot3d_solution_ascii(&path) {
                    Ok(solutions) => {
                        let metadata = get_last_solution_metadata();
                        (solutions, metadata)
                    }
                    Err(ascii_err) => {
                        let error_msg = format!(
                            "Failed to load solution file. Binary: {}. ASCII: {}",
                            binary_err, ascii_err
                        );
                        log_error(&error_msg);
                        return Err(error_msg);
                    }
                }
            }
        }
    };

    let file_name = Path::new(&path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();

    if let Some(metadata) = get_last_solution_metadata() {
        log_info(&format!(
            "Loaded solution file {} ({} format, {} precision, endianness: {})",
            path, metadata.format, metadata.precision, metadata.byte_order
        ));
    } else {
        log_info(&format!(
            "Successfully loaded {} solution(s) from {}",
            solutions.len(),
            path
        ));
    }

    // Cache solutions for old API compatibility
    cache_solutions(&solutions);

    // Cache solutions in v2 cache and generate metadata
    let mut cache = SOLUTION_CACHE_V2
        .lock()
        .map_err(|_| "Solution cache lock poisoned".to_string())?;

    let mut metadata_list = Vec::new();

    for solution in solutions.into_iter() {
        let grid_index = solution.grid_index;
        let solution_id = generate_solution_id(&path, grid_index);

        let cached_solution = CachedSolution {
            id: solution_id.clone(),
            solution: Arc::new(solution.clone()),
            file_path: path.clone(),
            file_name: file_name.clone(),
            grid_index,
        };

        cache.insert(solution_id.clone(), cached_solution);

        metadata_list.push(SolutionMetadata {
            id: solution_id,
            file_path: path.clone(),
            file_name: file_name.clone(),
            grid_index,
            dimensions: solution.dimensions,
        });
    }

    log_info(&format!("Cached {} solutions", metadata_list.len()));

    Ok(metadata_list)
}

/// Convert PLOT3D grid to Three.js mesh geometry
#[tauri::command]
fn convert_grid_to_mesh(
    grid: Plot3DGrid,
    respect_iblank: Option<bool>,
    window: WebviewWindow,
) -> Result<MeshGeometry, String> {
    // Emit loading start event
    let _ = window.emit("loading-start", "Converting grid to mesh...");

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

    // Auto-detect decimation based on grid size for better performance
    let i = grid.dimensions.i as usize;
    let j = grid.dimensions.j as usize;
    let max_dim = i.max(j);

    let decimation_factor = if max_dim > 1000 {
        4 // Very large grids: use 1/4 resolution
    } else if max_dim > 500 {
        3 // Large grids: use 1/3 resolution
    } else if max_dim > 250 {
        2 // Medium grids: use 1/2 resolution
    } else {
        1 // Small grids: full resolution
    };

    if decimation_factor > 1 {
        log_info(&format!(
            "Grid size {}x{} - applying {}x decimation for performance",
            i, j, decimation_factor
        ));
    }

    let mesh =
        grid.to_mesh_surface_geometry_decimated(respect_iblank.unwrap_or(false), decimation_factor);

    // Emit loading end event
    let _ = window.emit("loading-end", ());

    Ok(mesh)
}

/// Helper function to compute a scalar field value from a single point's solution data
fn compute_scalar_field_value(solution: &Plot3DSolution, field: solution::ScalarField) -> f32 {
    use solution::ScalarField;

    if solution.rho.is_empty() {
        return 0.0;
    }

    match field {
        ScalarField::Density => solution.rho[0],

        ScalarField::VelocityMagnitude => {
            let rho = solution.rho[0];
            if rho > 0.0 {
                let u = solution.rhou[0] / rho;
                let v = solution.rhov[0] / rho;
                let w = solution.rhow[0] / rho;
                (u * u + v * v + w * w).sqrt()
            } else {
                0.0
            }
        }

        ScalarField::MomentumX => solution.rhou[0],
        ScalarField::MomentumY => solution.rhov[0],
        ScalarField::MomentumZ => solution.rhow[0],

        ScalarField::Pressure => {
            const DEFAULT_GAMMA: f32 = 1.4;
            let rho = solution.rho[0];
            if rho > 0.0 {
                let gamma = solution
                    .gamma
                    .as_ref()
                    .map(|g| g[0])
                    .unwrap_or(DEFAULT_GAMMA);
                let u = solution.rhou[0] / rho;
                let v = solution.rhov[0] / rho;
                let w = solution.rhow[0] / rho;
                let kinetic_energy = 0.5 * rho * (u * u + v * v + w * w);
                let internal_energy = solution.rhoe[0] - kinetic_energy;
                (gamma - 1.0) * internal_energy
            } else {
                0.0
            }
        }

        ScalarField::Energy => solution.rhoe[0],
    }
}

/// Compute scalar field value directly from conservative variables at a point
fn compute_scalar_field_from_components(
    rho: f32,
    rhou: f32,
    rhov: f32,
    rhow: f32,
    rhoe: f32,
    gamma: Option<f32>,
    field: solution::ScalarField,
) -> f32 {
    use solution::ScalarField;

    match field {
        ScalarField::Density => rho,
        ScalarField::VelocityMagnitude => {
            if rho > 0.0 {
                let u = rhou / rho;
                let v = rhov / rho;
                let w = rhow / rho;
                (u * u + v * v + w * w).sqrt()
            } else {
                0.0
            }
        }
        ScalarField::MomentumX => rhou,
        ScalarField::MomentumY => rhov,
        ScalarField::MomentumZ => rhow,
        ScalarField::Pressure => {
            const DEFAULT_GAMMA: f32 = 1.4;
            if rho > 0.0 {
                let gamma = gamma.unwrap_or(DEFAULT_GAMMA);
                let u = rhou / rho;
                let v = rhov / rho;
                let w = rhow / rho;
                let kinetic_energy = 0.5 * rho * (u * u + v * v + w * w);
                let internal_energy = rhoe - kinetic_energy;
                (gamma - 1.0) * internal_energy
            } else {
                0.0
            }
        }
        ScalarField::Energy => rhoe,
    }
}

// ============================================================================
// NEW: ID-Based Compute Commands (Phase 2)
// ============================================================================

/// Convert cached grid to mesh geometry (ID-based)
#[allow(non_snake_case)]
#[tauri::command]
fn convert_grid_to_mesh_by_id(
    gridId: String,
    respect_iblank: Option<bool>,
    window: WebviewWindow,
) -> Result<MeshGeometry, String> {
    let _ = window.emit("loading-start", "Converting grid to mesh...");

    // Load grid from cache
    let grid = {
        let cache = GRID_CACHE
            .lock()
            .map_err(|_| "Grid cache lock poisoned".to_string())?;
        let cached = cache
            .get(&gridId)
            .ok_or_else(|| format!("Grid not found in cache: {}", gridId))?;
        Arc::clone(&cached.grid)
    };

    // Validate grid data
    let total_points = grid.total_points();
    if grid.x_coords.len() != total_points {
        return Err(format!(
            "Invalid grid: x_coords length {} != expected {}",
            grid.x_coords.len(),
            total_points
        ));
    }

    // Auto-detect decimation
    let i = grid.dimensions.i as usize;
    let j = grid.dimensions.j as usize;
    let max_dim = i.max(j);

    let decimation_factor = if max_dim > 1000 {
        4
    } else if max_dim > 500 {
        3
    } else if max_dim > 250 {
        2
    } else {
        1
    };

    if decimation_factor > 1 {
        log_info(&format!(
            "Grid size {}x{} - applying {}x decimation for performance",
            i, j, decimation_factor
        ));
    }

    let mesh =
        grid.to_mesh_surface_geometry_decimated(respect_iblank.unwrap_or(false), decimation_factor);

    let _ = window.emit("loading-end", ());

    Ok(mesh)
}

/// Slice a cached grid along I/J/K plane (ID-based)
#[allow(non_snake_case)]
#[tauri::command]
fn slice_grid_by_id(gridId: String, plane: String, index: u32) -> Result<Plot3DGrid, String> {
    log_debug(&format!(
        "Slicing cached grid {} along {} plane at index {}",
        gridId, plane, index
    ));

    // Load grid from cache
    let grid = {
        let cache = GRID_CACHE
            .lock()
            .map_err(|_| "Grid cache lock poisoned".to_string())?;
        let cached = cache
            .get(&gridId)
            .ok_or_else(|| format!("Grid not found in cache: {}", gridId))?;
        Arc::clone(&cached.grid)
    };

    grid.slice_grid(&plane, index).map_err(|e| {
        let error_msg = format!("Failed to slice grid: {}", e);
        log_error(&error_msg);
        error_msg
    })
}

/// Slice a cached grid with arbitrary plane (ID-based)
#[allow(non_snake_case)]
#[tauri::command]
fn slice_arbitrary_plane_by_id(
    gridId: String,
    planePoint: [f32; 3],
    planeNormal: [f32; 3],
    respect_iblank: Option<bool>,
    _window: WebviewWindow,
) -> Result<MeshGeometry, String> {
    log_debug(&format!(
        "Slicing cached grid {} with arbitrary plane: point={:?}, normal={:?}",
        gridId, planePoint, planeNormal
    ));

    // Load grid from cache
    let grid = {
        let cache = GRID_CACHE
            .lock()
            .map_err(|_| "Grid cache lock poisoned".to_string())?;
        let cached = cache
            .get(&gridId)
            .ok_or_else(|| format!("Grid not found in cache: {}", gridId))?;
        Arc::clone(&cached.grid)
    };

    let result =
        grid.slice_arbitrary_plane(planePoint, planeNormal, respect_iblank.unwrap_or(false));

    match &result {
        Ok(mesh) => {
            log_info(&format!(
                "Arbitrary plane slice generated: {} vertices, {} triangles",
                mesh.vertex_count,
                mesh.triangle_indices.len() / 3
            ));
        }
        Err(e) => {
            log_error(&format!("Failed to slice arbitrary plane: {}", e));
        }
    }

    result
}

/// Compute solution colors using cached grid and solution (ID-based)
#[allow(non_snake_case)]
#[tauri::command]
fn compute_solution_colors(
    gridId: String,
    solutionId: String,
    field: String,
    colorScheme: String,
    respect_iblank: Option<bool>,
    window: WebviewWindow,
) -> Result<MeshGeometry, String> {
    use solution::{compute_colors, compute_scalar_field_surface, ColorScheme, ScalarField};

    let _ = window.emit("loading-start", format!("Computing {} field...", field));

    // Load grid from cache
    let (grid, grid_file_path, grid_index) = {
        let cache = GRID_CACHE
            .lock()
            .map_err(|_| "Grid cache lock poisoned".to_string())?;
        let cached = cache
            .get(&gridId)
            .ok_or_else(|| format!("Grid not found in cache: {}", gridId))?;
        (
            Arc::clone(&cached.grid),
            cached.file_path.clone(),
            cached.grid_index,
        )
    };

    // Load solution from cache
    let (solution, solution_file_path, solution_grid_index) = {
        let cache = SOLUTION_CACHE_V2
            .lock()
            .map_err(|_| "Solution cache lock poisoned".to_string())?;
        let cached = cache
            .get(&solutionId)
            .ok_or_else(|| format!("Solution not found in cache: {}", solutionId))?;
        (
            Arc::clone(&cached.solution),
            cached.file_path.clone(),
            cached.grid_index,
        )
    };

    if grid_index != solution_grid_index {
        return Err(format!(
            "Grid/solution mismatch: grid/solution index differs: grid(id={}, index={}) vs solution(id={}, index={})",
            gridId, grid_index, solutionId, solution_grid_index
        ));
    }

    if grid.dimensions.i != solution.dimensions.i
        || grid.dimensions.j != solution.dimensions.j
        || grid.dimensions.k != solution.dimensions.k
    {
        return Err(format!(
            "Grid/solution mismatch: dimensions differ: grid(id={}, dims={}x{}x{}) vs solution(id={}, dims={}x{}x{})",
            gridId,
            grid.dimensions.i,
            grid.dimensions.j,
            grid.dimensions.k,
            solutionId,
            solution.dimensions.i,
            solution.dimensions.j,
            solution.dimensions.k
        ));
    }

    if grid_file_path != solution_file_path {
        log_debug(&format!(
            "Grid/solution file paths differ but pair accepted by index+dimensions: grid(id={}, file={}) solution(id={}, file={})",
            gridId, grid_file_path, solutionId, solution_file_path
        ));
    }

    // Parse field and scheme
    let field_enum =
        ScalarField::from_str(&field).ok_or_else(|| format!("Unknown scalar field: {}", field))?;
    let scheme = ColorScheme::from_str(&colorScheme)
        .ok_or_else(|| format!("Unknown color scheme: {}", colorScheme))?;

    // Validate
    let grid_points = grid.total_points();
    if solution.rho.len() != grid_points {
        return Err(format!(
            "Solution points {} != grid points {}",
            solution.rho.len(),
            grid_points
        ));
    }

    // Auto-detect decimation
    let i = grid.dimensions.i as usize;
    let j = grid.dimensions.j as usize;
    let max_dim = i.max(j);

    let decimation_factor = if max_dim > 1000 {
        4
    } else if max_dim > 500 {
        3
    } else if max_dim > 250 {
        2
    } else {
        1
    };

    if decimation_factor > 1 {
        log_info(&format!(
            "Solution grid size {}x{} - applying {}x decimation for performance",
            i, j, decimation_factor
        ));
    }

    // Compute colors
    let values = compute_scalar_field_surface(&solution, field_enum, decimation_factor);
    let colors = compute_colors(&values, &scheme);

    let mut mesh =
        grid.to_mesh_surface_geometry_decimated(respect_iblank.unwrap_or(false), decimation_factor);
    mesh.colors = Some(colors);

    // Filter colors to match blanked vertices
    if respect_iblank.unwrap_or(false) {
        if let (Some(iblank), Some(colors)) = (grid.iblank.as_ref(), mesh.colors.take()) {
            let decimation = decimation_factor.max(1);
            let grid_i = grid.dimensions.i as usize;
            let grid_j = grid.dimensions.j as usize;
            let i_decimated = ((grid_i - 1) / decimation) + 1;
            let j_decimated = ((grid_j - 1) / decimation) + 1;

            let mut filtered_colors = Vec::new();
            for j_step in 0..j_decimated {
                let j_idx = (j_step * decimation).min(grid_j - 1);
                for i_step in 0..i_decimated {
                    let i_idx = (i_step * decimation).min(grid_i - 1);
                    let grid_idx = j_idx * grid_i + i_idx;
                    if iblank[grid_idx] != 0 {
                        let grid_vertex_idx = j_step * i_decimated + i_step;
                        let color_idx = grid_vertex_idx * 3;
                        if color_idx + 2 < colors.len() {
                            filtered_colors.push(colors[color_idx]);
                            filtered_colors.push(colors[color_idx + 1]);
                            filtered_colors.push(colors[color_idx + 2]);
                        }
                    }
                }
            }

            mesh.colors = if filtered_colors.is_empty() {
                None
            } else {
                Some(filtered_colors)
            };
        }
    }
    let _ = window.emit("loading-end", ());

    Ok(mesh)
}

/// Compute solution colors for sliced grid using cached data (ID-based)
#[allow(non_snake_case)]
#[tauri::command]
fn compute_solution_colors_sliced(
    gridId: String,
    solutionId: String,
    slicePlane: String,
    sliceIndex: u32,
    field: String,
    colorScheme: String,
    respect_iblank: Option<bool>,
    window: WebviewWindow,
) -> Result<MeshGeometry, String> {
    use solution::{compute_colors, ColorScheme, ScalarField};

    let _ = window.emit(
        "loading-start",
        format!("Computing {} field on slice...", field),
    );

    // Load grid from cache
    let (original_grid, grid_file_path, grid_index) = {
        let cache = GRID_CACHE
            .lock()
            .map_err(|_| "Grid cache lock poisoned".to_string())?;
        let cached = cache
            .get(&gridId)
            .ok_or_else(|| format!("Grid not found in cache: {}", gridId))?;
        (
            Arc::clone(&cached.grid),
            cached.file_path.clone(),
            cached.grid_index,
        )
    };

    // Load solution from cache
    let (solution, solution_file_path, solution_grid_index) = {
        let cache = SOLUTION_CACHE_V2
            .lock()
            .map_err(|_| "Solution cache lock poisoned".to_string())?;
        let cached = cache
            .get(&solutionId)
            .ok_or_else(|| format!("Solution not found in cache: {}", solutionId))?;
        (
            Arc::clone(&cached.solution),
            cached.file_path.clone(),
            cached.grid_index,
        )
    };

    if grid_index != solution_grid_index {
        return Err(format!(
            "Grid/solution mismatch: grid/solution index differs: grid(id={}, index={}) vs solution(id={}, index={})",
            gridId, grid_index, solutionId, solution_grid_index
        ));
    }

    if original_grid.dimensions.i != solution.dimensions.i
        || original_grid.dimensions.j != solution.dimensions.j
        || original_grid.dimensions.k != solution.dimensions.k
    {
        return Err(format!(
            "Grid/solution mismatch: dimensions differ: grid(id={}, dims={}x{}x{}) vs solution(id={}, dims={}x{}x{})",
            gridId,
            original_grid.dimensions.i,
            original_grid.dimensions.j,
            original_grid.dimensions.k,
            solutionId,
            solution.dimensions.i,
            solution.dimensions.j,
            solution.dimensions.k
        ));
    }

    if grid_file_path != solution_file_path {
        log_debug(&format!(
            "Grid/solution file paths differ but pair accepted by index+dimensions: grid(id={}, file={}) solution(id={}, file={})",
            gridId, grid_file_path, solutionId, solution_file_path
        ));
    }

    // Parse field and scheme
    let field_enum =
        ScalarField::from_str(&field).ok_or_else(|| format!("Unknown scalar field: {}", field))?;
    let scheme = ColorScheme::from_str(&colorScheme)
        .ok_or_else(|| format!("Unknown color scheme: {}", colorScheme))?;

    // Perform slice
    let sliced_grid = original_grid
        .slice_grid(&slicePlane, sliceIndex)
        .map_err(|e| format!("Failed to slice grid: {}", e))?;

    // Validate solution matches original grid
    let grid_points = original_grid.total_points();
    if solution.rho.len() != grid_points {
        return Err(format!(
            "Solution points {} != grid points {}",
            solution.rho.len(),
            grid_points
        ));
    }

    // Extract dimensions
    let i_orig = original_grid.dimensions.i as usize;
    let j_orig = original_grid.dimensions.j as usize;
    let _k_orig = original_grid.dimensions.k as usize;

    let i_slice = sliced_grid.dimensions.i as usize;
    let j_slice = sliced_grid.dimensions.j as usize;

    let slice_idx = sliceIndex as usize;

    // Map each point in sliced grid to original grid for solution values
    let mut values = Vec::with_capacity(sliced_grid.total_points());

    let linear_index_original =
        |i: usize, j: usize, k: usize| -> usize { i + j * i_orig + k * i_orig * j_orig };

    match slicePlane.to_uppercase().as_str() {
        "K" => {
            for j_idx in 0..j_slice {
                for i_idx in 0..i_slice {
                    let orig_linear = linear_index_original(i_idx, j_idx, slice_idx);
                    let point_solution = create_point_solution(&solution, orig_linear);
                    values.push(compute_scalar_field_value(&point_solution, field_enum));
                }
            }
        }
        "J" => {
            for k_idx in 0..j_slice {
                for i_idx in 0..i_slice {
                    let orig_linear = linear_index_original(i_idx, slice_idx, k_idx);
                    let point_solution = create_point_solution(&solution, orig_linear);
                    values.push(compute_scalar_field_value(&point_solution, field_enum));
                }
            }
        }
        "I" => {
            for k_idx in 0..j_slice {
                for j_idx in 0..i_slice {
                    let orig_linear = linear_index_original(slice_idx, j_idx, k_idx);
                    let point_solution = create_point_solution(&solution, orig_linear);
                    values.push(compute_scalar_field_value(&point_solution, field_enum));
                }
            }
        }
        _ => {
            return Err(format!("Invalid slice plane: {}", slicePlane));
        }
    }

    let colors = compute_colors(&values, &scheme);

    let mut mesh =
        sliced_grid.to_mesh_surface_geometry_decimated(respect_iblank.unwrap_or(false), 1);
    mesh.colors = Some(colors);

    // Filter colors to match blanked vertices in sliced grid
    if respect_iblank.unwrap_or(false) {
        if let (Some(iblank), Some(colors)) = (sliced_grid.iblank.as_ref(), mesh.colors.take()) {
            let i_slice = sliced_grid.dimensions.i as usize;
            let j_slice = sliced_grid.dimensions.j as usize;

            let mut filtered_colors = Vec::new();
            for j_idx in 0..j_slice {
                for i_idx in 0..i_slice {
                    let grid_idx = j_idx * i_slice + i_idx;
                    if iblank[grid_idx] != 0 {
                        let vertex_idx = j_idx * i_slice + i_idx;
                        let color_idx = vertex_idx * 3;
                        if color_idx + 2 < colors.len() {
                            filtered_colors.push(colors[color_idx]);
                            filtered_colors.push(colors[color_idx + 1]);
                            filtered_colors.push(colors[color_idx + 2]);
                        }
                    }
                }
            }

            mesh.colors = if filtered_colors.is_empty() {
                None
            } else {
                Some(filtered_colors)
            };
        }
    }

    let _ = window.emit("loading-end", ());

    Ok(mesh)
}

/// Helper function to create a point solution from a solution array at a given index
fn create_point_solution(solution: &Plot3DSolution, index: usize) -> Plot3DSolution {
    Plot3DSolution {
        grid_index: 0,
        dimensions: GridDimensions { i: 1, j: 1, k: 1 },
        rho: vec![solution.rho[index]],
        rhou: vec![solution.rhou[index]],
        rhov: vec![solution.rhov[index]],
        rhow: vec![solution.rhow[index]],
        rhoe: vec![solution.rhoe[index]],
        gamma: solution.gamma.as_ref().map(|g| vec![g[index]]),
        metadata: None,
    }
}

/// Compute solution colors for arbitrary plane using cached data (ID-based)
#[allow(non_snake_case)]
#[tauri::command]
fn compute_solution_colors_arbitrary_plane(
    gridId: String,
    solutionId: String,
    planePoint: [f32; 3],
    planeNormal: [f32; 3],
    field: String,
    colorScheme: String,
    respect_iblank: Option<bool>,
    window: WebviewWindow,
) -> Result<MeshGeometry, String> {
    use solution::{compute_colors, ColorScheme, ScalarField};

    let _ = window.emit(
        "loading-start",
        format!("Computing {} field on arbitrary plane...", field),
    );

    // Load grid from cache
    let (grid, grid_file_path, grid_index) = {
        let cache = GRID_CACHE
            .lock()
            .map_err(|_| "Grid cache lock poisoned".to_string())?;
        let cached = cache
            .get(&gridId)
            .ok_or_else(|| format!("Grid not found in cache: {}", gridId))?;
        (
            Arc::clone(&cached.grid),
            cached.file_path.clone(),
            cached.grid_index,
        )
    };

    // Load solution from cache
    let (solution, solution_file_path, solution_grid_index) = {
        let cache = SOLUTION_CACHE_V2
            .lock()
            .map_err(|_| "Solution cache lock poisoned".to_string())?;
        let cached = cache
            .get(&solutionId)
            .ok_or_else(|| format!("Solution not found in cache: {}", solutionId))?;
        (
            Arc::clone(&cached.solution),
            cached.file_path.clone(),
            cached.grid_index,
        )
    };

    if grid_index != solution_grid_index {
        return Err(format!(
            "Grid/solution mismatch: grid/solution index differs: grid(id={}, index={}) vs solution(id={}, index={})",
            gridId, grid_index, solutionId, solution_grid_index
        ));
    }
    if grid.dimensions.i != solution.dimensions.i
        || grid.dimensions.j != solution.dimensions.j
        || grid.dimensions.k != solution.dimensions.k
    {
        return Err(format!(
            "Grid/solution mismatch: dimensions differ: grid(id={}, dims={}x{}x{}) vs solution(id={}, dims={}x{}x{})",
            gridId,
            grid.dimensions.i,
            grid.dimensions.j,
            grid.dimensions.k,
            solutionId,
            solution.dimensions.i,
            solution.dimensions.j,
            solution.dimensions.k
        ));
    }

    if grid_file_path != solution_file_path {
        log_debug(&format!(
            "Grid/solution file paths differ but pair accepted by index+dimensions: grid(id={}, file={}) solution(id={}, file={})",
            gridId, grid_file_path, solutionId, solution_file_path
        ));
    }

    // Parse field and scheme
    let field_enum =
        ScalarField::from_str(&field).ok_or_else(|| format!("Unknown scalar field: {}", field))?;
    let scheme = ColorScheme::from_str(&colorScheme)
        .ok_or_else(|| format!("Unknown color scheme: {}", colorScheme))?;

    // Validate
    let grid_points = grid.total_points();
    if solution.rho.len() != grid_points {
        return Err(format!(
            "Solution points {} != grid points {}",
            solution.rho.len(),
            grid_points
        ));
    }

    // Slice with solution tracking
    let mut mesh = grid.slice_arbitrary_plane_with_solution(
        planePoint,
        planeNormal,
        respect_iblank.unwrap_or(false),
    )?;

    let vertex_cell_data = mesh
        .vertex_cell_data
        .as_ref()
        .ok_or_else(|| "No vertex cell data available".to_string())?;

    // Precompute scalar field at original grid nodes, then interpolate scalar values
    // to arbitrary-plane vertices using the stored corner weights.
    // This directly interpolates the selected field data onto the plane.
    let nodal_field_values: Vec<f32> = (0..grid_points)
        .map(|idx| {
            let gamma = solution.gamma.as_ref().map(|g| g[idx]);
            compute_scalar_field_from_components(
                solution.rho[idx],
                solution.rhou[idx],
                solution.rhov[idx],
                solution.rhow[idx],
                solution.rhoe[idx],
                gamma,
                field_enum,
            )
        })
        .collect();

    let i_orig = grid.dimensions.i as usize;
    let j_orig = grid.dimensions.j as usize;
    let linear_index =
        |i: usize, j: usize, k: usize| -> usize { i + j * i_orig + k * i_orig * j_orig };

    let mut values = Vec::with_capacity(vertex_cell_data.len());

    for cell_data in vertex_cell_data {
        let i = cell_data.cell_i;
        let j = cell_data.cell_j;
        let k = cell_data.cell_k;

        let corner_indices = [
            linear_index(i, j, k),
            linear_index(i + 1, j, k),
            linear_index(i + 1, j + 1, k),
            linear_index(i, j + 1, k),
            linear_index(i, j, k + 1),
            linear_index(i + 1, j, k + 1),
            linear_index(i + 1, j + 1, k + 1),
            linear_index(i, j + 1, k + 1),
        ];

        let mut interpolated_field = 0.0;

        for (idx, &corner_idx) in corner_indices.iter().enumerate() {
            let weight = cell_data.weights[idx];
            interpolated_field += weight * nodal_field_values[corner_idx];
        }

        values.push(interpolated_field);
    }

    let colors = compute_colors(&values, &scheme);
    mesh.colors = Some(colors);

    let _ = window.emit("loading-end", ());

    Ok(mesh)
}

// ============================================================================
// Cache Management Commands
// ============================================================================

/// List all cached grids with their metadata
#[tauri::command]
fn list_cached_grids() -> Result<Vec<GridMetadata>, String> {
    let cache = GRID_CACHE
        .lock()
        .map_err(|_| "Grid cache lock poisoned".to_string())?;

    let metadata: Vec<GridMetadata> = cache
        .values()
        .map(|cached| {
            let has_solution = SOLUTION_CACHE_V2
                .lock()
                .ok()
                .and_then(|sol_cache| {
                    sol_cache
                        .values()
                        .any(|s| {
                            s.file_path == cached.file_path && s.grid_index == cached.grid_index
                        })
                        .then_some(true)
                })
                .unwrap_or(false);

            GridMetadata {
                id: cached.id.clone(),
                file_path: cached.file_path.clone(),
                file_name: cached.file_name.clone(),
                grid_index: cached.grid_index,
                dimensions: cached.grid.dimensions.clone(),
                has_iblank: cached.has_iblank,
                has_solution,
            }
        })
        .collect();

    Ok(metadata)
}

/// List all cached solutions with their metadata
#[tauri::command]
fn list_cached_solutions() -> Result<Vec<SolutionMetadata>, String> {
    let cache = SOLUTION_CACHE_V2
        .lock()
        .map_err(|_| "Solution cache lock poisoned".to_string())?;

    let metadata: Vec<SolutionMetadata> = cache
        .values()
        .map(|cached| SolutionMetadata {
            id: cached.id.clone(),
            file_path: cached.file_path.clone(),
            file_name: cached.file_name.clone(),
            grid_index: cached.grid_index,
            dimensions: cached.solution.dimensions.clone(),
        })
        .collect();

    Ok(metadata)
}

/// Get metadata for a specific cached grid
#[tauri::command]
fn get_grid_metadata(grid_id: String) -> Result<GridMetadata, String> {
    let cache = GRID_CACHE
        .lock()
        .map_err(|_| "Grid cache lock poisoned".to_string())?;

    let cached = cache
        .get(&grid_id)
        .ok_or_else(|| format!("Grid not found in cache: {}", grid_id))?;

    let has_solution = SOLUTION_CACHE_V2
        .lock()
        .ok()
        .and_then(|sol_cache| {
            sol_cache
                .values()
                .any(|s| s.file_path == cached.file_path && s.grid_index == cached.grid_index)
                .then_some(true)
        })
        .unwrap_or(false);

    Ok(GridMetadata {
        id: cached.id.clone(),
        file_path: cached.file_path.clone(),
        file_name: cached.file_name.clone(),
        grid_index: cached.grid_index,
        dimensions: cached.grid.dimensions.clone(),
        has_iblank: cached.has_iblank,
        has_solution,
    })
}

/// Clear all cached grids
#[tauri::command]
fn clear_grid_cache() -> Result<(), String> {
    let mut cache = GRID_CACHE
        .lock()
        .map_err(|_| "Grid cache lock poisoned".to_string())?;

    let count = cache.len();
    cache.clear();
    log_info(&format!("Cleared {} grids from cache", count));
    Ok(())
}

/// Clear all cached solutions (v2 cache)
#[tauri::command]
fn clear_solution_cache_v2() -> Result<(), String> {
    let mut cache = SOLUTION_CACHE_V2
        .lock()
        .map_err(|_| "Solution cache lock poisoned".to_string())?;

    let count = cache.len();
    cache.clear();
    log_info(&format!("Cleared {} solutions from cache", count));
    Ok(())
}

/// Unload a specific grid from cache
#[tauri::command]
fn unload_grid(grid_id: String) -> Result<(), String> {
    let mut cache = GRID_CACHE
        .lock()
        .map_err(|_| "Grid cache lock poisoned".to_string())?;

    if cache.remove(&grid_id).is_some() {
        log_info(&format!("Unloaded grid from cache: {}", grid_id));
        Ok(())
    } else {
        Err(format!("Grid not found in cache: {}", grid_id))
    }
}

/// Unload a specific solution from cache
#[tauri::command]
fn unload_solution(solution_id: String) -> Result<(), String> {
    let mut cache = SOLUTION_CACHE_V2
        .lock()
        .map_err(|_| "Solution cache lock poisoned".to_string())?;

    if cache.remove(&solution_id).is_some() {
        log_info(&format!("Unloaded solution from cache: {}", solution_id));
        Ok(())
    } else {
        Err(format!("Solution not found in cache: {}", solution_id))
    }
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
        .set_file_name("overview-logs.txt")
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

/// Print frontend debug messages to the terminal
#[tauri::command]
fn frontend_log(message: String) {
    println!("[frontend] {}", message);
}

/// Open the About window
#[tauri::command]
async fn open_about_window(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("about") {
        let _ = window.set_focus();
        Ok(())
    } else {
        WebviewWindow::builder(&app, "about", tauri::WebviewUrl::App("/about.html".into()))
            .title("About overview")
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
            load_plot3d_file_cached,
            load_plot3d_solution,
            load_plot3d_solution_ascii,
            load_plot3d_solution_auto,
            load_plot3d_solution_cached,
            load_plot3d_function,
            convert_grid_to_mesh,
            convert_grid_to_mesh_by_id,
            slice_grid_by_id,
            slice_arbitrary_plane_by_id,
            compute_solution_colors,
            compute_solution_colors_sliced,
            compute_solution_colors_arbitrary_plane,
            list_cached_grids,
            list_cached_solutions,
            get_grid_metadata,
            clear_grid_cache,
            clear_solution_cache_v2,
            unload_grid,
            unload_solution,
            open_file_dialog,
            open_multiple_files_dialog,
            detect_file_format,
            get_log_entries,
            clear_log_entries,
            export_logs_to_file,
            save_log_file_dialog,
            write_text_file,
            frontend_log,
            open_about_window,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

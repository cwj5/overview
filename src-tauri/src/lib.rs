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
use serde::Deserialize;
use std::cell::RefCell;
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

/// Slice a PLOT3D grid along a constant I, J, or K plane
#[tauri::command]
fn slice_grid(grid: Plot3DGrid, plane: String, index: u32) -> Result<Plot3DGrid, String> {
    log_debug(&format!(
        "Slicing grid along {} plane at index {}",
        plane, index
    ));
    grid.slice_grid(&plane, index).map_err(|e| {
        let error_msg = format!("Failed to slice grid: {}", e);
        log_error(&error_msg);
        error_msg
    })
}

/// Slice a PLOT3D grid with an arbitrary cutting plane
/// plane_point: [x, y, z] coordinates of a point on the plane
/// plane_normal: [nx, ny, nz] normal vector to the plane
#[tauri::command]
fn slice_arbitrary_plane(
    grid: Plot3DGrid,
    plane_point: [f32; 3],
    plane_normal: [f32; 3],
    respect_iblank: Option<bool>,
    _window: WebviewWindow,
) -> Result<MeshGeometry, String> {
    log_debug(&format!(
        "Slicing grid with arbitrary plane: point={:?}, normal={:?}",
        plane_point, plane_normal
    ));

    let result =
        grid.slice_arbitrary_plane(plane_point, plane_normal, respect_iblank.unwrap_or(false));

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

#[derive(Deserialize)]
struct ComputeSolutionColorsArgs {
    grid: Plot3DGrid,
    solution: Plot3DSolution,
    field: String,
    #[serde(alias = "colorScheme", alias = "color_scheme")]
    color_scheme: String,
}

#[derive(Deserialize)]
struct ComputeSolutionColorsCachedArgs {
    grid: Plot3DGrid,
    #[serde(alias = "gridIndex", alias = "grid_index")]
    grid_index: usize,
    field: String,
    #[serde(alias = "colorScheme", alias = "color_scheme")]
    color_scheme: String,
}

/// Compute scalar field colors for a grid with solution data
#[tauri::command]
fn compute_solution_colors(
    grid: Plot3DGrid,
    solution: Plot3DSolution,
    field: String,
    color_scheme: String,
    respect_iblank: Option<bool>,
    window: WebviewWindow,
) -> Result<MeshGeometry, String> {
    use solution::{compute_colors, compute_scalar_field_surface, ColorScheme, ScalarField};

    // Emit loading start event
    let _ = window.emit("loading-start", format!("Computing {} field...", field));

    let args = ComputeSolutionColorsArgs {
        grid,
        solution,
        field,
        color_scheme,
    };

    // Parse field type
    let field_enum = ScalarField::from_str(&args.field)
        .ok_or_else(|| format!("Unknown scalar field: {}", args.field))?;

    // Parse color scheme
    let scheme = ColorScheme::from_str(&args.color_scheme)
        .ok_or_else(|| format!("Unknown color scheme: {}", args.color_scheme))?;

    // Validate solution matches grid
    let grid_points = args.grid.total_points();
    if args.solution.rho.len() != grid_points {
        return Err(format!(
            "Solution points {} != grid points {}",
            args.solution.rho.len(),
            grid_points
        ));
    }

    // Auto-detect decimation based on grid size for better performance
    let i = args.grid.dimensions.i as usize;
    let j = args.grid.dimensions.j as usize;
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
            "Solution grid size {}x{} - applying {}x decimation for performance",
            i, j, decimation_factor
        ));
    }

    // Compute scalar values/colors for surface only
    let values = compute_scalar_field_surface(&args.solution, field_enum, decimation_factor);
    let colors = compute_colors(&values, &scheme);

    // Create surface mesh geometry (don't respect iblank for solution visualization)
    let mut mesh = args
        .grid
        .to_mesh_surface_geometry_decimated(respect_iblank.unwrap_or(false), decimation_factor);
    mesh.colors = Some(colors);

    // Emit loading end event
    let _ = window.emit("loading-end", ());

    Ok(mesh)
}

/// Compute scalar field colors using cached solution data
#[tauri::command]
fn compute_solution_colors_cached(
    grid: Plot3DGrid,
    grid_index: usize,
    field: String,
    color_scheme: String,
    respect_iblank: Option<bool>,
    window: WebviewWindow,
) -> Result<MeshGeometry, String> {
    use solution::{compute_colors, compute_scalar_field_surface, ColorScheme, ScalarField};

    let _ = window.emit("loading-start", format!("Computing {} field...", field));

    let args = ComputeSolutionColorsCachedArgs {
        grid,
        grid_index,
        field,
        color_scheme,
    };

    let field_enum = ScalarField::from_str(&args.field)
        .ok_or_else(|| format!("Unknown scalar field: {}", args.field))?;

    let scheme = ColorScheme::from_str(&args.color_scheme)
        .ok_or_else(|| format!("Unknown color scheme: {}", args.color_scheme))?;

    let solution = {
        let store = SOLUTION_CACHE
            .lock()
            .map_err(|_| "Solution cache lock poisoned".to_string())?;
        let cached = store
            .iter()
            .find(|sol| sol.grid_index == args.grid_index)
            .ok_or_else(|| format!("No cached solution for grid index {}", args.grid_index))?;
        Arc::clone(cached)
    };

    let grid_points = args.grid.total_points();
    if solution.rho.len() != grid_points {
        return Err(format!(
            "Solution points {} != grid points {}",
            solution.rho.len(),
            grid_points
        ));
    }

    let i = args.grid.dimensions.i as usize;
    let j = args.grid.dimensions.j as usize;
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

    let values = compute_scalar_field_surface(&solution, field_enum, decimation_factor);
    let colors = compute_colors(&values, &scheme);

    let mut mesh = args
        .grid
        .to_mesh_surface_geometry_decimated(respect_iblank.unwrap_or(false), decimation_factor);
    mesh.colors = Some(colors);

    let _ = window.emit("loading-end", ());

    Ok(mesh)
}

/// Compute scalar field colors only using cached solution data
#[tauri::command]
fn compute_solution_colors_only_cached(
    grid_index: usize,
    field: String,
    color_scheme: String,
) -> Result<Vec<f32>, String> {
    use solution::{compute_colors, compute_scalar_field_surface, ColorScheme, ScalarField};

    let field_enum =
        ScalarField::from_str(&field).ok_or_else(|| format!("Unknown scalar field: {}", field))?;

    let scheme = ColorScheme::from_str(&color_scheme)
        .ok_or_else(|| format!("Unknown color scheme: {}", color_scheme))?;

    let solution = {
        let store = SOLUTION_CACHE
            .lock()
            .map_err(|_| "Solution cache lock poisoned".to_string())?;
        let cached = store
            .iter()
            .find(|sol| sol.grid_index == grid_index)
            .ok_or_else(|| format!("No cached solution for grid index {}", grid_index))?;
        Arc::clone(cached)
    };

    let i = solution.dimensions.i as usize;
    let j = solution.dimensions.j as usize;
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

    let values = compute_scalar_field_surface(&solution, field_enum, decimation_factor);
    Ok(compute_colors(&values, &scheme))
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

/// Compute scalar field colors for a sliced grid using original grid's solution data
/// Maps sliced grid points back to their original indices for solution lookup
#[tauri::command]
fn compute_solution_colors_sliced(
    sliced_grid: Plot3DGrid,
    original_grid: Plot3DGrid,
    grid_index: usize,
    field: String,
    color_scheme: String,
    slice_plane: String,
    slice_index: u32,
    respect_iblank: Option<bool>,
    window: WebviewWindow,
) -> Result<MeshGeometry, String> {
    use solution::{compute_colors, ColorScheme, ScalarField};

    log_debug(&format!(
        "compute_solution_colors_sliced called: plane={}, index={}, field={}, grid={}",
        slice_plane, slice_index, field, grid_index
    ));
    let _ = window.emit(
        "loading-start",
        format!("Computing {} field on slice...", field),
    );

    let field_enum =
        ScalarField::from_str(&field).ok_or_else(|| format!("Unknown scalar field: {}", field))?;

    let scheme = ColorScheme::from_str(&color_scheme)
        .ok_or_else(|| format!("Unknown color scheme: {}", color_scheme))?;

    // Get the cached solution for this grid
    let solution = {
        let store = SOLUTION_CACHE
            .lock()
            .map_err(|_| "Solution cache lock poisoned".to_string())?;
        let cached = store
            .iter()
            .find(|sol| sol.grid_index == grid_index)
            .ok_or_else(|| format!("No cached solution for grid index {}", grid_index))?;
        Arc::clone(cached)
    };

    // Validate that solution dimensions match original grid
    let grid_points = original_grid.total_points();
    if solution.rho.len() != grid_points {
        return Err(format!(
            "Solution points {} != original grid points {}",
            solution.rho.len(),
            grid_points
        ));
    }

    // Extract the original grid dimensions
    let i_orig = original_grid.dimensions.i as usize;
    let j_orig = original_grid.dimensions.j as usize;
    let k_orig = original_grid.dimensions.k as usize;

    // Extract the sliced grid dimensions
    let i_slice = sliced_grid.dimensions.i as usize;
    let j_slice = sliced_grid.dimensions.j as usize;
    let _k_slice = sliced_grid.dimensions.k as usize;

    let slice_idx = slice_index as usize;

    // Map each point in the sliced grid back to its original indices to get solution values
    let mut values = Vec::with_capacity(sliced_grid.total_points());

    let linear_index_original =
        |i: usize, j: usize, k: usize| -> usize { i + j * i_orig + k * i_orig * j_orig };

    match slice_plane.to_uppercase().as_str() {
        "K" => {
            // Constant K plane: sliced grid has dimensions (i_orig x j_orig x 1)
            // Each point (i,j) in sliced grid maps to (i,j,slice_idx) in original
            for j_idx in 0..j_slice {
                for i_idx in 0..i_slice {
                    let orig_i = i_idx;
                    let orig_j = j_idx;
                    let orig_k = slice_idx;

                    if orig_i >= i_orig || orig_j >= j_orig || orig_k >= k_orig {
                        return Err(format!(
                            "K-slice mapping out of bounds: ({},{},{}) not in ({},{},{})",
                            orig_i, orig_j, orig_k, i_orig, j_orig, k_orig
                        ));
                    }

                    let orig_linear = linear_index_original(orig_i, orig_j, orig_k);

                    // Extract solution values at this point
                    let rho = solution.rho[orig_linear];
                    let rhou = solution.rhou[orig_linear];
                    let rhov = solution.rhov[orig_linear];
                    let rhow = solution.rhow[orig_linear];
                    let rhoe = solution.rhoe[orig_linear];

                    // Get gamma if available
                    let gamma = solution.gamma.as_ref().map(|g| g[orig_linear]);

                    // Create a temporary solution at this point to compute scalar field
                    let point_solution = Plot3DSolution {
                        grid_index: 0,
                        dimensions: GridDimensions { i: 1, j: 1, k: 1 },
                        rho: vec![rho],
                        rhou: vec![rhou],
                        rhov: vec![rhov],
                        rhow: vec![rhow],
                        rhoe: vec![rhoe],
                        gamma: gamma.map(|g| vec![g]),
                        metadata: None,
                    };

                    // Compute scalar field value
                    let value = compute_scalar_field_value(&point_solution, field_enum);
                    values.push(value);
                }
            }
        }
        "J" => {
            // Constant J plane: sliced grid has dimensions (i_orig x k_orig x 1)
            // The sliced dimensions are remapped: i_slice=i_orig, j_slice=k_orig
            // Each point (i,k) in sliced grid maps to (i,slice_idx,k) in original
            for k_idx in 0..j_slice {
                // j_slice becomes k for the loop
                for i_idx in 0..i_slice {
                    let orig_i = i_idx;
                    let orig_j = slice_idx;
                    let orig_k = k_idx;

                    if orig_i >= i_orig || orig_j >= j_orig || orig_k >= k_orig {
                        return Err(format!(
                            "J-slice mapping out of bounds: ({},{},{}) not in ({},{},{})",
                            orig_i, orig_j, orig_k, i_orig, j_orig, k_orig
                        ));
                    }

                    let orig_linear = linear_index_original(orig_i, orig_j, orig_k);

                    let rho = solution.rho[orig_linear];
                    let rhou = solution.rhou[orig_linear];
                    let rhov = solution.rhov[orig_linear];
                    let rhow = solution.rhow[orig_linear];
                    let rhoe = solution.rhoe[orig_linear];

                    let gamma = solution.gamma.as_ref().map(|g| g[orig_linear]);

                    let point_solution = Plot3DSolution {
                        grid_index: 0,
                        dimensions: GridDimensions { i: 1, j: 1, k: 1 },
                        rho: vec![rho],
                        rhou: vec![rhou],
                        rhov: vec![rhov],
                        rhow: vec![rhow],
                        rhoe: vec![rhoe],
                        gamma: gamma.map(|g| vec![g]),
                        metadata: None,
                    };

                    let value = compute_scalar_field_value(&point_solution, field_enum);
                    values.push(value);
                }
            }
        }
        "I" => {
            // Constant I plane: sliced grid has dimensions (j_orig x k_orig x 1)
            // The sliced dimensions are remapped: i_slice=j_orig, j_slice=k_orig
            // Each point (j,k) in sliced grid maps to (slice_idx,j,k) in original
            for k_idx in 0..j_slice {
                // j_slice becomes k for the loop
                for j_idx in 0..i_slice {
                    // i_slice becomes j for the loop
                    let orig_i = slice_idx;
                    let orig_j = j_idx;
                    let orig_k = k_idx;

                    if orig_i >= i_orig || orig_j >= j_orig || orig_k >= k_orig {
                        return Err(format!(
                            "I-slice mapping out of bounds: ({},{},{}) not in ({},{},{})",
                            orig_i, orig_j, orig_k, i_orig, j_orig, k_orig
                        ));
                    }

                    let orig_linear = linear_index_original(orig_i, orig_j, orig_k);

                    let rho = solution.rho[orig_linear];
                    let rhou = solution.rhou[orig_linear];
                    let rhov = solution.rhov[orig_linear];
                    let rhow = solution.rhow[orig_linear];
                    let rhoe = solution.rhoe[orig_linear];

                    let gamma = solution.gamma.as_ref().map(|g| g[orig_linear]);

                    let point_solution = Plot3DSolution {
                        grid_index: 0,
                        dimensions: GridDimensions { i: 1, j: 1, k: 1 },
                        rho: vec![rho],
                        rhou: vec![rhou],
                        rhov: vec![rhov],
                        rhow: vec![rhow],
                        rhoe: vec![rhoe],
                        gamma: gamma.map(|g| vec![g]),
                        metadata: None,
                    };

                    let value = compute_scalar_field_value(&point_solution, field_enum);
                    values.push(value);
                }
            }
        }
        _ => {
            return Err(format!(
                "Invalid slice plane: {}. Must be 'I', 'J', or 'K'",
                slice_plane
            ))
        }
    }

    // Convert values to colors
    let colors = compute_colors(&values, &scheme);
    log_debug(&format!(
        "Computed {} colors for slice {}{}",
        colors.len() / 3,
        slice_plane,
        slice_index
    ));

    // Create mesh geometry from sliced grid
    // Use decimation_factor=1 for consistency with the sliced geometry size
    let mut mesh =
        sliced_grid.to_mesh_surface_geometry_decimated(respect_iblank.unwrap_or(false), 1);
    mesh.colors = Some(colors);

    let _ = window.emit("loading-end", ());

    Ok(mesh)
}

/// Compute scalar field colors for an arbitrary plane slice with solution data  
/// Uses vertex interpolation data to map arbitrary plane intersection points to solution values
#[tauri::command]
fn compute_solution_colors_arbitrary_plane(
    grid: Plot3DGrid,
    grid_index: usize,
    field: String,
    color_scheme: String,
    plane_point: [f32; 3],
    plane_normal: [f32; 3],
    respect_iblank: Option<bool>,
    window: WebviewWindow,
) -> Result<MeshGeometry, String> {
    use solution::{compute_colors, ColorScheme, ScalarField};

    log_debug(&format!(
        "compute_solution_colors_arbitrary_plane called: field={}, grid={}",
        field, grid_index
    ));
    let _ = window.emit(
        "loading-start",
        format!("Computing {} field on arbitrary plane...", field),
    );

    let field_enum =
        ScalarField::from_str(&field).ok_or_else(|| format!("Unknown scalar field: {}", field))?;

    let scheme = ColorScheme::from_str(&color_scheme)
        .ok_or_else(|| format!("Unknown color scheme: {}", color_scheme))?;

    // Get the cached solution for this grid
    let solution = {
        let store = SOLUTION_CACHE
            .lock()
            .map_err(|_| "Solution cache lock poisoned".to_string())?;
        let cached = store
            .iter()
            .find(|sol| sol.grid_index == grid_index)
            .ok_or_else(|| format!("No cached solution for grid index {}", grid_index))?;
        Arc::clone(cached)
    };

    // Validate that solution dimensions match grid
    let grid_points = grid.total_points();
    if solution.rho.len() != grid_points {
        return Err(format!(
            "Solution points {} != grid points {}",
            solution.rho.len(),
            grid_points
        ));
    }

    // Slice the grid with the arbitrary plane (enhanced version that tracks interpolation data)
    let mut mesh = grid.slice_arbitrary_plane_with_solution(
        plane_point,
        plane_normal,
        respect_iblank.unwrap_or(false),
    )?;

    // Get vertex cell data
    let vertex_cell_data = mesh
        .vertex_cell_data
        .as_ref()
        .ok_or_else(|| "No vertex cell data available".to_string())?;

    // Extract grid dimensions for linear indexing
    let i_orig = grid.dimensions.i as usize;
    let j_orig = grid.dimensions.j as usize;
    let linear_index =
        |i: usize, j: usize, k: usize| -> usize { i + j * i_orig + k * i_orig * j_orig };

    // Interpolate solution values for each vertex
    let mut values = Vec::with_capacity(vertex_cell_data.len());

    for cell_data in vertex_cell_data {
        // Get the 8 corner indices of the cell
        let i = cell_data.cell_i;
        let j = cell_data.cell_j;
        let k = cell_data.cell_k;

        let corner_indices = [
            linear_index(i, j, k),             // 0
            linear_index(i + 1, j, k),         // 1
            linear_index(i + 1, j + 1, k),     // 2
            linear_index(i, j + 1, k),         // 3
            linear_index(i, j, k + 1),         // 4
            linear_index(i + 1, j, k + 1),     // 5
            linear_index(i + 1, j + 1, k + 1), // 6
            linear_index(i, j + 1, k + 1),     // 7
        ];

        // Interpolate solution variables using the weights
        let mut rho = 0.0;
        let mut rhou = 0.0;
        let mut rhov = 0.0;
        let mut rhow = 0.0;
        let mut rhoe = 0.0;
        let mut gamma_val = 0.0;

        for (idx, &corner_idx) in corner_indices.iter().enumerate() {
            let weight = cell_data.weights[idx];
            rho += weight * solution.rho[corner_idx];
            rhou += weight * solution.rhou[corner_idx];
            rhov += weight * solution.rhov[corner_idx];
            rhow += weight * solution.rhow[corner_idx];
            rhoe += weight * solution.rhoe[corner_idx];
            if let Some(ref gamma_arr) = solution.gamma {
                gamma_val += weight * gamma_arr[corner_idx];
            }
        }

        // Create a temporary solution at this interpolated point
        let point_solution = Plot3DSolution {
            grid_index: 0,
            dimensions: GridDimensions { i: 1, j: 1, k: 1 },
            rho: vec![rho],
            rhou: vec![rhou],
            rhov: vec![rhov],
            rhow: vec![rhow],
            rhoe: vec![rhoe],
            gamma: if solution.gamma.is_some() {
                Some(vec![gamma_val])
            } else {
                None
            },
            metadata: None,
        };

        // Compute scalar field value
        let value = compute_scalar_field_value(&point_solution, field_enum);
        values.push(value);
    }

    // Convert values to colors
    let colors = compute_colors(&values, &scheme);
    log_debug(&format!(
        "Computed {} colors for arbitrary plane slice",
        colors.len() / 3
    ));

    // Add colors to mesh
    mesh.colors = Some(colors);

    let _ = window.emit("loading-end", ());

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
            load_plot3d_solution,
            load_plot3d_solution_ascii,
            load_plot3d_solution_auto,
            load_plot3d_function,
            convert_grid_to_mesh,
            slice_grid,
            slice_arbitrary_plane,
            compute_solution_colors,
            compute_solution_colors_cached,
            compute_solution_colors_only_cached,
            compute_solution_colors_sliced,
            compute_solution_colors_arbitrary_plane,
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

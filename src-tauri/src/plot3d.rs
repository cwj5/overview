use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{self, BufRead, BufReader, Read};
use std::path::Path;

/// Represents a PLOT3D grid structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plot3DGrid {
    pub dimensions: GridDimensions,
    pub x_coords: Vec<f32>,
    pub y_coords: Vec<f32>,
    pub z_coords: Vec<f32>,
}

/// File metadata about the loaded grid
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GridFileMetadata {
    pub byte_order: String, // "Little-Endian" or "Big-Endian"
    pub is_detected: bool,  // true if auto-detected, false if assumed
    pub num_grids: usize,
    pub grid_dimensions: Vec<GridDimensions>,
}

/// Represents PLOT3D solution data (Q file)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plot3DSolution {
    pub grid_index: usize,
    pub dimensions: GridDimensions,
    pub rho: Vec<f32>,  // Density
    pub rhou: Vec<f32>, // Momentum X
    pub rhov: Vec<f32>, // Momentum Y
    pub rhow: Vec<f32>, // Momentum Z
    pub rhoe: Vec<f32>, // Energy
}

/// PLOT3D function file data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plot3DFunction {
    pub grid_index: usize,
    pub dimensions: GridDimensions,
    pub function_data: Vec<Vec<f32>>, // Multiple functions per grid
}

/// Grid dimensions (I, J, K)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GridDimensions {
    pub i: usize,
    pub j: usize,
    pub k: usize,
}

/// Byte order detection
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ByteOrder {
    LittleEndian,
    BigEndian,
}

impl Plot3DGrid {
    /// Calculate total number of points
    pub fn total_points(&self) -> usize {
        self.dimensions.i * self.dimensions.j * self.dimensions.k
    }
}

impl Plot3DSolution {
    /// Calculate total number of points
    pub fn total_points(&self) -> usize {
        self.dimensions.i * self.dimensions.j * self.dimensions.k
    }
}

impl Plot3DFunction {
    /// Calculate total number of points
    pub fn total_points(&self) -> usize {
        self.dimensions.i * self.dimensions.j * self.dimensions.k
    }
}

/// Auto-detect byte order by reading first dimension value
fn detect_byte_order<R: Read>(reader: &mut R) -> io::Result<ByteOrder> {
    let mut buf = [0u8; 4];
    reader.read_exact(&mut buf)?;

    let le_value = i32::from_le_bytes(buf);
    let be_value = i32::from_be_bytes(buf);

    // PLOT3D dimensions are typically between 1 and 10000
    // Little-endian is more common on modern systems
    if le_value > 0 && le_value < 10000 {
        Ok(ByteOrder::LittleEndian)
    } else if be_value > 0 && be_value < 10000 {
        Ok(ByteOrder::BigEndian)
    } else {
        // Default to little-endian if ambiguous
        Ok(ByteOrder::LittleEndian)
    }
}

/// Read PLOT3D grid file (binary format)
/// PLOT3D format specification:
/// - Header: number of grids (int32)
/// - For each grid: I, J, K dimensions (3 x int32)
/// - Grid coordinates: X, Y, Z arrays (float32)
pub fn read_plot3d_grid<P: AsRef<Path>>(path: P) -> io::Result<Vec<Plot3DGrid>> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);

    // Detect byte order from first dimension
    let byte_order = detect_byte_order(&mut reader)?;

    // Read number of grids
    let num_grids = read_i32(&mut reader, byte_order)?;
    if num_grids <= 0 || num_grids > 1000 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Invalid number of grids: {}", num_grids),
        ));
    }

    let mut grids = Vec::with_capacity(num_grids as usize);

    // Read dimensions for all grids first (PLOT3D whole format)
    let mut dimensions_list = Vec::with_capacity(num_grids as usize);
    for _ in 0..num_grids {
        let i = read_i32(&mut reader, byte_order)? as usize;
        let j = read_i32(&mut reader, byte_order)? as usize;
        let k = read_i32(&mut reader, byte_order)? as usize;

        if i == 0 || j == 0 || k == 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Invalid dimensions: {}x{}x{}", i, j, k),
            ));
        }

        dimensions_list.push(GridDimensions { i, j, k });
    }

    // Read coordinate data for each grid
    for dims in dimensions_list {
        let total_points = dims.i * dims.j * dims.k;

        let x_coords = read_f32_array(&mut reader, total_points, byte_order)?;
        let y_coords = read_f32_array(&mut reader, total_points, byte_order)?;
        let z_coords = read_f32_array(&mut reader, total_points, byte_order)?;

        grids.push(Plot3DGrid {
            dimensions: dims,
            x_coords,
            y_coords,
            z_coords,
        });
    }

    Ok(grids)
}

/// Read PLOT3D grid file with metadata about byte order and dimensions
pub fn read_plot3d_grid_with_metadata<P: AsRef<Path>>(
    path: P,
) -> io::Result<(Vec<Plot3DGrid>, GridFileMetadata)> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);

    // Detect byte order from first dimension
    let byte_order = detect_byte_order(&mut reader)?;
    let byte_order_str = match byte_order {
        ByteOrder::LittleEndian => "Little-Endian",
        ByteOrder::BigEndian => "Big-Endian",
    };

    // Read number of grids
    let num_grids = read_i32(&mut reader, byte_order)?;
    if num_grids <= 0 || num_grids > 1000 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Invalid number of grids: {}", num_grids),
        ));
    }

    let mut grids = Vec::with_capacity(num_grids as usize);
    let mut grid_dimensions = Vec::with_capacity(num_grids as usize);

    // Read dimensions for all grids first (PLOT3D whole format)
    let mut dimensions_list = Vec::with_capacity(num_grids as usize);
    for _ in 0..num_grids {
        let i = read_i32(&mut reader, byte_order)? as usize;
        let j = read_i32(&mut reader, byte_order)? as usize;
        let k = read_i32(&mut reader, byte_order)? as usize;

        if i == 0 || j == 0 || k == 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Invalid dimensions: {}x{}x{}", i, j, k),
            ));
        }

        let dims = GridDimensions { i, j, k };
        grid_dimensions.push(dims.clone());
        dimensions_list.push(dims);
    }

    // Read coordinate data for each grid
    for dims in dimensions_list {
        let total_points = dims.i * dims.j * dims.k;

        let x_coords = read_f32_array(&mut reader, total_points, byte_order)?;
        let y_coords = read_f32_array(&mut reader, total_points, byte_order)?;
        let z_coords = read_f32_array(&mut reader, total_points, byte_order)?;

        grids.push(Plot3DGrid {
            dimensions: dims,
            x_coords,
            y_coords,
            z_coords,
        });
    }

    let metadata = GridFileMetadata {
        byte_order: byte_order_str.to_string(),
        is_detected: true,
        num_grids: num_grids as usize,
        grid_dimensions,
    };

    Ok((grids, metadata))
}

/// Read PLOT3D grid file in ASCII format
pub fn read_plot3d_grid_ascii<P: AsRef<Path>>(path: P) -> io::Result<Vec<Plot3DGrid>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let mut lines = reader.lines();

    // Read number of grids
    let first_line = lines
        .next()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Empty file"))??;
    let num_grids: i32 = first_line
        .trim()
        .parse()
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Cannot parse number of grids"))?;

    if num_grids <= 0 || num_grids > 1000 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Invalid number of grids: {}", num_grids),
        ));
    }

    let mut grids = Vec::with_capacity(num_grids as usize);

    // Read dimensions for all grids
    let mut dimensions_list = Vec::with_capacity(num_grids as usize);
    for _ in 0..num_grids {
        let dims_line = lines.next().ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidData, "Missing dimension line")
        })??;
        let dims: Vec<usize> = dims_line
            .split_whitespace()
            .map(|s| s.parse::<usize>())
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Cannot parse dimensions"))?;

        if dims.len() != 3 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Expected 3 dimensions (I, J, K)",
            ));
        }

        if dims[0] == 0 || dims[1] == 0 || dims[2] == 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Invalid dimensions: {}x{}x{}", dims[0], dims[1], dims[2]),
            ));
        }

        dimensions_list.push(GridDimensions {
            i: dims[0],
            j: dims[1],
            k: dims[2],
        });
    }

    // Read coordinate data for each grid
    for dims in dimensions_list {
        let total_points = dims.i * dims.j * dims.k;
        let mut x_coords = Vec::with_capacity(total_points);
        let mut y_coords = Vec::with_capacity(total_points);
        let mut z_coords = Vec::with_capacity(total_points);

        // Read coordinates (typically one per line or multiple per line)
        let mut values_read = 0;
        let mut current_array = 0; // 0 = x, 1 = y, 2 = z

        for line in lines.by_ref() {
            let line = line?;
            let values: Vec<f32> = line
                .split_whitespace()
                .map(|s| s.parse::<f32>())
                .collect::<Result<Vec<_>, _>>()
                .map_err(|_| {
                    io::Error::new(io::ErrorKind::InvalidData, "Cannot parse coordinate value")
                })?;

            for value in values {
                match current_array {
                    0 => x_coords.push(value),
                    1 => y_coords.push(value),
                    2 => z_coords.push(value),
                    _ => unreachable!(),
                }
                values_read += 1;

                if values_read == total_points {
                    current_array += 1;
                    values_read = 0;
                    if current_array == 3 {
                        break;
                    }
                }
            }

            if current_array == 3 {
                break;
            }
        }

        if x_coords.len() != total_points
            || y_coords.len() != total_points
            || z_coords.len() != total_points
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "Incomplete coordinate data: expected {}, got {}/{}/{} values",
                    total_points,
                    x_coords.len(),
                    y_coords.len(),
                    z_coords.len()
                ),
            ));
        }

        grids.push(Plot3DGrid {
            dimensions: dims,
            x_coords,
            y_coords,
            z_coords,
        });
    }

    Ok(grids)
}

/// Read PLOT3D solution file (Q file) in binary format
pub fn read_plot3d_solution<P: AsRef<Path>>(path: P) -> io::Result<Vec<Plot3DSolution>> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);

    // Detect byte order
    let byte_order = detect_byte_order(&mut reader)?;

    // Read number of grids
    let num_grids = read_i32(&mut reader, byte_order)?;
    if num_grids <= 0 || num_grids > 1000 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Invalid number of grids: {}", num_grids),
        ));
    }

    let mut solutions = Vec::with_capacity(num_grids as usize);

    // Read dimensions for all grids
    let mut dimensions_list = Vec::with_capacity(num_grids as usize);
    for _ in 0..num_grids {
        let i = read_i32(&mut reader, byte_order)? as usize;
        let j = read_i32(&mut reader, byte_order)? as usize;
        let k = read_i32(&mut reader, byte_order)? as usize;

        if i == 0 || j == 0 || k == 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Invalid dimensions: {}x{}x{}", i, j, k),
            ));
        }

        dimensions_list.push(GridDimensions { i, j, k });
    }

    // Read solution data for each grid (5 variables: rho, rhou, rhov, rhow, rhoe)
    for (grid_index, dims) in dimensions_list.into_iter().enumerate() {
        let total_points = dims.i * dims.j * dims.k;

        let rho = read_f32_array(&mut reader, total_points, byte_order)?;
        let rhou = read_f32_array(&mut reader, total_points, byte_order)?;
        let rhov = read_f32_array(&mut reader, total_points, byte_order)?;
        let rhow = read_f32_array(&mut reader, total_points, byte_order)?;
        let rhoe = read_f32_array(&mut reader, total_points, byte_order)?;

        solutions.push(Plot3DSolution {
            grid_index,
            dimensions: dims,
            rho,
            rhou,
            rhov,
            rhow,
            rhoe,
        });
    }

    Ok(solutions)
}

/// Read PLOT3D solution file in ASCII format
pub fn read_plot3d_solution_ascii<P: AsRef<Path>>(path: P) -> io::Result<Vec<Plot3DSolution>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let mut lines = reader.lines();

    // Read number of grids
    let first_line = lines
        .next()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Empty file"))??;
    let num_grids: i32 = first_line
        .trim()
        .parse()
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Cannot parse number of grids"))?;

    if num_grids <= 0 || num_grids > 1000 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Invalid number of grids: {}", num_grids),
        ));
    }

    let mut solutions = Vec::with_capacity(num_grids as usize);

    // Read dimensions for all grids
    let mut dimensions_list = Vec::with_capacity(num_grids as usize);
    for _ in 0..num_grids {
        let dims_line = lines.next().ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidData, "Missing dimension line")
        })??;
        let dims: Vec<usize> = dims_line
            .split_whitespace()
            .map(|s| s.parse::<usize>())
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Cannot parse dimensions"))?;

        if dims.len() != 3 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Expected 3 dimensions (I, J, K)",
            ));
        }

        dimensions_list.push(GridDimensions {
            i: dims[0],
            j: dims[1],
            k: dims[2],
        });
    }

    // Read solution data for each grid (5 variables: rho, rhou, rhov, rhow, rhoe)
    for (grid_index, dims) in dimensions_list.into_iter().enumerate() {
        let total_points = dims.i * dims.j * dims.k;
        let mut rho = Vec::with_capacity(total_points);
        let mut rhou = Vec::with_capacity(total_points);
        let mut rhov = Vec::with_capacity(total_points);
        let mut rhow = Vec::with_capacity(total_points);
        let mut rhoe = Vec::with_capacity(total_points);

        // Read 5 arrays of solution variables
        let mut vars_read = 0;
        let mut values_read = 0;

        for line in lines.by_ref() {
            let line = line?;
            let values: Vec<f32> = line
                .split_whitespace()
                .map(|s| s.parse::<f32>())
                .collect::<Result<Vec<_>, _>>()
                .map_err(|_| {
                    io::Error::new(io::ErrorKind::InvalidData, "Cannot parse solution value")
                })?;

            for value in values {
                match vars_read {
                    0 => rho.push(value),
                    1 => rhou.push(value),
                    2 => rhov.push(value),
                    3 => rhow.push(value),
                    4 => rhoe.push(value),
                    _ => unreachable!(),
                }
                values_read += 1;

                if values_read == total_points {
                    vars_read += 1;
                    values_read = 0;
                    if vars_read == 5 {
                        break;
                    }
                }
            }

            if vars_read == 5 {
                break;
            }
        }

        if rho.len() != total_points
            || rhou.len() != total_points
            || rhov.len() != total_points
            || rhow.len() != total_points
            || rhoe.len() != total_points
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Incomplete solution data for grid {}", grid_index),
            ));
        }

        solutions.push(Plot3DSolution {
            grid_index,
            dimensions: dims,
            rho,
            rhou,
            rhov,
            rhow,
            rhoe,
        });
    }

    Ok(solutions)
}

/// Read PLOT3D function file (F file) in binary format
pub fn read_plot3d_function<P: AsRef<Path>>(path: P) -> io::Result<Vec<Plot3DFunction>> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);

    // Detect byte order
    let byte_order = detect_byte_order(&mut reader)?;

    // Read number of grids
    let num_grids = read_i32(&mut reader, byte_order)?;
    if num_grids <= 0 || num_grids > 1000 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Invalid number of grids: {}", num_grids),
        ));
    }

    let mut functions = Vec::with_capacity(num_grids as usize);

    // Read dimensions for all grids
    let mut dimensions_list = Vec::with_capacity(num_grids as usize);
    for _ in 0..num_grids {
        let i = read_i32(&mut reader, byte_order)? as usize;
        let j = read_i32(&mut reader, byte_order)? as usize;
        let k = read_i32(&mut reader, byte_order)? as usize;

        if i == 0 || j == 0 || k == 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Invalid dimensions: {}x{}x{}", i, j, k),
            ));
        }

        dimensions_list.push(GridDimensions { i, j, k });
    }

    // Read function data for each grid
    for (grid_index, dims) in dimensions_list.into_iter().enumerate() {
        let total_points = dims.i * dims.j * dims.k;

        // Read number of functions
        let num_functions = read_i32(&mut reader, byte_order)? as usize;
        let mut function_data = Vec::with_capacity(num_functions);

        for _ in 0..num_functions {
            let func_array = read_f32_array(&mut reader, total_points, byte_order)?;
            function_data.push(func_array);
        }

        functions.push(Plot3DFunction {
            grid_index,
            dimensions: dims,
            function_data,
        });
    }

    Ok(functions)
}

// Helper functions for binary reading
fn read_i32<R: Read>(reader: &mut R, byte_order: ByteOrder) -> io::Result<i32> {
    let mut buf = [0u8; 4];
    reader.read_exact(&mut buf)?;
    Ok(match byte_order {
        ByteOrder::LittleEndian => i32::from_le_bytes(buf),
        ByteOrder::BigEndian => i32::from_be_bytes(buf),
    })
}

fn read_f32_array<R: Read>(
    reader: &mut R,
    count: usize,
    byte_order: ByteOrder,
) -> io::Result<Vec<f32>> {
    let mut result = Vec::with_capacity(count);
    for _ in 0..count {
        let mut buf = [0u8; 4];
        reader.read_exact(&mut buf)?;
        let value = match byte_order {
            ByteOrder::LittleEndian => f32::from_le_bytes(buf),
            ByteOrder::BigEndian => f32::from_be_bytes(buf),
        };
        result.push(value);
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grid_dimensions() {
        let dims = GridDimensions {
            i: 10,
            j: 20,
            k: 30,
        };
        let grid = Plot3DGrid {
            dimensions: dims,
            x_coords: vec![],
            y_coords: vec![],
            z_coords: vec![],
        };
        assert_eq!(grid.total_points(), 6000);
    }
}

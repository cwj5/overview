use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{self, Read, BufReader};
use std::path::Path;

/// Represents a PLOT3D grid structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plot3DGrid {
    pub dimensions: GridDimensions,
    pub x_coords: Vec<f32>,
    pub y_coords: Vec<f32>,
    pub z_coords: Vec<f32>,
}

/// Grid dimensions (I, J, K)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GridDimensions {
    pub i: usize,
    pub j: usize,
    pub k: usize,
}

impl Plot3DGrid {
    /// Calculate total number of points
    pub fn total_points(&self) -> usize {
        self.dimensions.i * self.dimensions.j * self.dimensions.k
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
    
    // Read number of grids
    let num_grids = read_i32(&mut reader)?;
    let mut grids = Vec::with_capacity(num_grids as usize);
    
    // Read dimensions for all grids first (PLOT3D format)
    let mut dimensions_list = Vec::with_capacity(num_grids as usize);
    for _ in 0..num_grids {
        let i = read_i32(&mut reader)? as usize;
        let j = read_i32(&mut reader)? as usize;
        let k = read_i32(&mut reader)? as usize;
        dimensions_list.push(GridDimensions { i, j, k });
    }
    
    // Read coordinate data for each grid
    for dims in dimensions_list {
        let total_points = dims.i * dims.j * dims.k;
        
        let x_coords = read_f32_array(&mut reader, total_points)?;
        let y_coords = read_f32_array(&mut reader, total_points)?;
        let z_coords = read_f32_array(&mut reader, total_points)?;
        
        grids.push(Plot3DGrid {
            dimensions: dims,
            x_coords,
            y_coords,
            z_coords,
        });
    }
    
    Ok(grids)
}

// Helper functions for binary reading
fn read_i32<R: Read>(reader: &mut R) -> io::Result<i32> {
    let mut buf = [0u8; 4];
    reader.read_exact(&mut buf)?;
    Ok(i32::from_le_bytes(buf))
}

fn read_f32_array<R: Read>(reader: &mut R, count: usize) -> io::Result<Vec<f32>> {
    let mut result = Vec::with_capacity(count);
    for _ in 0..count {
        let mut buf = [0u8; 4];
        reader.read_exact(&mut buf)?;
        result.push(f32::from_le_bytes(buf));
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_grid_dimensions() {
        let dims = GridDimensions { i: 10, j: 20, k: 30 };
        let grid = Plot3DGrid {
            dimensions: dims,
            x_coords: vec![],
            y_coords: vec![],
            z_coords: vec![],
        };
        assert_eq!(grid.total_points(), 6000);
    }
}

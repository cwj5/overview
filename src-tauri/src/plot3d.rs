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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub iblank: Option<Vec<i32>>, // Blanking array (0=blanked, 1=visible)
}

/// File metadata about the loaded grid
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GridFileMetadata {
    pub byte_order: String, // "Little-Endian" or "Big-Endian"
    pub is_detected: bool,  // true if auto-detected, false if assumed
    pub precision: String,  // "f32", "f64", or "mixed"
    pub has_iblank: bool,
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
    pub i: u32,
    pub j: u32,
    pub k: u32,
}

/// Mesh geometry suitable for Three.js rendering
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeshGeometry {
    pub vertices: Vec<f32>, // Flat array of x, y, z coordinates
    pub indices: Vec<u32>,  // Triangle indices
    pub normals: Vec<f32>,  // Computed vertex normals
    pub vertex_count: usize,
    pub face_count: usize,
}

/// Byte order detection
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ByteOrder {
    LittleEndian,
    BigEndian,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Precision {
    F32,
    F64,
    Mixed,
}

impl Precision {
    pub fn as_str(&self) -> &'static str {
        match self {
            Precision::F32 => "f32",
            Precision::F64 => "f64",
            Precision::Mixed => "mixed",
        }
    }
}

impl Plot3DGrid {
    /// Calculate total number of points
    pub fn total_points(&self) -> usize {
        (self.dimensions.i as usize) * (self.dimensions.j as usize) * (self.dimensions.k as usize)
    }

    /// Convert PLOT3D grid to Three.js mesh geometry
    /// This creates a wireframe surface by connecting grid points
    pub fn to_mesh_geometry(&self) -> MeshGeometry {
        let i = self.dimensions.i as usize;
        let j = self.dimensions.j as usize;
        let k = self.dimensions.k as usize;
        let total_points = self.total_points();

        // Convert coordinates to vertex array (x, y, z interleaved)
        let mut vertices = Vec::with_capacity(total_points * 3);
        for idx in 0..total_points {
            vertices.push(self.x_coords[idx]);
            vertices.push(self.y_coords[idx]);
            vertices.push(self.z_coords[idx]);
        }

        // Generate indices for surface triangulation
        // For a structured grid, we create quads and triangulate them
        let mut indices = Vec::new();

        // Triangulate I-J planes (constant K surfaces)
        for k_idx in 0..k {
            for j_idx in 0..j - 1 {
                for i_idx in 0..i - 1 {
                    let idx00 = Self::linear_index(i_idx, j_idx, k_idx, i, j);
                    let idx10 = Self::linear_index(i_idx + 1, j_idx, k_idx, i, j);
                    let idx01 = Self::linear_index(i_idx, j_idx + 1, k_idx, i, j);
                    let idx11 = Self::linear_index(i_idx + 1, j_idx + 1, k_idx, i, j);

                    // First triangle of quad
                    indices.push(idx00 as u32);
                    indices.push(idx10 as u32);
                    indices.push(idx01 as u32);

                    // Second triangle of quad
                    indices.push(idx10 as u32);
                    indices.push(idx11 as u32);
                    indices.push(idx01 as u32);
                }
            }
        }

        // Triangulate I-K planes (constant J surfaces)
        if k > 1 {
            for j_idx in 0..j {
                for k_idx in 0..k - 1 {
                    for i_idx in 0..i - 1 {
                        let idx00 = Self::linear_index(i_idx, j_idx, k_idx, i, j);
                        let idx10 = Self::linear_index(i_idx + 1, j_idx, k_idx, i, j);
                        let idx01 = Self::linear_index(i_idx, j_idx, k_idx + 1, i, j);
                        let idx11 = Self::linear_index(i_idx + 1, j_idx, k_idx + 1, i, j);

                        indices.push(idx00 as u32);
                        indices.push(idx10 as u32);
                        indices.push(idx01 as u32);

                        indices.push(idx10 as u32);
                        indices.push(idx11 as u32);
                        indices.push(idx01 as u32);
                    }
                }
            }
        }

        // Triangulate J-K planes (constant I surfaces)
        if j > 1 && k > 1 {
            for i_idx in 0..i {
                for k_idx in 0..k - 1 {
                    for j_idx in 0..j - 1 {
                        let idx00 = Self::linear_index(i_idx, j_idx, k_idx, i, j);
                        let idx10 = Self::linear_index(i_idx, j_idx + 1, k_idx, i, j);
                        let idx01 = Self::linear_index(i_idx, j_idx, k_idx + 1, i, j);
                        let idx11 = Self::linear_index(i_idx, j_idx + 1, k_idx + 1, i, j);

                        indices.push(idx00 as u32);
                        indices.push(idx10 as u32);
                        indices.push(idx01 as u32);

                        indices.push(idx10 as u32);
                        indices.push(idx11 as u32);
                        indices.push(idx01 as u32);
                    }
                }
            }
        }

        // Compute vertex normals
        let mut normals = vec![0.0f32; total_points * 3];
        let face_count = indices.len() / 3;

        for face_idx in 0..face_count {
            let i0 = indices[face_idx * 3] as usize;
            let i1 = indices[face_idx * 3 + 1] as usize;
            let i2 = indices[face_idx * 3 + 2] as usize;

            let v0 = [vertices[i0 * 3], vertices[i0 * 3 + 1], vertices[i0 * 3 + 2]];
            let v1 = [vertices[i1 * 3], vertices[i1 * 3 + 1], vertices[i1 * 3 + 2]];
            let v2 = [vertices[i2 * 3], vertices[i2 * 3 + 1], vertices[i2 * 3 + 2]];

            // Compute face normal using cross product
            let edge1 = [v1[0] - v0[0], v1[1] - v0[1], v1[2] - v0[2]];
            let edge2 = [v2[0] - v0[0], v2[1] - v0[1], v2[2] - v0[2]];

            let normal = [
                edge1[1] * edge2[2] - edge1[2] * edge2[1],
                edge1[2] * edge2[0] - edge1[0] * edge2[2],
                edge1[0] * edge2[1] - edge1[1] * edge2[0],
            ];

            // Add to vertex normals
            normals[i0 * 3] += normal[0];
            normals[i0 * 3 + 1] += normal[1];
            normals[i0 * 3 + 2] += normal[2];

            normals[i1 * 3] += normal[0];
            normals[i1 * 3 + 1] += normal[1];
            normals[i1 * 3 + 2] += normal[2];

            normals[i2 * 3] += normal[0];
            normals[i2 * 3 + 1] += normal[1];
            normals[i2 * 3 + 2] += normal[2];
        }

        // Normalize normals
        for i in (0..normals.len()).step_by(3) {
            let len = (normals[i] * normals[i]
                + normals[i + 1] * normals[i + 1]
                + normals[i + 2] * normals[i + 2])
                .sqrt();
            if len > 0.0 {
                normals[i] /= len;
                normals[i + 1] /= len;
                normals[i + 2] /= len;
            }
        }

        MeshGeometry {
            vertices,
            indices,
            normals,
            vertex_count: total_points,
            face_count,
        }
    }

    /// Helper function to convert 3D grid index to linear index
    fn linear_index(i: usize, j: usize, k: usize, dim_i: usize, dim_j: usize) -> usize {
        k * dim_i * dim_j + j * dim_i + i
    }
}

impl Plot3DSolution {
    /// Calculate total number of points
    #[allow(dead_code)]
    pub fn total_points(&self) -> usize {
        (self.dimensions.i as usize) * (self.dimensions.j as usize) * (self.dimensions.k as usize)
    }
}

impl Plot3DFunction {
    /// Calculate total number of points
    #[allow(dead_code)]
    pub fn total_points(&self) -> usize {
        (self.dimensions.i as usize) * (self.dimensions.j as usize) * (self.dimensions.k as usize)
    }
}

/// Auto-detect byte order by reading first dimension value
#[allow(dead_code)]
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
/// PLOT3D format specification (Fortran unformatted):
/// - Record 1: number of grids (int32) - surrounded by record markers
/// - Record 2: For each grid: I, J, K dimensions (3 x int32 x num_grids) - surrounded by record markers  
/// - Records 3+: Grid coordinates: X, Y, Z arrays (float32) - each array in its own record with markers
#[allow(dead_code)]
pub fn read_plot3d_grid<P: AsRef<Path>>(path: P) -> io::Result<Vec<Plot3DGrid>> {
    let path_ref = path.as_ref();
    let file = File::open(path_ref)?;
    let mut reader = BufReader::new(file);

    // Skip opening record marker for number of grids
    skip_record_marker(&mut reader)?;

    // Read number of grids
    let num_grids = read_i32(&mut reader, ByteOrder::LittleEndian)?; // Try little-endian first

    // Skip closing record marker
    skip_record_marker(&mut reader)?;

    let byte_order = if num_grids > 0 && num_grids < 1000 {
        ByteOrder::LittleEndian
    } else {
        // Try big-endian
        let file = File::open(path_ref)?;
        let mut reader = BufReader::new(file);
        skip_record_marker(&mut reader)?;
        let num_grids_be = read_i32(&mut reader, ByteOrder::BigEndian)?;
        skip_record_marker(&mut reader)?;

        if num_grids_be > 0 && num_grids_be < 1000 {
            ByteOrder::BigEndian
        } else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "Invalid number of grids: {} (LE) or {} (BE)",
                    num_grids, num_grids_be
                ),
            ));
        }
    };

    // Re-read from start with correct byte order
    let file = File::open(path_ref)?;
    let mut reader = BufReader::new(file);

    skip_record_marker(&mut reader)?;
    let num_grids = read_i32(&mut reader, byte_order)?;
    skip_record_marker(&mut reader)?;

    if num_grids <= 0 || num_grids > 1000 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Invalid number of grids: {}", num_grids),
        ));
    }

    let mut grids = Vec::with_capacity(num_grids as usize);

    // Read dimensions for all grids (in one record with markers)
    skip_record_marker(&mut reader)?;
    let mut dimensions_list = Vec::with_capacity(num_grids as usize);
    for _ in 0..num_grids {
        let i = read_i32(&mut reader, byte_order)? as u32;
        let j = read_i32(&mut reader, byte_order)? as u32;
        let k = read_i32(&mut reader, byte_order)? as u32;

        if i == 0 || j == 0 || k == 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Invalid dimensions: {}x{}x{}", i, j, k),
            ));
        }

        dimensions_list.push(GridDimensions { i, j, k });
    }
    skip_record_marker(&mut reader)?;

    // Read coordinate data for each grid
    for dims in dimensions_list {
        let total_points = (dims.i as usize) * (dims.j as usize) * (dims.k as usize);

        let (x_coords, y_coords, z_coords, _precision) =
            read_xyz_coords_with_markers(&mut reader, total_points, byte_order)?;

        // Try to read iblank array if present
        let iblank = try_read_iblank_array(&mut reader, total_points, byte_order)?;

        grids.push(Plot3DGrid {
            dimensions: dims,
            x_coords,
            y_coords,
            z_coords,
            iblank,
        });
    }

    Ok(grids)
}

/// Read PLOT3D grid file with metadata about byte order and dimensions
pub fn read_plot3d_grid_with_metadata<P: AsRef<Path>>(
    path: P,
) -> io::Result<(Vec<Plot3DGrid>, GridFileMetadata)> {
    let file = File::open(&path)?;
    let mut reader = BufReader::new(file);

    // Skip opening record marker for number of grids
    skip_record_marker(&mut reader)?;

    // Try reading number of grids with little-endian
    let num_grids_le = read_i32(&mut reader, ByteOrder::LittleEndian)?;

    // Determine byte order based on validity of num_grids
    let byte_order = if num_grids_le > 0 && num_grids_le < 1000 {
        ByteOrder::LittleEndian
    } else {
        // Try big-endian - need to re-read the file
        let file = File::open(&path)?;
        let mut reader = BufReader::new(file);
        skip_record_marker(&mut reader)?;
        let num_grids_be = read_i32(&mut reader, ByteOrder::BigEndian)?;

        if num_grids_be > 0 && num_grids_be < 1000 {
            ByteOrder::BigEndian
        } else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "Invalid number of grids: {} (LE) or {} (BE)",
                    num_grids_le, num_grids_be
                ),
            ));
        }
    };

    let byte_order_str = match byte_order {
        ByteOrder::LittleEndian => "Little-Endian",
        ByteOrder::BigEndian => "Big-Endian",
    };

    // Re-read from start with correct byte order
    let file = File::open(&path)?;
    let mut reader = BufReader::new(file);

    skip_record_marker(&mut reader)?;
    let num_grids = read_i32(&mut reader, byte_order)?;
    skip_record_marker(&mut reader)?;

    if num_grids <= 0 || num_grids > 1000 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Invalid number of grids: {}", num_grids),
        ));
    }

    let mut grids = Vec::with_capacity(num_grids as usize);
    let mut grid_dimensions = Vec::with_capacity(num_grids as usize);
    let mut precision: Option<Precision> = None;
    let mut has_iblank = false;

    // Read dimensions for all grids (in one record with markers)
    skip_record_marker(&mut reader)?;
    let mut dimensions_list = Vec::with_capacity(num_grids as usize);
    for _ in 0..num_grids {
        let i = read_i32(&mut reader, byte_order)? as u32;
        let j = read_i32(&mut reader, byte_order)? as u32;
        let k = read_i32(&mut reader, byte_order)? as u32;

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
    skip_record_marker(&mut reader)?;

    // Read coordinate data for each grid
    for dims in dimensions_list {
        let total_points = (dims.i as usize) * (dims.j as usize) * (dims.k as usize);

        let (x_coords, y_coords, z_coords, grid_precision) =
            read_xyz_coords_with_markers(&mut reader, total_points, byte_order)?;

        precision = Some(match precision {
            None => grid_precision,
            Some(existing) if existing == grid_precision => existing,
            Some(_) => Precision::Mixed,
        });

        // Try to read iblank array if present
        let iblank = try_read_iblank_array(&mut reader, total_points, byte_order)?;
        if iblank.is_some() {
            has_iblank = true;
        }

        grids.push(Plot3DGrid {
            dimensions: dims,
            x_coords,
            y_coords,
            z_coords,
            iblank,
        });
    }

    let metadata = GridFileMetadata {
        byte_order: byte_order_str.to_string(),
        is_detected: true,
        precision: precision.unwrap_or(Precision::Mixed).as_str().to_string(),
        has_iblank,
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
        let dims: Vec<u32> = dims_line
            .split_whitespace()
            .map(|s| s.parse::<u32>())
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
        let total_points = (dims.i as usize) * (dims.j as usize) * (dims.k as usize);
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
            iblank: None, // ASCII format typically doesn't include iblank
        });
    }

    Ok(grids)
}

/// Read PLOT3D solution file (Q file) in binary format
pub fn read_plot3d_solution<P: AsRef<Path>>(path: P) -> io::Result<Vec<Plot3DSolution>> {
    let path_ref = path.as_ref();
    let file = File::open(path_ref)?;
    let mut reader = BufReader::new(file);

    // Try little-endian first
    skip_record_marker(&mut reader)?;
    let num_grids_le = read_i32(&mut reader, ByteOrder::LittleEndian)?;
    skip_record_marker(&mut reader)?;

    let byte_order = if num_grids_le > 0 && num_grids_le < 1000 {
        ByteOrder::LittleEndian
    } else {
        // Try big-endian
        let file = File::open(path_ref)?;
        let mut reader = BufReader::new(file);
        skip_record_marker(&mut reader)?;
        let num_grids_be = read_i32(&mut reader, ByteOrder::BigEndian)?;
        skip_record_marker(&mut reader)?;

        if num_grids_be > 0 && num_grids_be < 1000 {
            ByteOrder::BigEndian
        } else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "Invalid number of grids: {} (LE) or {} (BE)",
                    num_grids_le, num_grids_be
                ),
            ));
        }
    };

    // Re-read from start with correct byte order
    let file = File::open(path_ref)?;
    let mut reader = BufReader::new(file);

    skip_record_marker(&mut reader)?;
    let num_grids = read_i32(&mut reader, byte_order)?;
    skip_record_marker(&mut reader)?;

    let mut solutions = Vec::with_capacity(num_grids as usize);

    // Skip opening record marker for dimensions
    skip_record_marker(&mut reader)?;

    // Read dimensions for all grids
    let mut dimensions_list = Vec::with_capacity(num_grids as usize);
    for _ in 0..num_grids {
        let i = read_i32(&mut reader, byte_order)? as u32;
        let j = read_i32(&mut reader, byte_order)? as u32;
        let k = read_i32(&mut reader, byte_order)? as u32;

        if i == 0 || j == 0 || k == 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Invalid dimensions: {}x{}x{}", i, j, k),
            ));
        }

        dimensions_list.push(GridDimensions { i, j, k });
    }

    // Skip closing record marker for dimensions
    skip_record_marker(&mut reader)?;

    // Read solution data for each grid (5 variables: rho, rhou, rhov, rhow, rhoe)
    for (grid_index, dims) in dimensions_list.into_iter().enumerate() {
        let total_points = (dims.i as usize) * (dims.j as usize) * (dims.k as usize);

        // Read each variable array with its record markers
        skip_record_marker(&mut reader)?;
        let rho = read_f32_array(&mut reader, total_points, byte_order)?;
        skip_record_marker(&mut reader)?;

        skip_record_marker(&mut reader)?;
        let rhou = read_f32_array(&mut reader, total_points, byte_order)?;
        skip_record_marker(&mut reader)?;

        skip_record_marker(&mut reader)?;
        let rhov = read_f32_array(&mut reader, total_points, byte_order)?;
        skip_record_marker(&mut reader)?;

        skip_record_marker(&mut reader)?;
        let rhow = read_f32_array(&mut reader, total_points, byte_order)?;
        skip_record_marker(&mut reader)?;

        skip_record_marker(&mut reader)?;
        let rhoe = read_f32_array(&mut reader, total_points, byte_order)?;
        skip_record_marker(&mut reader)?;

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
        let dims: Vec<u32> = dims_line
            .split_whitespace()
            .map(|s| s.parse::<u32>())
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
        let total_points = (dims.i as usize) * (dims.j as usize) * (dims.k as usize);
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
    let path_ref = path.as_ref();
    let file = File::open(path_ref)?;
    let mut reader = BufReader::new(file);

    // Try little-endian first
    skip_record_marker(&mut reader)?;
    let num_grids_le = read_i32(&mut reader, ByteOrder::LittleEndian)?;
    skip_record_marker(&mut reader)?;

    let byte_order = if num_grids_le > 0 && num_grids_le < 1000 {
        ByteOrder::LittleEndian
    } else {
        // Try big-endian
        let file = File::open(path_ref)?;
        let mut reader = BufReader::new(file);
        skip_record_marker(&mut reader)?;
        let num_grids_be = read_i32(&mut reader, ByteOrder::BigEndian)?;
        skip_record_marker(&mut reader)?;

        if num_grids_be > 0 && num_grids_be < 1000 {
            ByteOrder::BigEndian
        } else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "Invalid number of grids: {} (LE) or {} (BE)",
                    num_grids_le, num_grids_be
                ),
            ));
        }
    };

    // Re-read from start with correct byte order
    let file = File::open(path_ref)?;
    let mut reader = BufReader::new(file);

    skip_record_marker(&mut reader)?;
    let num_grids = read_i32(&mut reader, byte_order)?;
    skip_record_marker(&mut reader)?;

    let mut functions = Vec::with_capacity(num_grids as usize);

    // Skip opening record marker for dimensions
    skip_record_marker(&mut reader)?;

    // Read dimensions for all grids
    let mut dimensions_list = Vec::with_capacity(num_grids as usize);
    for _ in 0..num_grids {
        let i = read_i32(&mut reader, byte_order)? as u32;
        let j = read_i32(&mut reader, byte_order)? as u32;
        let k = read_i32(&mut reader, byte_order)? as u32;

        if i == 0 || j == 0 || k == 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Invalid dimensions: {}x{}x{}", i, j, k),
            ));
        }

        dimensions_list.push(GridDimensions { i, j, k });
    }

    // Skip closing record marker for dimensions
    skip_record_marker(&mut reader)?;

    // Read function data for each grid
    for (grid_index, dims) in dimensions_list.into_iter().enumerate() {
        let total_points = (dims.i as usize) * (dims.j as usize) * (dims.k as usize);

        // Skip opening record marker and read number of functions
        skip_record_marker(&mut reader)?;
        let num_functions = read_i32(&mut reader, byte_order)? as usize;
        skip_record_marker(&mut reader)?;

        let mut function_data = Vec::with_capacity(num_functions);

        for _ in 0..num_functions {
            skip_record_marker(&mut reader)?;
            let func_array = read_f32_array(&mut reader, total_points, byte_order)?;
            skip_record_marker(&mut reader)?;
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

/// Read Fortran record marker and return the record length in bytes
fn read_record_marker<R: Read>(reader: &mut R, byte_order: ByteOrder) -> io::Result<i32> {
    read_i32(reader, byte_order)
}

/// Skip Fortran record marker (4-byte integer)
fn skip_record_marker<R: Read>(reader: &mut R) -> io::Result<()> {
    let mut buf = [0u8; 4];
    reader.read_exact(&mut buf)?;
    Ok(())
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

/// Read three f32 arrays (x,y,z) with Fortran record markers
/// Handles both separate records (one per coordinate) and combined records (all xyz in one)
fn read_xyz_coords_with_markers<R: Read>(
    reader: &mut R,
    count: usize,
    byte_order: ByteOrder,
) -> io::Result<(Vec<f32>, Vec<f32>, Vec<f32>, Precision)> {
    let record_size = read_record_marker(reader, byte_order)?;

    if record_size <= 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Invalid record marker: {}", record_size),
        ));
    }

    let total_values_f32 = record_size as usize / 4;
    let total_values_f64 = record_size as usize / 8;

    if total_values_f32 == count * 3 {
        let x_coords = read_values_with_precision(reader, count, byte_order, Precision::F32)?;
        let y_coords = read_values_with_precision(reader, count, byte_order, Precision::F32)?;
        let z_coords = read_values_with_precision(reader, count, byte_order, Precision::F32)?;

        let closing_marker = read_record_marker(reader, byte_order)?;
        if closing_marker != record_size {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "Record marker mismatch: {} != {}",
                    record_size, closing_marker
                ),
            ));
        }

        Ok((x_coords, y_coords, z_coords, Precision::F32))
    } else if total_values_f64 == count * 3 {
        let x_coords = read_values_with_precision(reader, count, byte_order, Precision::F64)?;
        let y_coords = read_values_with_precision(reader, count, byte_order, Precision::F64)?;
        let z_coords = read_values_with_precision(reader, count, byte_order, Precision::F64)?;

        let closing_marker = read_record_marker(reader, byte_order)?;
        if closing_marker != record_size {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "Record marker mismatch: {} != {}",
                    record_size, closing_marker
                ),
            ));
        }

        Ok((x_coords, y_coords, z_coords, Precision::F64))
    } else {
        let precision = match record_size as usize {
            size if size == count * 4 => Precision::F32,
            size if size == count * 8 => Precision::F64,
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!(
                        "Unexpected precision: {} bytes per value",
                        record_size as usize / count
                    ),
                ));
            }
        };

        let x_coords = read_values_with_precision(reader, count, byte_order, precision)?;

        let closing_marker = read_record_marker(reader, byte_order)?;
        if closing_marker != record_size {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "X record marker mismatch: {} != {}",
                    record_size, closing_marker
                ),
            ));
        }

        let y_coords = read_f32_array_with_markers_precision(reader, count, byte_order, precision)?;
        let z_coords = read_f32_array_with_markers_precision(reader, count, byte_order, precision)?;

        Ok((x_coords, y_coords, z_coords, precision))
    }
}

fn read_values_with_precision<R: Read>(
    reader: &mut R,
    count: usize,
    byte_order: ByteOrder,
    precision: Precision,
) -> io::Result<Vec<f32>> {
    let mut result = Vec::with_capacity(count);
    match precision {
        Precision::F32 => {
            for _ in 0..count {
                let mut buf = [0u8; 4];
                reader.read_exact(&mut buf)?;
                result.push(match byte_order {
                    ByteOrder::LittleEndian => f32::from_le_bytes(buf),
                    ByteOrder::BigEndian => f32::from_be_bytes(buf),
                });
            }
        }
        Precision::F64 => {
            for _ in 0..count {
                let mut buf = [0u8; 8];
                reader.read_exact(&mut buf)?;
                result.push(
                    (match byte_order {
                        ByteOrder::LittleEndian => f64::from_le_bytes(buf),
                        ByteOrder::BigEndian => f64::from_be_bytes(buf),
                    }) as f32,
                );
            }
        }
        Precision::Mixed => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Unexpected mixed precision in data block",
            ));
        }
    }
    Ok(result)
}

fn read_f32_array_with_markers_precision<R: Read>(
    reader: &mut R,
    count: usize,
    byte_order: ByteOrder,
    precision: Precision,
) -> io::Result<Vec<f32>> {
    let record_size = read_record_marker(reader, byte_order)?;
    let expected_size = match precision {
        Precision::F32 => (count * 4) as i32,
        Precision::F64 => (count * 8) as i32,
        Precision::Mixed => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Unexpected mixed precision in record",
            ));
        }
    };

    if record_size != expected_size {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "Record size {} does not match expected {}",
                record_size, expected_size
            ),
        ));
    }

    let result = read_values_with_precision(reader, count, byte_order, precision)?;

    let closing_marker = read_record_marker(reader, byte_order)?;
    if closing_marker != record_size {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "Record marker mismatch: {} != {}",
                record_size, closing_marker
            ),
        ));
    }

    Ok(result)
}

/// Try to read iblank array if present (returns None if no more data or invalid marker)
fn try_read_iblank_array<R: BufRead>(
    reader: &mut R,
    count: usize,
    byte_order: ByteOrder,
) -> io::Result<Option<Vec<i32>>> {
    let buf = reader.fill_buf()?;
    if buf.len() < 4 {
        return Ok(None);
    }

    let record_size = match byte_order {
        ByteOrder::LittleEndian => i32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]),
        ByteOrder::BigEndian => i32::from_be_bytes([buf[0], buf[1], buf[2], buf[3]]),
    };

    if record_size != (count * 4) as i32 {
        return Ok(None);
    }

    reader.consume(4);

    let mut iblank = Vec::with_capacity(count);
    for _ in 0..count {
        let mut buf = [0u8; 4];
        reader.read_exact(&mut buf)?;
        let value = match byte_order {
            ByteOrder::LittleEndian => i32::from_le_bytes(buf),
            ByteOrder::BigEndian => i32::from_be_bytes(buf),
        };
        iblank.push(value);
    }

    let closing_marker = read_record_marker(reader, byte_order)?;
    if closing_marker != record_size {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "iblank record marker mismatch: {} != {}",
                record_size, closing_marker
            ),
        ));
    }

    Ok(Some(iblank))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_grid_dimensions() {
        let dims = GridDimensions {
            i: 10,
            j: 20,
            k: 30,
        };
        let grid = Plot3DGrid {
            dimensions: dims.clone(),
            x_coords: vec![],
            y_coords: vec![],
            z_coords: vec![],
            iblank: None,
        };
        assert_eq!(grid.total_points(), 6000);
    }

    #[test]
    fn test_solution_total_points() {
        let solution = Plot3DSolution {
            grid_index: 0,
            dimensions: GridDimensions { i: 5, j: 4, k: 3 },
            rho: vec![],
            rhou: vec![],
            rhov: vec![],
            rhow: vec![],
            rhoe: vec![],
        };
        assert_eq!(solution.total_points(), 60);
    }

    #[test]
    fn test_function_total_points() {
        let function = Plot3DFunction {
            grid_index: 0,
            dimensions: GridDimensions { i: 2, j: 3, k: 4 },
            function_data: vec![],
        };
        assert_eq!(function.total_points(), 24);
    }

    #[test]
    fn test_byte_order_detection_little_endian() {
        // Create a buffer with a small value in little-endian format
        let mut data = vec![];
        data.extend_from_slice(&100i32.to_le_bytes());
        let mut cursor = std::io::Cursor::new(data);

        let result = detect_byte_order(&mut cursor);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), ByteOrder::LittleEndian);
    }

    #[test]
    fn test_byte_order_detection_big_endian() {
        // Create a buffer with a value that appears valid only in big-endian
        let mut data = vec![];
        data.extend_from_slice(&100i32.to_be_bytes());
        let mut cursor = std::io::Cursor::new(data);

        let result = detect_byte_order(&mut cursor);
        assert!(result.is_ok());
    }

    #[test]
    fn test_read_i32_little_endian() {
        let mut data = vec![];
        data.extend_from_slice(&42i32.to_le_bytes());
        let mut cursor = std::io::Cursor::new(data);

        let result = read_i32(&mut cursor, ByteOrder::LittleEndian);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_read_i32_big_endian() {
        let mut data = vec![];
        data.extend_from_slice(&42i32.to_be_bytes());
        let mut cursor = std::io::Cursor::new(data);

        let result = read_i32(&mut cursor, ByteOrder::BigEndian);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_read_f32_array() {
        let values = vec![1.0f32, 2.5f32, 3.14f32];
        let mut data = vec![];
        for v in &values {
            data.extend_from_slice(&v.to_le_bytes());
        }
        let mut cursor = std::io::Cursor::new(data);

        let result = read_f32_array(&mut cursor, 3, ByteOrder::LittleEndian);
        assert!(result.is_ok());
        let arr = result.unwrap();
        assert_eq!(arr.len(), 3);
        assert_eq!(arr[0], 1.0);
        assert_eq!(arr[1], 2.5);
        assert_eq!(arr[2], 3.14);
    }

    #[test]
    fn test_read_plot3d_grid_ascii_simple() -> io::Result<()> {
        // Create a temporary ASCII PLOT3D file
        let mut temp_file = NamedTempFile::new()?;
        writeln!(temp_file, "1")?; // 1 grid
        writeln!(temp_file, "2 2 2")?; // 2x2x2 dimensions

        // Write 8 X coordinates
        writeln!(temp_file, "0.0 1.0 0.0 1.0 0.0 1.0 0.0 1.0")?;
        // Write 8 Y coordinates
        writeln!(temp_file, "0.0 0.0 1.0 1.0 0.0 0.0 1.0 1.0")?;
        // Write 8 Z coordinates
        writeln!(temp_file, "0.0 0.0 0.0 0.0 1.0 1.0 1.0 1.0")?;

        temp_file.flush()?;
        let path = temp_file.path();

        let result = read_plot3d_grid_ascii(path);
        assert!(result.is_ok());

        let grids = result.unwrap();
        assert_eq!(grids.len(), 1);
        assert_eq!(grids[0].dimensions.i, 2);
        assert_eq!(grids[0].dimensions.j, 2);
        assert_eq!(grids[0].dimensions.k, 2);
        assert_eq!(grids[0].total_points(), 8);
        assert_eq!(grids[0].x_coords.len(), 8);
        assert_eq!(grids[0].y_coords.len(), 8);
        assert_eq!(grids[0].z_coords.len(), 8);

        Ok(())
    }

    #[test]
    fn test_read_plot3d_grid_ascii_invalid_grid_count() -> io::Result<()> {
        let mut temp_file = NamedTempFile::new()?;
        writeln!(temp_file, "-1")?; // Invalid grid count
        temp_file.flush()?;

        let result = read_plot3d_grid_ascii(temp_file.path());
        assert!(result.is_err());

        Ok(())
    }

    #[test]
    fn test_read_plot3d_grid_ascii_zero_dimensions() -> io::Result<()> {
        let mut temp_file = NamedTempFile::new()?;
        writeln!(temp_file, "1")?;
        writeln!(temp_file, "0 2 2")?; // Zero dimension
        temp_file.flush()?;

        let result = read_plot3d_grid_ascii(temp_file.path());
        assert!(result.is_err());

        Ok(())
    }

    #[test]
    fn test_read_plot3d_solution_ascii_simple() -> io::Result<()> {
        let mut temp_file = NamedTempFile::new()?;
        writeln!(temp_file, "1")?; // 1 grid
        writeln!(temp_file, "2 1 1")?; // 2x1x1 = 2 points

        // Write 5 variables × 2 points = 10 values
        writeln!(temp_file, "1.0 2.0")?; // rho
        writeln!(temp_file, "3.0 4.0")?; // rhou
        writeln!(temp_file, "5.0 6.0")?; // rhov
        writeln!(temp_file, "7.0 8.0")?; // rhow
        writeln!(temp_file, "9.0 10.0")?; // rhoe

        temp_file.flush()?;

        let result = read_plot3d_solution_ascii(temp_file.path());
        assert!(result.is_ok());

        let solutions = result.unwrap();
        assert_eq!(solutions.len(), 1);
        assert_eq!(solutions[0].total_points(), 2);
        assert_eq!(solutions[0].rho, vec![1.0, 2.0]);
        assert_eq!(solutions[0].rhou, vec![3.0, 4.0]);
        assert_eq!(solutions[0].rhov, vec![5.0, 6.0]);
        assert_eq!(solutions[0].rhow, vec![7.0, 8.0]);
        assert_eq!(solutions[0].rhoe, vec![9.0, 10.0]);

        Ok(())
    }

    #[test]
    fn test_grid_dimensions_clone() {
        let dims1 = GridDimensions { i: 5, j: 10, k: 15 };
        let dims2 = dims1.clone();
        assert_eq!(dims1.i, dims2.i);
        assert_eq!(dims1.j, dims2.j);
        assert_eq!(dims1.k, dims2.k);
    }

    #[test]
    fn test_byte_order_equality() {
        assert_eq!(ByteOrder::LittleEndian, ByteOrder::LittleEndian);
        assert_eq!(ByteOrder::BigEndian, ByteOrder::BigEndian);
        assert_ne!(ByteOrder::LittleEndian, ByteOrder::BigEndian);
    }

    #[test]
    fn test_mesh_geometry_simple_grid() {
        // Create a simple 2x2x1 grid
        let grid = Plot3DGrid {
            dimensions: GridDimensions { i: 2, j: 2, k: 1 },
            x_coords: vec![0.0, 1.0, 0.0, 1.0],
            y_coords: vec![0.0, 0.0, 1.0, 1.0],
            z_coords: vec![0.0, 0.0, 0.0, 0.0],
            iblank: None,
        };

        let mesh = grid.to_mesh_geometry();

        // Check vertex count
        assert_eq!(mesh.vertex_count, 4);
        assert_eq!(mesh.vertices.len(), 12); // 4 vertices * 3 components

        // Check vertices
        assert_eq!(mesh.vertices[0], 0.0); // x of vertex 0
        assert_eq!(mesh.vertices[1], 0.0); // y of vertex 0
        assert_eq!(mesh.vertices[2], 0.0); // z of vertex 0

        // Check that indices were generated
        assert!(mesh.indices.len() > 0);
        assert_eq!(mesh.face_count, mesh.indices.len() / 3);

        // Check normals
        assert_eq!(mesh.normals.len(), 12);
    }

    #[test]
    fn test_mesh_geometry_larger_grid() {
        // Create a 3x3x2 grid
        let mut grid = Plot3DGrid {
            dimensions: GridDimensions { i: 3, j: 3, k: 2 },
            x_coords: Vec::with_capacity(18),
            y_coords: Vec::with_capacity(18),
            z_coords: Vec::with_capacity(18),
            iblank: None,
        };

        // Fill with test data
        for i in 0..3 {
            for j in 0..3 {
                for k in 0..2 {
                    grid.x_coords.push(i as f32);
                    grid.y_coords.push(j as f32);
                    grid.z_coords.push(k as f32);
                }
            }
        }

        let mesh = grid.to_mesh_geometry();

        assert_eq!(mesh.vertex_count, 18);
        assert_eq!(mesh.vertices.len(), 54); // 18 * 3
        assert!(mesh.indices.len() > 0);
        assert_eq!(mesh.normals.len(), 54);
    }

    #[test]
    fn test_mesh_linear_index_calculation() {
        // Test the linear index calculation
        let i = 1;
        let j = 2;
        let k = 1;
        let dim_i = 3;
        let dim_j = 4;

        let idx = Plot3DGrid::linear_index(i, j, k, dim_i, dim_j);
        // k * (i*j) + j * i + i = 1 * 12 + 2 * 3 + 1 = 19
        assert_eq!(idx, 19);
    }

    #[test]
    fn test_mesh_geometry_normals_normalized() {
        // Create a simple grid and check that normals are normalized
        let grid = Plot3DGrid {
            dimensions: GridDimensions { i: 2, j: 2, k: 1 },
            x_coords: vec![0.0, 1.0, 0.0, 1.0],
            y_coords: vec![0.0, 0.0, 1.0, 1.0],
            z_coords: vec![0.0, 0.0, 0.0, 0.0],
            iblank: None,
        };

        let mesh = grid.to_mesh_geometry();

        // Check that normals are normalized (length should be 1 or close to 0)
        for i in (0..mesh.normals.len()).step_by(3) {
            let nx = mesh.normals[i];
            let ny = mesh.normals[i + 1];
            let nz = mesh.normals[i + 2];
            let length_sq = nx * nx + ny * ny + nz * nz;

            // Should be either ~1 (normalized) or ~0 (no normal contribution)
            assert!(
                length_sq < 1.1 && (length_sq > 0.9 || length_sq < 0.01),
                "Normal magnitude squared: {}",
                length_sq
            );
        }
    }

    #[test]
    fn test_mesh_geometry_preserves_coordinates() {
        let coords = vec![1.5, 2.5, 3.5, 4.5];
        let grid = Plot3DGrid {
            dimensions: GridDimensions { i: 2, j: 2, k: 1 },
            x_coords: coords.clone(),
            y_coords: coords.clone(),
            z_coords: coords.clone(),
            iblank: None,
        };

        let mesh = grid.to_mesh_geometry();

        // Check that coordinates are preserved in vertices
        for i in 0..4 {
            assert_eq!(mesh.vertices[i * 3], coords[i]);
            assert_eq!(mesh.vertices[i * 3 + 1], coords[i]);
            assert_eq!(mesh.vertices[i * 3 + 2], coords[i]);
        }
    }

    #[test]
    fn test_read_plot3d_grid_with_metadata_binary() -> io::Result<()> {
        // Create a simple binary PLOT3D grid file with Fortran record markers
        let mut temp_file = NamedTempFile::new()?;

        // Record 1: Number of grids (1 grid) with Fortran markers
        temp_file.write_all(&4i32.to_le_bytes())?; // Opening marker
        temp_file.write_all(&1i32.to_le_bytes())?; // num_grids = 1
        temp_file.write_all(&4i32.to_le_bytes())?; // Closing marker

        // Record 2: Dimensions (2x2x2) with Fortran markers
        temp_file.write_all(&12i32.to_le_bytes())?; // Opening marker (3 * 4 bytes)
        temp_file.write_all(&2i32.to_le_bytes())?; // i = 2
        temp_file.write_all(&2i32.to_le_bytes())?; // j = 2
        temp_file.write_all(&2i32.to_le_bytes())?; // k = 2
        temp_file.write_all(&12i32.to_le_bytes())?; // Closing marker

        // Record 3: X coordinates (8 values) with markers
        temp_file.write_all(&32i32.to_le_bytes())?; // Opening marker (8 * 4 bytes)
        for i in 0..8 {
            temp_file.write_all(&(i as f32).to_le_bytes())?;
        }
        temp_file.write_all(&32i32.to_le_bytes())?; // Closing marker

        // Record 4: Y coordinates (8 values) with markers
        temp_file.write_all(&32i32.to_le_bytes())?;
        for i in 0..8 {
            temp_file.write_all(&(i as f32 + 0.5).to_le_bytes())?;
        }
        temp_file.write_all(&32i32.to_le_bytes())?;

        // Record 5: Z coordinates (8 values) with markers
        temp_file.write_all(&32i32.to_le_bytes())?;
        for i in 0..8 {
            temp_file.write_all(&(i as f32 + 1.0).to_le_bytes())?;
        }
        temp_file.write_all(&32i32.to_le_bytes())?;

        temp_file.flush()?;

        let result = read_plot3d_grid_with_metadata(temp_file.path());
        assert!(result.is_ok());

        let (grids, metadata) = result.unwrap();
        assert_eq!(grids.len(), 1);
        assert_eq!(metadata.num_grids, 1);
        assert_eq!(metadata.byte_order, "Little-Endian");
        assert_eq!(grids[0].dimensions.i, 2);
        assert_eq!(grids[0].dimensions.j, 2);
        assert_eq!(grids[0].dimensions.k, 2);
        assert_eq!(grids[0].total_points(), 8);

        Ok(())
    }

    #[test]
    fn test_read_plot3d_solution_binary() -> io::Result<()> {
        // Create a binary PLOT3D solution file with complete Fortran record markers
        let mut temp_file = NamedTempFile::new()?;

        // Record 1: num_grids
        temp_file.write_all(&4i32.to_le_bytes())?; // Opening marker (4 bytes of data)
        temp_file.write_all(&1i32.to_le_bytes())?; // num_grids = 1
        temp_file.write_all(&4i32.to_le_bytes())?; // Closing marker

        // Record 2: dimensions (3 integers = 12 bytes)
        temp_file.write_all(&12i32.to_le_bytes())?; // Opening marker
        temp_file.write_all(&2i32.to_le_bytes())?; // i = 2
        temp_file.write_all(&1i32.to_le_bytes())?; // j = 1
        temp_file.write_all(&1i32.to_le_bytes())?; // k = 1
        temp_file.write_all(&12i32.to_le_bytes())?; // Closing marker

        // Solution data for 2 points (i=2, j=1, k=1), 5 variables
        let rho_data = vec![1.0f32, 2.0f32];
        let rhou_data = vec![3.0f32, 4.0f32];
        let rhov_data = vec![5.0f32, 6.0f32];
        let rhow_data = vec![7.0f32, 8.0f32];
        let rhoe_data = vec![9.0f32, 10.0f32];

        // Record 3: rho array (2 floats = 8 bytes)
        temp_file.write_all(&8i32.to_le_bytes())?;
        for v in &rho_data {
            temp_file.write_all(&v.to_le_bytes())?;
        }
        temp_file.write_all(&8i32.to_le_bytes())?;

        // Record 4: rhou array
        temp_file.write_all(&8i32.to_le_bytes())?;
        for v in &rhou_data {
            temp_file.write_all(&v.to_le_bytes())?;
        }
        temp_file.write_all(&8i32.to_le_bytes())?;

        // Record 5: rhov array
        temp_file.write_all(&8i32.to_le_bytes())?;
        for v in &rhov_data {
            temp_file.write_all(&v.to_le_bytes())?;
        }
        temp_file.write_all(&8i32.to_le_bytes())?;

        // Record 6: rhow array
        temp_file.write_all(&8i32.to_le_bytes())?;
        for v in &rhow_data {
            temp_file.write_all(&v.to_le_bytes())?;
        }
        temp_file.write_all(&8i32.to_le_bytes())?;

        // Record 7: rhoe array
        temp_file.write_all(&8i32.to_le_bytes())?;
        for v in &rhoe_data {
            temp_file.write_all(&v.to_le_bytes())?;
        }
        temp_file.write_all(&8i32.to_le_bytes())?;

        temp_file.flush()?;

        let result = read_plot3d_solution(temp_file.path());
        assert!(
            result.is_ok(),
            "Failed to read binary solution: {:?}",
            result.err()
        );

        let solutions = result.unwrap();
        assert_eq!(solutions.len(), 1);
        assert_eq!(solutions[0].total_points(), 2);
        assert_eq!(solutions[0].rho, rho_data);
        assert_eq!(solutions[0].rhou, rhou_data);

        Ok(())
    }

    #[test]
    fn test_read_plot3d_function_binary() -> io::Result<()> {
        // Create a binary PLOT3D function file with complete Fortran record markers
        let mut temp_file = NamedTempFile::new()?;

        // Record 1: num_grids
        temp_file.write_all(&4i32.to_le_bytes())?; // Opening marker
        temp_file.write_all(&1i32.to_le_bytes())?; // num_grids = 1
        temp_file.write_all(&4i32.to_le_bytes())?; // Closing marker

        // Record 2: dimensions (3 integers = 12 bytes)
        temp_file.write_all(&12i32.to_le_bytes())?; // Opening marker
        temp_file.write_all(&2i32.to_le_bytes())?; // i = 2
        temp_file.write_all(&1i32.to_le_bytes())?; // j = 1
        temp_file.write_all(&1i32.to_le_bytes())?; // k = 1
        temp_file.write_all(&12i32.to_le_bytes())?; // Closing marker

        // Record 3: num_functions
        temp_file.write_all(&4i32.to_le_bytes())?; // Opening marker
        temp_file.write_all(&2i32.to_le_bytes())?; // num_functions = 2
        temp_file.write_all(&4i32.to_le_bytes())?; // Closing marker

        // Function data for 2 points (i=2, j=1, k=1), 2 functions
        let func1_data = vec![1.0f32, 2.0f32];
        let func2_data = vec![3.0f32, 4.0f32];

        // Record 4: First function array (2 floats = 8 bytes)
        temp_file.write_all(&8i32.to_le_bytes())?;
        for v in &func1_data {
            temp_file.write_all(&v.to_le_bytes())?;
        }
        temp_file.write_all(&8i32.to_le_bytes())?;

        // Record 5: Second function array (2 floats = 8 bytes)
        temp_file.write_all(&8i32.to_le_bytes())?;
        for v in &func2_data {
            temp_file.write_all(&v.to_le_bytes())?;
        }
        temp_file.write_all(&8i32.to_le_bytes())?;

        temp_file.flush()?;

        let result = read_plot3d_function(temp_file.path());
        assert!(
            result.is_ok(),
            "Failed to read binary function: {:?}",
            result.err()
        );

        let functions = result.unwrap();
        assert_eq!(functions.len(), 1);
        assert_eq!(functions[0].total_points(), 2);
        assert_eq!(functions[0].function_data.len(), 2);
        assert_eq!(functions[0].function_data[0], func1_data);
        assert_eq!(functions[0].function_data[1], func2_data);

        Ok(())
    }

    #[test]
    fn test_precision_as_str() {
        assert_eq!(Precision::F32.as_str(), "f32");
        assert_eq!(Precision::F64.as_str(), "f64");
        assert_eq!(Precision::Mixed.as_str(), "mixed");
    }
}

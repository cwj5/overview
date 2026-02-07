use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Read};
use std::path::Path;

// Thread-local storage for last loaded solution file metadata
thread_local! {
    static LAST_SOLUTION_METADATA: RefCell<Option<SolutionFileMetadata>> = RefCell::new(None);
}

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

/// File metadata about the loaded solution file
#[derive(Debug, Clone)]
pub struct SolutionFileMetadata {
    pub format: String,     // "binary" or "ASCII"
    pub precision: String,  // "f32" or "f64"
    pub byte_order: String, // "Little-Endian" or "Big-Endian" (ASCII uses "N/A")
}

/// PLOT3D solution metadata from Q file header
/// Fields are read in sequence; if the metadata record is shorter than expected,
/// later fields will be None
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plot3DMetadata {
    pub refmach: Option<f32>,   // Reference Mach number
    pub alpha: Option<f32>,     // Angle of attack (degrees)
    pub rey: Option<f32>,       // Reynolds number
    pub time: Option<f32>,      // Time value
    pub gaminf: Option<f32>,    // Gamma at infinity
    pub beta: Option<f32>,      // Sideslip angle (degrees)
    pub tinf: Option<f32>,      // Temperature at infinity
    pub igam: Option<f32>,      // Gas model flag (0=perfect gas, 1=equilibrium)
    pub htinf: Option<f32>,     // Total enthalpy at infinity
    pub ht1: Option<f32>,       // Reserved
    pub ht2: Option<f32>,       // Reserved
    pub rgas: Option<Vec<f32>>, // Gas constants (variable length)
    pub fsmach: Option<f32>,    // Free stream Mach number
    pub tvref: Option<f32>,     // Reference temperature
    pub dtvref: Option<f32>,    // Delta reference temperature
}

/// Represents PLOT3D solution data (Q file)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plot3DSolution {
    pub grid_index: usize,
    pub dimensions: GridDimensions,
    pub rho: Vec<f32>,  // Density (non-dimensional)
    pub rhou: Vec<f32>, // Momentum X (non-dimensional)
    pub rhov: Vec<f32>, // Momentum Y (non-dimensional)
    pub rhow: Vec<f32>, // Momentum Z (non-dimensional)
    pub rhoe: Vec<f32>, // Total Energy (non-dimensional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gamma: Option<Vec<f32>>, // Ratio of specific heats (always at Q[5], NQ=6+NQC+NQT)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Plot3DMetadata>, // Solution metadata from file header
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub colors: Option<Vec<f32>>, // Optional vertex colors (r, g, b interleaved)
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
    /// This creates quad edges for wireframe display (4 edges per quad, no triangulation)
    /// If respect_iblank is true and iblank data exists, points with iblank=0 are excluded
    pub fn to_mesh_geometry(&self, respect_iblank: bool) -> MeshGeometry {
        let i = self.dimensions.i as usize;
        let j = self.dimensions.j as usize;
        let total_points = self.total_points();

        // Helper function to check if a point is blanked
        let is_blanked = |idx: usize| -> bool {
            if respect_iblank {
                if let Some(ref iblank) = self.iblank {
                    return iblank[idx] == 0;
                }
            }
            false
        };

        // Convert coordinates to vertex array (x, y, z interleaved)
        let mut vertices = Vec::with_capacity(total_points * 3);
        for idx in 0..total_points {
            vertices.push(self.x_coords[idx]);
            vertices.push(self.y_coords[idx]);
            vertices.push(self.z_coords[idx]);
        }

        // Generate line indices for quad edges (not triangles)
        // For a structured grid, we only render the k=1 surface (k=0 in 0-indexed)
        let mut indices = Vec::new();

        // Create edges for I-J plane quads (constant K surface) - only k=0
        let k_idx = 0;
        for j_idx in 0..j - 1 {
            for i_idx in 0..i - 1 {
                let idx00 = Self::linear_index(i_idx, j_idx, k_idx, i, j);
                let idx10 = Self::linear_index(i_idx + 1, j_idx, k_idx, i, j);
                let idx01 = Self::linear_index(i_idx, j_idx + 1, k_idx, i, j);
                let idx11 = Self::linear_index(i_idx + 1, j_idx + 1, k_idx, i, j);

                // Skip this quad if any corner is blanked
                if is_blanked(idx00) || is_blanked(idx10) || is_blanked(idx01) || is_blanked(idx11)
                {
                    continue;
                }

                // Bottom edge (idx00 -> idx10)
                indices.push(idx00 as u32);
                indices.push(idx10 as u32);

                // Right edge (idx10 -> idx11)
                indices.push(idx10 as u32);
                indices.push(idx11 as u32);

                // Top edge (idx11 -> idx01)
                indices.push(idx11 as u32);
                indices.push(idx01 as u32);

                // Left edge (idx01 -> idx00)
                indices.push(idx01 as u32);
                indices.push(idx00 as u32);
            }
        }

        // Compute simple vertex normals (averaged from adjacent faces)
        // For line rendering, normals aren't critical, but we keep them for consistency
        let mut normals = vec![0.0f32; total_points * 3];

        // For each quad, compute a normal and distribute it to vertices
        for j_idx in 0..j - 1 {
            for i_idx in 0..i - 1 {
                let idx00 = Self::linear_index(i_idx, j_idx, k_idx, i, j);
                let idx10 = Self::linear_index(i_idx + 1, j_idx, k_idx, i, j);
                let idx01 = Self::linear_index(i_idx, j_idx + 1, k_idx, i, j);

                let v0 = [
                    vertices[idx00 * 3],
                    vertices[idx00 * 3 + 1],
                    vertices[idx00 * 3 + 2],
                ];
                let v1 = [
                    vertices[idx10 * 3],
                    vertices[idx10 * 3 + 1],
                    vertices[idx10 * 3 + 2],
                ];
                let v2 = [
                    vertices[idx01 * 3],
                    vertices[idx01 * 3 + 1],
                    vertices[idx01 * 3 + 2],
                ];

                // Compute quad normal using cross product
                let edge1 = [v1[0] - v0[0], v1[1] - v0[1], v1[2] - v0[2]];
                let edge2 = [v2[0] - v0[0], v2[1] - v0[1], v2[2] - v0[2]];

                let normal = [
                    edge1[1] * edge2[2] - edge1[2] * edge2[1],
                    edge1[2] * edge2[0] - edge1[0] * edge2[2],
                    edge1[0] * edge2[1] - edge1[1] * edge2[0],
                ];

                // Add to all four vertices of the quad
                let idx11 = Self::linear_index(i_idx + 1, j_idx + 1, k_idx, i, j);
                for &idx in &[idx00, idx10, idx01, idx11] {
                    normals[idx * 3] += normal[0];
                    normals[idx * 3 + 1] += normal[1];
                    normals[idx * 3 + 2] += normal[2];
                }
            }
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

        // For line rendering, face_count represents number of line segments (indices.len() / 2)
        let line_count = indices.len() / 2;

        MeshGeometry {
            vertices,
            indices,
            normals,
            vertex_count: total_points,
            face_count: line_count,
            colors: None,
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

/// Parse metadata from buffer, reading as many fields as available
/// Fields are read in order: REFMACH, ALPHA, REY, TIME, GAMINF, BETA, TINF, IGAM, HTINF, HT1, HT2, RGAS[...], FSMACH, TVREF, DTVREF
/// If the buffer is shorter than expected, later fields will be None
fn parse_metadata(buffer: &[u8], byte_order: ByteOrder) -> Plot3DMetadata {
    let num_floats = buffer.len() / 4;
    let mut values = Vec::with_capacity(num_floats);

    // Read all available f32 values from buffer
    for i in 0..num_floats {
        let start = i * 4;
        if start + 4 <= buffer.len() {
            let bytes = [
                buffer[start],
                buffer[start + 1],
                buffer[start + 2],
                buffer[start + 3],
            ];
            let value = match byte_order {
                ByteOrder::LittleEndian => f32::from_le_bytes(bytes),
                ByteOrder::BigEndian => f32::from_be_bytes(bytes),
            };
            values.push(value);
        }
    }

    // Extract fields in order - if not enough values, fields remain None
    let refmach = values.get(0).copied();
    let alpha = values.get(1).copied();
    let rey = values.get(2).copied();
    let time = values.get(3).copied();
    let gaminf = values.get(4).copied();
    let beta = values.get(5).copied();
    let tinf = values.get(6).copied();
    let igam = values.get(7).copied();
    let htinf = values.get(8).copied();
    let ht1 = values.get(9).copied();
    let ht2 = values.get(10).copied();

    // RGAS is variable length - collect remaining values except last 3 (FSMACH, TVREF, DTVREF)
    // If we have at least 15 values (11 fixed + 1 RGAS + 3 tail), assume last 3 are FSMACH, TVREF, DTVREF
    let (rgas, fsmach, tvref, dtvref) = if values.len() > 14 {
        let rgas_values: Vec<f32> = values[11..values.len() - 3].to_vec();
        let fsmach = values.get(values.len() - 3).copied();
        let tvref = values.get(values.len() - 2).copied();
        let dtvref = values.get(values.len() - 1).copied();
        (
            if rgas_values.is_empty() {
                None
            } else {
                Some(rgas_values)
            },
            fsmach,
            tvref,
            dtvref,
        )
    } else if values.len() > 11 {
        // We have values beyond the first 11, but not enough for the last 3
        let rgas_values: Vec<f32> = values[11..].to_vec();
        (
            if rgas_values.is_empty() {
                None
            } else {
                Some(rgas_values)
            },
            None,
            None,
            None,
        )
    } else {
        // No RGAS or tail values
        (None, None, None, None)
    };

    Plot3DMetadata {
        refmach,
        alpha,
        rey,
        time,
        gaminf,
        beta,
        tinf,
        igam,
        htinf,
        ht1,
        ht2,
        rgas,
        fsmach,
        tvref,
        dtvref,
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

    // Read dimensions and NQ, NQC from a single record
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

    // Read NQ (number of solution variables) and NQC (number of conservative variables)
    let nq = read_i32(&mut reader, byte_order)? as usize;
    let _nqc = read_i32(&mut reader, byte_order)? as usize;
    skip_record_marker(&mut reader)?;

    if nq < 5 || nq > 100 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Invalid NQ: {}", nq),
        ));
    }

    let mut solutions = Vec::with_capacity(num_grids as usize);

    // Track the first precision detected (should be same for whole file)
    let mut detected_precision: Option<Precision> = None;

    // Read solution data for each grid
    for (grid_index, dims) in dimensions_list.into_iter().enumerate() {
        let total_points = (dims.i as usize) * (dims.j as usize) * (dims.k as usize);

        // FIRST: Detect precision from Q array record size before reading metadata
        // Peek at the Q array record marker to determine f32 vs f64
        // We need to know this before reading data, so read metadata and Q positions first

        // Record sequence: metadata record marker, metadata, closing marker, Q record marker, ...
        // Read metadata record opening marker
        let metadata_record_size = read_record_marker(&mut reader, byte_order)? as usize;

        // Read and store metadata buffer
        let mut metadata_buf = vec![0u8; metadata_record_size];
        reader.read_exact(&mut metadata_buf).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Failed to read metadata record: {}", e),
            )
        })?;
        skip_record_marker(&mut reader)?;

        // Now read the Q array record marker to detect precision BEFORE reading Q data
        let q_record_size = read_record_marker(&mut reader, byte_order)? as usize;

        // Determine precision based on Q record size
        // NQ variables * total_points values per variable
        let expected_f32_size = nq * total_points * 4; // f32 = 4 bytes
        let expected_f64_size = nq * total_points * 8; // f64 = 8 bytes

        let precision = if q_record_size == expected_f32_size {
            Precision::F32
        } else if q_record_size == expected_f64_size {
            Precision::F64
        } else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "Invalid Q record size: expected {} bytes (f32) or {} bytes (f64), got {} bytes",
                    expected_f32_size, expected_f64_size, q_record_size
                ),
            ));
        };

        // Track the first precision detected
        if detected_precision.is_none() {
            detected_precision = Some(precision);
        }

        // Parse metadata - will read as many fields as are available
        let metadata = parse_metadata(&metadata_buf, byte_order);

        // Now read Q data with the detected precision
        let mut q_data = vec![Vec::with_capacity(total_points); nq];
        for n in 0..nq {
            q_data[n] = match precision {
                Precision::F32 => read_f32_array(&mut reader, total_points, byte_order)?,
                Precision::F64 => {
                    // Read f64 and convert to f32 for storage (lossy but preserves values)
                    let f64_data = read_f64_array(&mut reader, total_points, byte_order)?;
                    f64_data.iter().map(|&v| v as f32).collect()
                }
                Precision::Mixed => {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "Mixed precision not supported for solution data",
                    ));
                }
            };
        }

        skip_record_marker(&mut reader)?;

        // Extract the conservative variables (first 5) and gamma (6th if present)
        // NQ = 6 + NQC + NQT where first 6 are: rho*, rho*u*, rho*v*, rho*w*, rho*e0*, gamma
        let rho = q_data.get(0).cloned().unwrap_or_default();
        let rhou = q_data.get(1).cloned().unwrap_or_default();
        let rhov = q_data.get(2).cloned().unwrap_or_default();
        let rhow = q_data.get(3).cloned().unwrap_or_default();
        let rhoe = q_data.get(4).cloned().unwrap_or_default();
        let gamma = q_data.get(5).cloned(); // Optional: ratio of specific heats

        // Validate we got the right amount of data
        if rho.len() != total_points
            || rhou.len() != total_points
            || rhov.len() != total_points
            || rhow.len() != total_points
            || rhoe.len() != total_points
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "Incomplete solution data for grid {}: expected {} points",
                    grid_index, total_points
                ),
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
            gamma,
            metadata: Some(metadata),
        });
    }

    // Set metadata for logging in the command handler
    let byte_order_str = match byte_order {
        ByteOrder::LittleEndian => "Little-Endian",
        ByteOrder::BigEndian => "Big-Endian",
    };
    let precision_str = detected_precision.unwrap_or(Precision::F32).as_str();
    set_last_solution_metadata(SolutionFileMetadata {
        format: "binary".to_string(),
        precision: precision_str.to_string(),
        byte_order: byte_order_str.to_string(),
    });

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

    // Read solution data for each grid
    for (grid_index, dims) in dimensions_list.into_iter().enumerate() {
        let total_points = (dims.i as usize) * (dims.j as usize) * (dims.k as usize);

        // First, read metadata values (variable number depending on file format)
        // Try to read metadata fields: REFMACH, ALPHA, REY, TIME, GAMINF, BETA, TINF, IGAM, HTINF, HT1, HT2, RGAS[...], FSMACH, TVREF, DTVREF
        // For ASCII files, we need to parse until we find the solution data
        // Minimum metadata is 4 values (REFMACH, ALPHA, REY, TIME)
        // We'll read values greedily and then determine which are metadata vs solution data

        let mut all_values: Vec<f32> = Vec::new();

        // Read values until we have enough for metadata + solution data
        // We need: metadata (at least 4 floats) + 5 variable arrays * total_points
        let min_metadata_count = 4;
        let min_solution_count = 5 * total_points;
        let min_total = min_metadata_count + min_solution_count;

        for line in lines.by_ref() {
            let line = line?;
            let values: Vec<f32> = line
                .split_whitespace()
                .map(|s| s.parse::<f32>())
                .collect::<Result<Vec<_>, _>>()
                .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Cannot parse value"))?;

            all_values.extend(values);

            if all_values.len() >= min_total {
                break;
            }
        }

        if all_values.len() < min_total {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "Incomplete solution data for grid {}: expected at least {} values, got {}",
                    grid_index,
                    min_total,
                    all_values.len()
                ),
            ));
        }

        // Split into metadata and solution data
        // We'll try to parse metadata first, reading as many fields as available
        // but leaving at least 5*total_points for solution data
        let max_metadata_idx = all_values.len() - min_solution_count;

        // Convert metadata values to buffer format for parse_metadata function
        let mut metadata_buf = Vec::new();
        for i in 0..max_metadata_idx {
            metadata_buf.extend_from_slice(&all_values[i].to_le_bytes());
        }
        let metadata = parse_metadata(&metadata_buf, ByteOrder::LittleEndian);

        // Remaining values are solution data
        let solution_data = &all_values[max_metadata_idx..];

        let mut rho = Vec::with_capacity(total_points);
        let mut rhou = Vec::with_capacity(total_points);
        let mut rhov = Vec::with_capacity(total_points);
        let mut rhow = Vec::with_capacity(total_points);
        let mut rhoe = Vec::with_capacity(total_points);
        let mut gamma = Vec::with_capacity(total_points);

        // Distribute values across the 5+ variables
        for (idx, &value) in solution_data.iter().enumerate() {
            let var_index = idx / total_points;
            let point_index = idx % total_points;

            match var_index {
                0 => {
                    if point_index >= rho.len() {
                        rho.push(value);
                    }
                }
                1 => {
                    if point_index >= rhou.len() {
                        rhou.push(value);
                    }
                }
                2 => {
                    if point_index >= rhov.len() {
                        rhov.push(value);
                    }
                }
                3 => {
                    if point_index >= rhow.len() {
                        rhow.push(value);
                    }
                }
                4 => {
                    if point_index >= rhoe.len() {
                        rhoe.push(value);
                    }
                }
                5 => {
                    if point_index >= gamma.len() {
                        gamma.push(value);
                    }
                }
                _ => break, // Additional variables beyond gamma
            }
        }

        let gamma_opt = if gamma.len() == total_points {
            Some(gamma)
        } else {
            None
        };

        // Validate we got the right amount of data
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
            gamma: gamma_opt,
            metadata: Some(metadata),
        });
    }

    // Set metadata for logging in the command handler
    set_last_solution_metadata(SolutionFileMetadata {
        format: "ASCII".to_string(),
        precision: "f32".to_string(),
        byte_order: "N/A".to_string(),
    });

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

#[allow(dead_code)]
fn read_f32<R: Read>(reader: &mut R, byte_order: ByteOrder) -> io::Result<f32> {
    let mut buf = [0u8; 4];
    reader.read_exact(&mut buf)?;
    Ok(match byte_order {
        ByteOrder::LittleEndian => f32::from_le_bytes(buf),
        ByteOrder::BigEndian => f32::from_be_bytes(buf),
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

fn read_f64_array<R: Read>(
    reader: &mut R,
    count: usize,
    byte_order: ByteOrder,
) -> io::Result<Vec<f64>> {
    let mut result = Vec::with_capacity(count);
    for _ in 0..count {
        let mut buf = [0u8; 8];
        reader.read_exact(&mut buf)?;
        let value = match byte_order {
            ByteOrder::LittleEndian => f64::from_le_bytes(buf),
            ByteOrder::BigEndian => f64::from_be_bytes(buf),
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

    // XYZ only (f32)
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
    }
    // XYZ only (f64)
    else if total_values_f64 == count * 3 {
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
    }
    // XYZ (f32) + IBLANK (i32): count * 3 * 4 + count * 4 = count * 16
    else if record_size as usize == count * 16 {
        let x_coords = read_values_with_precision(reader, count, byte_order, Precision::F32)?;
        let y_coords = read_values_with_precision(reader, count, byte_order, Precision::F32)?;
        let z_coords = read_values_with_precision(reader, count, byte_order, Precision::F32)?;

        // Skip IBLANK data (will be read separately if needed)
        let mut iblank_data = vec![0u8; count * 4];
        reader.read_exact(&mut iblank_data)?;

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
    }
    // XYZ (f64) + IBLANK (i32): count * 3 * 8 + count * 4 = count * 28
    else if record_size as usize == count * 28 {
        let x_coords = read_values_with_precision(reader, count, byte_order, Precision::F64)?;
        let y_coords = read_values_with_precision(reader, count, byte_order, Precision::F64)?;
        let z_coords = read_values_with_precision(reader, count, byte_order, Precision::F64)?;

        // Skip IBLANK data (will be read separately if needed)
        let mut iblank_data = vec![0u8; count * 4];
        reader.read_exact(&mut iblank_data)?;

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
    }
    // Separate XYZ records - check if record_size matches expected size for one coordinate array
    else {
        let precision = match record_size as usize {
            size if size == count * 4 => Precision::F32,
            size if size == count * 8 => Precision::F64,
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!(
                        "Invalid record size: expected {} (f32), {} (f64), {} (f32+IBLANK), or {} (f64+IBLANK) bytes, got {} bytes",
                        count * 12,  // XYZ f32
                        count * 24,  // XYZ f64
                        count * 16,  // XYZ f32 + IBLANK i32
                        count * 28,  // XYZ f64 + IBLANK i32
                        record_size
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

/// Get the metadata from the last loaded solution file
pub fn get_last_solution_metadata() -> Option<SolutionFileMetadata> {
    LAST_SOLUTION_METADATA.with(|m| m.borrow().clone())
}

/// Set the metadata for the last loaded solution file (internal use)
fn set_last_solution_metadata(metadata: SolutionFileMetadata) {
    LAST_SOLUTION_METADATA.with(|m| *m.borrow_mut() = Some(metadata));
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
            gamma: None,
            metadata: None,
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

        // Write metadata (4 minimum values): REFMACH, ALPHA, REY, TIME
        writeln!(temp_file, "1.2 5.0 1e6 0.5")?;

        // Write 5 variables × 2 points = 10 values
        writeln!(temp_file, "1.0 2.0")?; // rho
        writeln!(temp_file, "3.0 4.0")?; // rhou
        writeln!(temp_file, "5.0 6.0")?; // rhov
        writeln!(temp_file, "7.0 8.0")?; // rhow
        writeln!(temp_file, "9.0 10.0")?; // rhoe

        temp_file.flush()?;

        let result = read_plot3d_solution_ascii(temp_file.path());
        assert!(
            result.is_ok(),
            "Failed to read ASCII solution: {:?}",
            result.err()
        );

        let solutions = result.unwrap();
        assert_eq!(solutions.len(), 1);
        assert_eq!(solutions[0].total_points(), 2);
        assert_eq!(solutions[0].rho, vec![1.0, 2.0]);
        assert_eq!(solutions[0].rhou, vec![3.0, 4.0]);
        assert_eq!(solutions[0].rhov, vec![5.0, 6.0]);
        assert_eq!(solutions[0].rhow, vec![7.0, 8.0]);
        assert_eq!(solutions[0].rhoe, vec![9.0, 10.0]);

        // Check metadata was parsed
        assert!(solutions[0].metadata.is_some());
        let meta = solutions[0].metadata.as_ref().unwrap();
        assert_eq!(meta.refmach, Some(1.2));
        assert_eq!(meta.alpha, Some(5.0));

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

        let mesh = grid.to_mesh_geometry(false);

        // Check vertex count
        assert_eq!(mesh.vertex_count, 4);
        assert_eq!(mesh.vertices.len(), 12); // 4 vertices * 3 components

        // Check vertices
        assert_eq!(mesh.vertices[0], 0.0); // x of vertex 0
        assert_eq!(mesh.vertices[1], 0.0); // y of vertex 0
        assert_eq!(mesh.vertices[2], 0.0); // z of vertex 0

        // Check that indices were generated for line segments
        assert!(mesh.indices.len() > 0);
        assert_eq!(mesh.face_count, mesh.indices.len() / 2); // Line segments, not triangles

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

        let mesh = grid.to_mesh_geometry(false);

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

        let mesh = grid.to_mesh_geometry(false);

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

        let mesh = grid.to_mesh_geometry(false);

        // Check that coordinates are preserved in vertices
        for i in 0..4 {
            assert_eq!(mesh.vertices[i * 3], coords[i]);
            assert_eq!(mesh.vertices[i * 3 + 1], coords[i]);
            assert_eq!(mesh.vertices[i * 3 + 2], coords[i]);
        }
    }

    #[test]
    fn test_mesh_geometry_iblank_filtering() {
        // Create a 3x3x1 grid with some blanked points
        let grid = Plot3DGrid {
            dimensions: GridDimensions { i: 3, j: 3, k: 1 },
            x_coords: vec![0.0, 1.0, 2.0, 0.0, 1.0, 2.0, 0.0, 1.0, 2.0],
            y_coords: vec![0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 2.0, 2.0, 2.0],
            z_coords: vec![0.0; 9],
            // Blank the center point (index 4) and corner (index 8)
            iblank: Some(vec![1, 1, 1, 1, 0, 1, 1, 1, 0]),
        };

        // Without respecting iblank, should have 4 quads (2x2 grid = 4 quads)
        let mesh_no_blank = grid.to_mesh_geometry(false);
        assert_eq!(mesh_no_blank.face_count, 16); // 4 quads * 4 edges = 16 line segments

        // With respecting iblank, should have fewer quads (those with blanked corners are excluded)
        let mesh_with_blank = grid.to_mesh_geometry(true);
        // All 4 quads touch at least one blanked point (center or corner), so should have 0 quads
        assert_eq!(mesh_with_blank.face_count, 0);
    }

    #[test]
    fn test_mesh_geometry_iblank_partial_blanking() {
        // Create a 3x3x1 grid with only one corner blanked
        let grid = Plot3DGrid {
            dimensions: GridDimensions { i: 3, j: 3, k: 1 },
            x_coords: vec![0.0, 1.0, 2.0, 0.0, 1.0, 2.0, 0.0, 1.0, 2.0],
            y_coords: vec![0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 2.0, 2.0, 2.0],
            z_coords: vec![0.0; 9],
            // Blank only the top-right corner (index 8)
            iblank: Some(vec![1, 1, 1, 1, 1, 1, 1, 1, 0]),
        };

        // Without iblank: 4 quads
        let mesh_no_blank = grid.to_mesh_geometry(false);
        assert_eq!(mesh_no_blank.face_count, 16); // 4 quads * 4 edges

        // With iblank: should lose 1 quad (top-right quad that uses point 8)
        let mesh_with_blank = grid.to_mesh_geometry(true);
        assert_eq!(mesh_with_blank.face_count, 12); // 3 quads * 4 edges
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

        // Record 2: dimensions + NQ + NQC (5 integers = 20 bytes)
        temp_file.write_all(&20i32.to_le_bytes())?; // Opening marker
        temp_file.write_all(&2i32.to_le_bytes())?; // i = 2
        temp_file.write_all(&1i32.to_le_bytes())?; // j = 1
        temp_file.write_all(&1i32.to_le_bytes())?; // k = 1
        temp_file.write_all(&6i32.to_le_bytes())?; // NQ = 6 (5 conservative + gamma)
        temp_file.write_all(&0i32.to_le_bytes())?; // NQC = 0 (no species)
        temp_file.write_all(&20i32.to_le_bytes())?; // Closing marker

        // Record 3: Metadata record (minimal - just write a small metadata block)
        // For simplicity, write a small metadata record (16 floats = 64 bytes)
        temp_file.write_all(&64i32.to_le_bytes())?; // Opening marker
        for _ in 0..16 {
            temp_file.write_all(&0.0f32.to_le_bytes())?;
        }
        temp_file.write_all(&64i32.to_le_bytes())?; // Closing marker

        // Solution data for 2 points (i=2, j=1, k=1), 6 variables in ONE record
        let rho_data = vec![1.0f32, 2.0f32];
        let rhou_data = vec![3.0f32, 4.0f32];
        let rhov_data = vec![5.0f32, 6.0f32];
        let rhow_data = vec![7.0f32, 8.0f32];
        let rhoe_data = vec![9.0f32, 10.0f32];
        let gamma_data = vec![1.4f32, 1.4f32];

        // Record 4: All Q variables in ONE record (6 variables * 2 points * 4 bytes = 48 bytes)
        temp_file.write_all(&48i32.to_le_bytes())?; // Opening marker
        for v in &rho_data {
            temp_file.write_all(&v.to_le_bytes())?;
        }
        for v in &rhou_data {
            temp_file.write_all(&v.to_le_bytes())?;
        }
        for v in &rhov_data {
            temp_file.write_all(&v.to_le_bytes())?;
        }
        for v in &rhow_data {
            temp_file.write_all(&v.to_le_bytes())?;
        }
        for v in &rhoe_data {
            temp_file.write_all(&v.to_le_bytes())?;
        }
        for v in &gamma_data {
            temp_file.write_all(&v.to_le_bytes())?;
        }
        temp_file.write_all(&48i32.to_le_bytes())?; // Closing marker

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
        assert_eq!(solutions[0].gamma, Some(gamma_data));

        Ok(())
    }

    #[test]
    fn test_read_plot3d_solution_binary_with_large_metadata() -> io::Result<()> {
        // Test that we can handle metadata records larger than our minimal expectation
        // Some PLOT3D variants have extended metadata with additional fields
        let mut temp_file = NamedTempFile::new()?;

        // Record 1: num_grids
        temp_file.write_all(&4i32.to_le_bytes())?;
        temp_file.write_all(&1i32.to_le_bytes())?;
        temp_file.write_all(&4i32.to_le_bytes())?;

        // Record 2: dimensions + NQ + NQC
        temp_file.write_all(&20i32.to_le_bytes())?;
        temp_file.write_all(&3i32.to_le_bytes())?; // i = 3
        temp_file.write_all(&2i32.to_le_bytes())?; // j = 2
        temp_file.write_all(&1i32.to_le_bytes())?; // k = 1
        temp_file.write_all(&6i32.to_le_bytes())?; // NQ = 6
        temp_file.write_all(&0i32.to_le_bytes())?; // NQC = 0
        temp_file.write_all(&20i32.to_le_bytes())?;

        // Record 3: Large metadata record (128 floats = 512 bytes)
        // This is intentionally much larger than the minimal 16 floats we used before
        temp_file.write_all(&512i32.to_le_bytes())?;
        for i in 0..128 {
            temp_file.write_all(&(i as f32).to_le_bytes())?;
        }
        temp_file.write_all(&512i32.to_le_bytes())?;

        // Record 4: Q data for 6 points (3*2*1), 6 variables
        let total_points = 6;
        let total_floats = total_points * 6; // 36 floats = 144 bytes
        temp_file.write_all(&(total_floats as i32 * 4).to_le_bytes())?;

        // Write test data: rho, rhou, rhov, rhow, rhoe, gamma
        for var in 0..6 {
            for _pt in 0..total_points {
                let value = (var * 10 + _pt) as f32;
                temp_file.write_all(&value.to_le_bytes())?;
            }
        }
        temp_file.write_all(&(total_floats as i32 * 4).to_le_bytes())?;

        temp_file.flush()?;

        // Test that it can be read successfully despite large metadata
        let result = read_plot3d_solution(temp_file.path());
        assert!(
            result.is_ok(),
            "Failed to read solution with large metadata: {:?}",
            result.err()
        );

        let solutions = result.unwrap();
        assert_eq!(solutions.len(), 1);
        assert_eq!(solutions[0].total_points(), 6);

        // Verify the data was read correctly (rho should be 0,1,2,3,4,5)
        assert_eq!(solutions[0].rho[0], 0.0);
        assert_eq!(solutions[0].rho[1], 1.0);
        assert_eq!(solutions[0].rho[5], 5.0);

        // Verify gamma was extracted correctly (should be 50,51,52,53,54,55)
        assert!(solutions[0].gamma.is_some());
        let gamma = solutions[0].gamma.as_ref().unwrap();
        assert_eq!(gamma[0], 50.0);
        assert_eq!(gamma[5], 55.0);

        Ok(())
    }

    #[test]
    fn test_read_plot3d_solution_binary_with_minimal_metadata() -> io::Result<()> {
        // Test with minimal metadata (e.g., just a few required fields)
        let mut temp_file = NamedTempFile::new()?;

        // Record 1: num_grids
        temp_file.write_all(&4i32.to_le_bytes())?;
        temp_file.write_all(&1i32.to_le_bytes())?;
        temp_file.write_all(&4i32.to_le_bytes())?;

        // Record 2: dimensions + NQ + NQC
        temp_file.write_all(&20i32.to_le_bytes())?;
        temp_file.write_all(&2i32.to_le_bytes())?; // i = 2
        temp_file.write_all(&2i32.to_le_bytes())?; // j = 2
        temp_file.write_all(&1i32.to_le_bytes())?; // k = 1
        temp_file.write_all(&6i32.to_le_bytes())?; // NQ = 6
        temp_file.write_all(&0i32.to_le_bytes())?; // NQC = 0
        temp_file.write_all(&20i32.to_le_bytes())?;

        // Record 3: Minimal metadata (just 4 floats = 16 bytes)
        temp_file.write_all(&16i32.to_le_bytes())?;
        temp_file.write_all(&1.0f32.to_le_bytes())?; // REFMACH
        temp_file.write_all(&0.0f32.to_le_bytes())?; // ALPHA
        temp_file.write_all(&0.0f32.to_le_bytes())?; // REY
        temp_file.write_all(&0.0f32.to_le_bytes())?; // TIME
        temp_file.write_all(&16i32.to_le_bytes())?;

        // Record 4: Q data for 4 points, 6 variables
        let total_points = 4;
        temp_file.write_all(&96i32.to_le_bytes())?; // 24 floats * 4 bytes

        for var in 0..6 {
            for _pt in 0..total_points {
                temp_file.write_all(&((var + 1) as f32).to_le_bytes())?;
            }
        }
        temp_file.write_all(&96i32.to_le_bytes())?;

        temp_file.flush()?;

        let result = read_plot3d_solution(temp_file.path());
        assert!(
            result.is_ok(),
            "Failed to read solution with minimal metadata: {:?}",
            result.err()
        );

        let solutions = result.unwrap();
        assert_eq!(solutions.len(), 1);
        assert_eq!(solutions[0].total_points(), 4);
        assert_eq!(solutions[0].rho[0], 1.0);
        assert_eq!(solutions[0].rhoe[0], 5.0);

        assert!(solutions[0].gamma.is_some());
        assert_eq!(solutions[0].gamma.as_ref().unwrap()[0], 6.0);

        Ok(())
    }

    #[test]
    fn test_parse_metadata_full() {
        // Test parsing full metadata with all fields
        let mut buffer = Vec::new();
        let values: Vec<f32> = vec![
            1.2,   // REFMACH
            5.0,   // ALPHA
            1e6,   // REY
            0.5,   // TIME
            1.4,   // GAMINF
            0.0,   // BETA
            288.0, // TINF
            0.0,   // IGAM
            500.0, // HTINF
            100.0, // HT1
            200.0, // HT2
            287.0, // RGAS[0]
            0.5,   // FSMACH
            300.0, // TVREF
            50.0,  // DTVREF
        ];

        for val in &values {
            buffer.extend_from_slice(&(*val).to_le_bytes());
        }

        let metadata = parse_metadata(&buffer, ByteOrder::LittleEndian);

        assert_eq!(metadata.refmach, Some(1.2));
        assert_eq!(metadata.alpha, Some(5.0));
        assert_eq!(metadata.rey, Some(1e6));
        assert_eq!(metadata.time, Some(0.5));
        assert_eq!(metadata.gaminf, Some(1.4));
        assert_eq!(metadata.beta, Some(0.0));
        assert_eq!(metadata.tinf, Some(288.0));
        assert_eq!(metadata.igam, Some(0.0));
        assert_eq!(metadata.htinf, Some(500.0));
        assert_eq!(metadata.ht1, Some(100.0));
        assert_eq!(metadata.ht2, Some(200.0));
        assert_eq!(metadata.rgas, Some(vec![287.0]));
        assert_eq!(metadata.fsmach, Some(0.5));
        assert_eq!(metadata.tvref, Some(300.0));
        assert_eq!(metadata.dtvref, Some(50.0));
    }

    #[test]
    fn test_parse_metadata_minimal() {
        // Test parsing minimal metadata (only first 4 fields)
        let mut buffer = Vec::new();
        let values: Vec<f32> = vec![
            1.5,  // REFMACH
            10.0, // ALPHA
            5e5,  // REY
            1.0,  // TIME
        ];

        for val in &values {
            buffer.extend_from_slice(&(*val).to_le_bytes());
        }

        let metadata = parse_metadata(&buffer, ByteOrder::LittleEndian);

        assert_eq!(metadata.refmach, Some(1.5));
        assert_eq!(metadata.alpha, Some(10.0));
        assert_eq!(metadata.rey, Some(5e5));
        assert_eq!(metadata.time, Some(1.0));
        assert_eq!(metadata.gaminf, None);
        assert_eq!(metadata.beta, None);
        assert_eq!(metadata.tinf, None);
        assert_eq!(metadata.igam, None);
        assert_eq!(metadata.htinf, None);
        assert_eq!(metadata.ht1, None);
        assert_eq!(metadata.ht2, None);
        assert_eq!(metadata.rgas, None);
        assert_eq!(metadata.fsmach, None);
        assert_eq!(metadata.tvref, None);
        assert_eq!(metadata.dtvref, None);
    }

    #[test]
    fn test_parse_metadata_with_multiple_rgas() {
        // Test parsing metadata with multiple RGAS values
        let mut buffer = Vec::new();
        let values: Vec<f32> = vec![
            1.0,   // REFMACH
            0.0,   // ALPHA
            1e7,   // REY
            0.0,   // TIME
            1.4,   // GAMINF
            0.0,   // BETA
            300.0, // TINF
            0.0,   // IGAM
            600.0, // HTINF
            50.0,  // HT1
            100.0, // HT2
            287.0, // RGAS[0]
            1.0,   // RGAS[1]
            2.0,   // RGAS[2]
            0.6,   // FSMACH
            280.0, // TVREF
            10.0,  // DTVREF
        ];

        for val in &values {
            buffer.extend_from_slice(&(*val).to_le_bytes());
        }

        let metadata = parse_metadata(&buffer, ByteOrder::LittleEndian);

        assert_eq!(metadata.rgas, Some(vec![287.0, 1.0, 2.0]));
        assert_eq!(metadata.fsmach, Some(0.6));
        assert_eq!(metadata.tvref, Some(280.0));
        assert_eq!(metadata.dtvref, Some(10.0));
    }

    #[test]
    fn test_parse_metadata_big_endian() {
        // Test parsing metadata with big-endian byte order
        let mut buffer = Vec::new();
        let values: Vec<f32> = vec![
            1.2, // REFMACH
            5.0, // ALPHA
            1e6, // REY
            0.5, // TIME
        ];

        for val in &values {
            buffer.extend_from_slice(&(*val).to_be_bytes());
        }

        let metadata = parse_metadata(&buffer, ByteOrder::BigEndian);

        assert_eq!(metadata.refmach, Some(1.2));
        assert_eq!(metadata.alpha, Some(5.0));
        assert_eq!(metadata.rey, Some(1e6));
        assert_eq!(metadata.time, Some(0.5));
    }

    #[test]
    fn test_read_plot3d_solution_with_metadata() -> io::Result<()> {
        // Test reading solution with full metadata parsed
        let mut temp_file = NamedTempFile::new()?;

        // Record 1: num_grids
        temp_file.write_all(&4i32.to_le_bytes())?;
        temp_file.write_all(&1i32.to_le_bytes())?;
        temp_file.write_all(&4i32.to_le_bytes())?;

        // Record 2: dimensions + NQ + NQC
        temp_file.write_all(&20i32.to_le_bytes())?;
        temp_file.write_all(&2i32.to_le_bytes())?; // i = 2
        temp_file.write_all(&2i32.to_le_bytes())?; // j = 2
        temp_file.write_all(&1i32.to_le_bytes())?; // k = 1
        temp_file.write_all(&6i32.to_le_bytes())?; // NQ = 6
        temp_file.write_all(&0i32.to_le_bytes())?; // NQC = 0
        temp_file.write_all(&20i32.to_le_bytes())?;

        // Record 3: Full metadata (15 values = 60 bytes)
        temp_file.write_all(&60i32.to_le_bytes())?;
        let metadata_values: Vec<f32> = vec![
            1.2,   // REFMACH
            5.0,   // ALPHA
            1e6,   // REY
            0.5,   // TIME
            1.4,   // GAMINF
            0.0,   // BETA
            288.0, // TINF
            0.0,   // IGAM
            500.0, // HTINF
            100.0, // HT1
            200.0, // HT2
            287.0, // RGAS[0]
            0.5,   // FSMACH
            300.0, // TVREF
            50.0,  // DTVREF
        ];
        for val in &metadata_values {
            temp_file.write_all(&(*val).to_le_bytes())?;
        }
        temp_file.write_all(&60i32.to_le_bytes())?;

        // Record 4: Q data for 4 points, 6 variables
        let total_points = 4;
        temp_file.write_all(&96i32.to_le_bytes())?; // 24 floats * 4 bytes

        for var in 0..6 {
            for _pt in 0..total_points {
                temp_file.write_all(&((var + 1) as f32).to_le_bytes())?;
            }
        }
        temp_file.write_all(&96i32.to_le_bytes())?;

        temp_file.flush()?;

        let result = read_plot3d_solution(temp_file.path());
        assert!(
            result.is_ok(),
            "Failed to read solution: {:?}",
            result.err()
        );

        let solutions = result.unwrap();
        assert_eq!(solutions.len(), 1);

        let sol = &solutions[0];
        assert_eq!(sol.total_points(), 4);

        // Check metadata was parsed correctly
        assert!(sol.metadata.is_some());
        let meta = sol.metadata.as_ref().unwrap();
        assert_eq!(meta.refmach, Some(1.2));
        assert_eq!(meta.alpha, Some(5.0));
        assert_eq!(meta.rey, Some(1e6));
        assert_eq!(meta.time, Some(0.5));
        assert_eq!(meta.gaminf, Some(1.4));
        assert_eq!(meta.rgas, Some(vec![287.0]));
        assert_eq!(meta.fsmach, Some(0.5));
        assert_eq!(meta.tvref, Some(300.0));
        assert_eq!(meta.dtvref, Some(50.0));

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

    #[test]
    fn test_read_plot3d_solution_binary_f32_precision() -> io::Result<()> {
        // Create a binary PLOT3D solution file with f32 precision
        let mut temp_file = NamedTempFile::new()?;

        // Record 1: num_grids
        temp_file.write_all(&4i32.to_le_bytes())?;
        temp_file.write_all(&1i32.to_le_bytes())?;
        temp_file.write_all(&4i32.to_le_bytes())?;

        // Record 2: dimensions + NQ + NQC (5 integers = 20 bytes)
        temp_file.write_all(&20i32.to_le_bytes())?;
        temp_file.write_all(&2i32.to_le_bytes())?; // i = 2
        temp_file.write_all(&1i32.to_le_bytes())?; // j = 1
        temp_file.write_all(&1i32.to_le_bytes())?; // k = 1
        temp_file.write_all(&5i32.to_le_bytes())?; // NQ = 5 (no gamma)
        temp_file.write_all(&0i32.to_le_bytes())?; // NQC = 0
        temp_file.write_all(&20i32.to_le_bytes())?;

        // Record 3: Metadata record (small)
        temp_file.write_all(&64i32.to_le_bytes())?;
        for _ in 0..16 {
            temp_file.write_all(&0.0f32.to_le_bytes())?;
        }
        temp_file.write_all(&64i32.to_le_bytes())?;

        // Record 4: Q data (5 variables * 2 points * 4 bytes = 40 bytes for f32)
        temp_file.write_all(&40i32.to_le_bytes())?;
        for i in 0..10 {
            temp_file.write_all(&(i as f32).to_le_bytes())?;
        }
        temp_file.write_all(&40i32.to_le_bytes())?;

        temp_file.flush()?;

        let result = read_plot3d_solution(temp_file.path());
        assert!(
            result.is_ok(),
            "Failed to read f32 solution: {:?}",
            result.err()
        );

        // Check that metadata was set correctly
        let metadata = get_last_solution_metadata();
        assert!(metadata.is_some(), "Solution metadata should be set");
        let meta = metadata.unwrap();
        assert_eq!(meta.format, "binary");
        assert_eq!(meta.precision, "f32");
        assert_eq!(meta.byte_order, "Little-Endian");

        Ok(())
    }

    #[test]
    fn test_read_plot3d_solution_binary_f64_precision() -> io::Result<()> {
        // Create a binary PLOT3D solution file with f64 precision (double)
        let mut temp_file = NamedTempFile::new()?;

        // Record 1: num_grids
        temp_file.write_all(&4i32.to_le_bytes())?;
        temp_file.write_all(&1i32.to_le_bytes())?;
        temp_file.write_all(&4i32.to_le_bytes())?;

        // Record 2: dimensions + NQ + NQC (5 integers = 20 bytes)
        temp_file.write_all(&20i32.to_le_bytes())?;
        temp_file.write_all(&2i32.to_le_bytes())?; // i = 2
        temp_file.write_all(&1i32.to_le_bytes())?; // j = 1
        temp_file.write_all(&1i32.to_le_bytes())?; // k = 1
        temp_file.write_all(&5i32.to_le_bytes())?; // NQ = 5
        temp_file.write_all(&0i32.to_le_bytes())?; // NQC = 0
        temp_file.write_all(&20i32.to_le_bytes())?;

        // Record 3: Metadata record (small)
        temp_file.write_all(&64i32.to_le_bytes())?;
        for _ in 0..16 {
            temp_file.write_all(&0.0f32.to_le_bytes())?;
        }
        temp_file.write_all(&64i32.to_le_bytes())?;

        // Record 4: Q data (5 variables * 2 points * 8 bytes = 80 bytes for f64)
        temp_file.write_all(&80i32.to_le_bytes())?;
        for i in 0..10 {
            temp_file.write_all(&(i as f64).to_le_bytes())?;
        }
        temp_file.write_all(&80i32.to_le_bytes())?;

        temp_file.flush()?;

        let result = read_plot3d_solution(temp_file.path());
        assert!(
            result.is_ok(),
            "Failed to read f64 solution: {:?}",
            result.err()
        );

        // Check that metadata was set correctly
        let metadata = get_last_solution_metadata();
        assert!(metadata.is_some(), "Solution metadata should be set");
        let meta = metadata.unwrap();
        assert_eq!(meta.format, "binary");
        assert_eq!(meta.precision, "f64");
        assert_eq!(meta.byte_order, "Little-Endian");

        Ok(())
    }

    #[test]
    fn test_read_plot3d_solution_ascii_metadata() -> io::Result<()> {
        // Create ASCII solution file
        let mut temp_file = NamedTempFile::new()?;
        temp_file.write_all(b"1\n")?; // num_grids
        temp_file.write_all(b"2 1 1\n")?; // i j k
        temp_file.write_all(b"5 0\n")?; // NQ NQC

        // Metadata line (5 floats)
        temp_file.write_all(b"0.0 0.0 0.0 0.0 0.0\n")?;

        // Q data (5 variables * 2 points = 10 values)
        for i in 0..10 {
            temp_file.write_all(format!("{}.0 ", i).as_bytes())?;
        }
        temp_file.write_all(b"\n")?;

        temp_file.flush()?;

        let result = read_plot3d_solution_ascii(temp_file.path());
        assert!(
            result.is_ok(),
            "Failed to read ASCII solution: {:?}",
            result.err()
        );

        // Check that metadata was set correctly
        let metadata = get_last_solution_metadata();
        assert!(metadata.is_some(), "Solution metadata should be set");
        let meta = metadata.unwrap();
        assert_eq!(meta.format, "ASCII");
        assert_eq!(meta.precision, "f32");
        assert_eq!(meta.byte_order, "N/A");

        Ok(())
    }
}

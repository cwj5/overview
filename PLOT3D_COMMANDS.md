# PLOT3D Command Reference

This document catalogs the command options and features from NASA's original PLOT3D visualization software (Walatka et al., 1990). This serves as a reference for feature implementation in overview.

## Reference
NASA Technical Memorandum 101067: "PLOT3D User's Manual"  
https://ntrs.nasa.gov/api/citations/19900013774/downloads/19900013774.pdf

---

## Command Categories

### 1. Display Commands

Commands that control what type of visualization is shown:

#### `GRID` - Grid Display
- Display computational mesh structure
- Show grid lines (I, J, K constant lines)
- Wireframe representation of computational domain
- Options for grid line density/spacing

#### `CONT` - Contour Plots
- Generate contour lines of scalar quantities
- 2D contours on surfaces or planes
- Contour line labeling
- Customizable contour levels (automatic or manual)
- Flood-filled contours (colored regions between levels)

#### `VECT` - Vector Plots
- Visualize vector fields (velocity, momentum, etc.)
- Arrow glyphs showing magnitude and direction
- Vector scaling controls
- Vector density/sampling options
- Color by vector magnitude

#### `PART` - Particle Traces
- Release massless particles in flow field
- Streamline tracing (forward/backward)
- Pathlines and streaklines
- Particle injection controls (rake, point, surface)
- Time integration parameters

#### `SURF` - Surface Plots
- 3D surface rendering of grids
- Elevation/carpet plots
- Color mapping to scalar values
- Surface normal computation
- Hidden surface removal

#### `RAKE` - Particle/Vector Rakes
- Linear arrangement of particles or vectors
- Rake positioning in 3D space
- Number and spacing of particles/vectors
- Multiple rake support

### 2. View Control Commands

Commands for camera and viewport manipulation:

#### `VIEW` - Viewing Parameters
- Azimuth angle (rotation around vertical axis)
- Elevation angle (rotation from horizontal plane)
- Zoom/magnification factor
- Eye distance from target
- Look-at point (view center)
- Up vector orientation

#### `PERS` - Perspective Mode
- Toggle perspective vs orthographic projection
- Field of view angle for perspective
- Perspective depth cues

#### `ROTA` - Rotation Controls
- Interactive rotation about axes
- Rotation center point
- Incremental rotation angles
- Continuous rotation/animation

#### `SCAL` - Scaling Options
- Uniform scaling (overall size)
- Non-uniform scaling (X, Y, Z independent)
- Automatic fit-to-window
- Aspect ratio controls

#### `WIND` - Window/Viewport
- Viewport dimensions and position
- Multiple viewport support (split screen)
- Window borders and titles
- Aspect ratio locking

### 3. Variable Selection Commands

Commands for choosing what data to visualize:

#### `VARS` - Variable Selection
Select from flow variables:
- **Primitive Variables:**
  - Density (ρ)
  - Velocity components (u, v, w)
  - Pressure (p)
  - Temperature (T)
  - Energy (e)

- **Derived Variables:**
  - Mach number
  - Total pressure
  - Total temperature
  - Pressure coefficient (Cp)
  - Velocity magnitude
  - Speed of sound

#### `FUNC` - Function Definition
- Create custom functions from primitive variables
- Mathematical operations: +, -, *, /, ^
- Trigonometric functions
- Conditional expressions
- Spatial derivatives
- Access to all 74 built-in functions (see Section 7)

### 4. Grid Operations Commands

Commands for manipulating and selecting grid regions:

#### `ZONE` - Zone/Block Selection
- Select which computational blocks to display
- Multi-block CFD support
- Zone visibility toggles
- Zone highlighting and labeling

#### `PLAN` - Plane Extraction
- Extract 2D planes from 3D grids
- I=constant, J=constant, K=constant planes
- Multiple plane selection
- Plane positioning (index value)
- Interpolated planes between grid lines

#### `CLIP` - Clipping Planes
- Define arbitrary clipping planes
- Boolean operations (union, intersection)
- Clip away portions of domain
- Inside/outside selection
- Clipping plane visualization

#### `BLAN` - Blanking Operations
- Use IBLANK arrays from grid files
- Manual blanking regions
- Blanking by value ranges
- Boolean blanking operations
- Blanking visualization (ghosting)

### 5. Output and Animation Commands

#### `HARD` - Hardcopy Output
- Generate output files for printing
- PostScript format
- Vector vs raster output
- Resolution settings
- Color vs black & white

#### `FRAM` - Frame Control
- Animation frame sequencing
- Frame rate control
- Frame capture for movie creation
- Keyframe animation
- Time stepping through unsteady data

#### `DUMP` - Data Output
- Export processed data
- Extract plane data
- Save function evaluations
- Export in various formats
- Coordinate data output

### 6. Color and Appearance Commands

#### `COLOR` - Color Mapping
- Color scheme selection (rainbow, grayscale, etc.)
- Color bar/legend display
- Value range mapping (min/max)
- Discrete vs continuous colors
- Color inversion

#### `LIGHT` - Lighting Control
- Light source positioning
- Ambient, diffuse, specular components
- Multiple light sources
- Shadows (optional)
- Shininess and material properties

#### `LINE` - Line Attributes
- Line width/thickness
- Line style (solid, dashed, dotted)
- Line color
- Anti-aliasing options

### 7. Built-in Functions (74 Total)

The original PLOT3D included 74 built-in functions for data analysis. Categories include:

#### **Coordinate Functions (Functions 1-9)**
- Cartesian coordinates (X, Y, Z)
- Cylindrical coordinates (R, θ, Z)
- Spherical coordinates
- Grid indices (I, J, K)

#### **Flow Variables (Functions 10-20)**
- Density (RHO)
- Velocity components (U, V, W)
- Velocity magnitude (VMAG)
- Pressure (P)
- Temperature (T)
- Mach number (MACH)
- Total pressure (PT)
- Total temperature (TT)
- Sound speed (A)

#### **Pressure Coefficients (Functions 21-25)**
- Static pressure coefficient (CP)
- Total pressure coefficient (CPT)
- Compressibility corrections
- Dynamic pressure

#### **Momentum and Energy (Functions 26-30)**
- Momentum components (ρu, ρv, ρw)
- Total energy (ρe)
- Kinetic energy
- Internal energy
- Enthalpy (H), total enthalpy (HT)

#### **Derivatives - First Order (Functions 31-45)**
Partial derivatives with respect to X, Y, Z:
- ∂u/∂x, ∂u/∂y, ∂u/∂z
- ∂v/∂x, ∂v/∂y, ∂v/∂z  
- ∂w/∂x, ∂w/∂y, ∂w/∂z
- ∂p/∂x, ∂p/∂y, ∂p/∂z
- ∂T/∂x, ∂T/∂y, ∂T/∂z

#### **Vorticity (Functions 46-48)**
- Vorticity components (ωx, ωy, ωz)
- Vorticity magnitude (|ω|)
- Helicity

#### **Strain Rate (Functions 49-54)**
- Strain rate tensor components
- Shear strain rates
- Volumetric strain rate
- Strain rate magnitude

#### **Turbulence Quantities (Functions 55-60)**
- Turbulent kinetic energy
- Dissipation rate
- Eddy viscosity
- y+ (wall distance)
- Skin friction coefficient

#### **Geometric Functions (Functions 61-68)**
- Grid metrics (∂x/∂ξ, ∂y/∂η, ∂z/∂ζ, etc.)
- Jacobian determinant
- Cell volumes
- Surface areas
- Grid quality metrics

#### **Special Functions (Functions 69-74)**
- Reynolds number (local)
- Peclet number
- Entropy
- Total enthalpy
- Stream function
- Vector potential components

### 8. Analysis and Probe Commands

#### `PROB` - Point Probe
- Query values at specific points
- Interpolation in cell
- Display all variables at point
- Coordinate readout

#### `INTE` - Integration
- Surface integrals (flux, force, moment)
- Volume integrals
- Line integrals
- Circulation and vorticity flux

#### `EXTR` - Extrema Finding
- Find minimum/maximum values
- Locate critical points
- Saddle points
- Stagnation points

### 9. Interactive Controls

#### `MOUS` - Mouse Controls
- Interactive rotation (click-drag)
- Pan (shift-drag)
- Zoom (wheel or drag)
- Point selection
- Region selection

#### `MENU` - Menu System
- Hierarchical command menus
- Keyboard shortcuts
- Command history
- Macro recording

#### `HELP` - Online Help
- Command documentation
- Function reference
- Example workflows
- Context-sensitive help

### 10. Command Files and Batch Processing

One of PLOT3D's powerful features was the ability to save command sequences to files for later replay and batch processing. This enables:

#### **Command File Format**
```
# PLOT3D Command File
# Comments start with #

# File loading
LOAD GRID grid.xyz
LOAD SOLUTION solution.q

# Visualization setup
ZONE 1
VARS MACH
COLOR RAINBOW

# View configuration
VIEW AZIMUTH 45 ELEVATION 30 ZOOM 1.5

# Display options
GRID ON
SURF ON
CONT ON LEVELS 10

# Output
HARD OUTPUT image.ps COLOR

# Animation frames (if time-varying data)
FRAM 1 TO 100 STEP 1
  DUMP FRAME frame_%04d.tiff
END
```

#### **Use Cases**
1. **Reproducibility**: Save exact visualization settings used in analysis
2. **Batch Processing**: Run same visualization on multiple datasets
3. **Collaboration**: Share visualization configurations with team members
4. **Documentation**: Record analysis workflows for reports/papers
5. **Automation**: Process unsteady solutions through time automatically
6. **Quality Control**: Consistent visualizations across multiple runs

#### **Command File Features**
- Comments (lines starting with #)
- Variable substitution (for file paths, parameters)
- Conditional execution (IF/ELSE blocks)
- Looping (FOR loops over time steps or zones)
- Conditional logic
- Error handling and logging directives

#### **Recording vs. Manual Creation**
- **Record Mode**: UI tracks all user actions and saves as command file
- **Manual Creation**: Directly write command files in text editor
- **Hybrid**: Edit and replay, modifying commands as needed

#### **Batch Processing Workflow**
```
overview --batch command_file.plot3d
```
- Execute commands sequentially without UI
- Generate all output files
- Log all operations
- Return exit code indicating success/failure

---

## Implementation Priority for overview

Based on the original PLOT3D capabilities, here's a suggested implementation priority:

### High Priority (Core Visualization)
1. ✅ `GRID` - Grid display (wireframe) - **Completed**
2. ✅ `VIEW` - View controls - **Completed** (orbit controls)
3. ✅ `SURF` - Surface rendering - **Completed** (shaded mode)
4. ✅ `ZONE` - Multi-grid selection - **Completed**
5. ✅ `VARS` - Variable selection - **Completed** (basic)
6. ✅ `COLOR` - Color mapping - **Completed** (multiple schemes)
7. `PLAN` - Plane extraction
8. `CONT` - Contour plots

### Medium Priority (Enhanced Features)
9. `VECT` - Vector plots
10. `PART` - Particle traces/streamlines
11. `CLIP` - Clipping planes
12. `BLAN` - Blanking support
13. `FUNC` - Custom functions (74 built-in functions)
14. `LIGHT` - Lighting controls
15. `PROB` - Point probe/query

### Lower Priority (Advanced Features)
16. `RAKE` - Particle rakes
17. `FRAM` - Animation/time stepping
18. `INTE` - Integration tools
19. `EXTR` - Extrema finding
20. `HARD` - Export/hardcopy
21. `DUMP` - Data export

---

## Notes on Modern Implementation

While replicating PLOT3D's capabilities, overview can modernize the interface:

- **Original PLOT3D**: Text-based command-line interface
- **overview Approach**: GUI with panels, buttons, and interactive 3D controls

- **Original PLOT3D**: Sequential command execution
- **overview Approach**: Real-time interactive manipulation

- **Original PLOT3D**: Limited graphics hardware (1990s)
- **overview Approach**: Modern GPU acceleration via WebGL/Three.js

The goal is to preserve PLOT3D's powerful analysis capabilities while providing a modern, intuitive user experience.
---

## Tauri Command API Reference

This section documents the Tauri backend commands exposed to the frontend for PLOT3D grid and solution visualization.

### Core Commands

#### `convert_grid_to_mesh_by_id`

Converts a PLOT3D grid block to Three.js mesh geometry with optional solution field coloring.

**Signature:**
```typescript
invoke('convert_grid_to_mesh_by_id', {
  gridId: number,
  blockId: number,
  fieldIndex?: number,
  iblankFilterMode?: 'vertex' | 'cell'
})
```

**Parameters:**
- `gridId` (number, required): The ID of the loaded PLOT3D grid file
- `blockId` (number, required): The block index within the multi-block grid
- `fieldIndex` (number, optional): Solution field index for coloring (omit for wireframe)
- `iblankFilterMode` (string, optional): IBLANK filtering mode, defaults to `'vertex'`
  - `'vertex'`: Filter out individual vertices/points where IBLANK indicates hidden (value 0 or -1 when fringe points are hidden)
  - `'cell'`: Filter out entire quads/cells where any corner vertex is hidden

**Returns:**
```typescript
{
  vertices: number[],        // Flat array [x,y,z, x,y,z, ...]
  indices: number[],         // Triangle indices for wireframe edges
  triangle_indices: number[], // Triangle indices for solid surfaces
  normals?: number[],        // Normals for lighting (if available)
  colors?: number[]          // RGB colors per vertex (if fieldIndex provided)
}
```

**Behavior:**
- **Vertex Mode**: Creates an IndexMap for non-hidden vertices, remaps all geometry to compacted indices. Efficient for datasets with sparse blanking.
- **Cell Mode**: Keeps all vertices in original positions, rejects quads where any corner has IBLANK indicating hidden. Preserves vertex alignment for simpler color mapping.

---

#### `compute_solution_colors`

Computes per-vertex color arrays for a previously loaded mesh based on normalized solution field values.

**Signature:**
```typescript
invoke('compute_solution_colors', {
  gridId: number,
  blockId: number,
  fieldIndex: number,
  showFringePoints?: boolean,
  iblankFilterMode?: 'vertex' | 'cell'
})
```

**Parameters:**
- `gridId` (number, required): The ID of the loaded PLOT3D grid file
- `blockId` (number, required): The block index within the multi-block grid
- `fieldIndex` (number, required): Solution field index (0-based) to visualize
- `showFringePoints` (boolean, optional): Whether to show fringe points (IBLANK = -1), defaults to `false`
- `iblankFilterMode` (string, optional): IBLANK filtering mode, defaults to `'vertex'`

**Returns:**
```typescript
{
  colors: number[],  // Flat RGB array [r,g,b, r,g,b, ...] normalized to [0,1]
  min: number,       // Global minimum field value across all grids
  max: number        // Global maximum field value across all grids
}
```

**Behavior:**
- Uses global min/max normalization across all loaded solution files for consistent color mapping
- **Vertex Mode**: Filters colors to exclude hidden vertices (respecting `showFringePoints` flag), resulting in compacted arrays matching vertex-mode mesh
- **Cell Mode**: Compacts mesh vertices and colors to only those referenced by geometry indices, then updates indices to reference compacted array

**Note:** Color array length must match the mesh vertices array length. The alignment logic is mode-specific:
- Vertex mode: filter_vertex_mode_surface_colors (removes entries for hidden points)
- Cell mode: compact_mesh_and_colors_to_used_vertices (remaps to only used vertices)

---

#### `compute_solution_colors_sliced`

Computes colors for index-plane slices (I/J/K constant planes) extracted from the grid.

**Signature:**
```typescript
invoke('compute_solution_colors_sliced', {
  gridId: number,
  blockId: number,
  axis: 'I' | 'J' | 'K',
  index: number,
  fieldIndex: number,
  showFringePoints?: boolean,
  iblankFilterMode?: 'vertex' | 'cell'
})
```

**Parameters:**
- `gridId` (number, required): The ID of the loaded PLOT3D grid file
- `blockId` (number, required): The block index within the multi-block grid
- `axis` (string, required): Slice axis - one of `'I'`, `'J'`, or `'K'`
- `index` (number, required): Grid index along the specified axis (0-based)
- `fieldIndex` (number, required): Solution field index to visualize
- `showFringePoints` (boolean, optional): Whether to show fringe points, defaults to `false`
- `iblankFilterMode` (string, optional): IBLANK filtering mode, defaults to `'vertex'`

**Returns:**
```typescript
{
  colors: number[],  // RGB color array aligned with slice mesh vertices
  min: number,       // Global minimum field value
  max: number        // Global maximum field value
}
```

**Behavior:**
- Extracts 2D plane from 3D grid at specified index
- Applies IBLANK filtering according to mode:
  - **Vertex Mode**: Skips hidden vertices when building slice mesh
  - **Cell Mode**: Rejects slice quads with any hidden corner
- Color array is aligned with the resulting slice geometry

---

#### `compute_solution_colors_arbitrary_plane`

Computes colors for arbitrary plane slices defined by point and normal vector.

**Signature:**
```typescript
invoke('compute_solution_colors_arbitrary_plane', {
  gridId: number,
  blockId: number,
  planePoint: [number, number, number],
  planeNormal: [number, number, number],
  fieldIndex: number,
  showFringePoints?: boolean,
  iblankFilterMode?: 'vertex' | 'cell'
})
```

**Parameters:**
- `gridId` (number, required): The ID of the loaded PLOT3D grid file
- `blockId` (number, required): The block index within the multi-block grid
- `planePoint` (array, required): 3D point [x, y, z] that the plane passes through
- `planeNormal` (array, required): Normal vector [nx, ny, nz] defining plane orientation
- `fieldIndex` (number, required): Solution field index to visualize
- `showFringePoints` (boolean, optional): Whether to show fringe points, defaults to `false`
- `iblankFilterMode` (string, optional): IBLANK filtering mode, defaults to `'vertex'`

**Returns:**
```typescript
{
  colors: number[],  // RGB color array for arbitrary plane intersection
  min: number,       // Global minimum field value
  max: number        // Global maximum field value
}
```

**Behavior:**
- Intersects arbitrary plane with grid cells to create slice geometry
- **Vertex Mode**: Standard intersection logic with per-vertex IBLANK checks
- **Cell Mode**: Early rejection of cells where all 8 corners are hidden before computing intersection geometry
- Interpolates solution field values at intersection points
- Returns colors aligned with the generated intersection mesh

---

#### `slice_arbitrary_plane_by_id`

Generates mesh geometry for an arbitrary plane slice through the grid.

**Signature:**
```typescript
invoke('slice_arbitrary_plane_by_id', {
  gridId: number,
  blockId: number,
  planePoint: [number, number, number],
  planeNormal: [number, number, number],
  iblankFilterMode?: 'vertex' | 'cell'
})
```

**Parameters:**
- `gridId` (number, required): The ID of the loaded PLOT3D grid file
- `blockId` (number, required): The block index within the multi-block grid
- `planePoint` (array, required): 3D point [x, y, z] on the cutting plane
- `planeNormal` (array, required): Normal vector [nx, ny, nz] of the cutting plane
- `iblankFilterMode` (string, optional): IBLANK filtering mode, defaults to `'vertex'`

**Returns:**
```typescript
{
  vertices: number[],        // Intersection vertices [x,y,z, ...]
  indices: number[],         // Line indices for wireframe
  triangle_indices: number[], // Triangle indices for surface
  normals?: number[]         // Normals (typically aligned with plane normal)
}
```

**Behavior:**
- Computes intersection of plane with each grid cell
- **Vertex Mode**: Standard Marching Cubes-style intersection
- **Cell Mode**: Skips cells where all 8 corners are hidden (optimization + filtering)
- Generates triangle mesh from intersection polygons

---

### IBLANK Filtering Modes

The `iblank_filter_mode` parameter controls how IBLANK arrays (blanking arrays in PLOT3D grid files) are used to filter geometry:

#### **Vertex Mode** (`'vertex'`)
- **Filtering Strategy**: Individual vertices/points are filtered based on their IBLANK value
- **Hidden Criteria**: A point is hidden if:
  - `IBLANK[idx] == 0` (always hidden)
  - `IBLANK[idx] == -1` AND `show_fringe_points == false` (fringe point, conditionally hidden)
- **Geometry Impact**: 
  - Vertices are compacted to only non-hidden points
  - Indices are remapped to the compacted vertex array
  - Quads/cells are included if they have at least some visible vertices
- **Use Cases**: 
  - Fine-grained control over point visibility
  - Efficient for datasets with many isolated blanked points
  - Preserves partial cells at blanking boundaries

#### **Cell Mode** (`'cell'`)
- **Filtering Strategy**: Entire quads/cells are filtered based on corner vertices
- **Hidden Criteria**: A quad/cell is rejected if ANY corner vertex is hidden
- **Geometry Impact**:
  - All vertices initially retained at original indices
  - Quads with any hidden corner are discarded
  - For surfaces: Final compaction removes unused vertices and remaps indices
  - For arbitrary planes: Early rejection before expensive intersection tests
- **Use Cases**:
  - Conservative blanking (avoid partial cells)
  - Cleaner boundaries at blanking interfaces
  - Performance optimization for arbitrary planes (early cell rejection)

#### **Mode Selection Guidelines**
- **Vertex Mode**: Default for most visualizations, shows maximum detail
- **Cell Mode**: For cleaner visualization when partial cells at boundaries are undesirable, or when performance is critical for arbitrary plane slicing

#### **Implementation Details**
- Both modes respect the `show_fringe_points` flag for IBLANK = -1 handling
- Global min/max normalization ensures consistent colors across modes
- Color arrays are always length-aligned with final mesh vertex arrays
- Mode parameter is optional; defaults to `'vertex'` for backward compatibility

---

### Supporting Commands

#### `load_plot3d_grid`
Loads a PLOT3D grid file (XYZ format, structured or unstructured).

**Signature:**
```typescript
invoke('load_plot3d_grid', { filePath: string })
```

**Returns:** `{ gridId: number }`

---

#### `load_plot3d_solution`
Loads a PLOT3D solution file (Q format) associated with a grid.

**Signature:**
```typescript
invoke('load_plot3d_solution', { 
  gridId: number, 
  filePath: string 
})
```

**Returns:** `{ success: boolean }`

---

#### `get_grid_blocks`
Retrieves metadata about blocks in a loaded grid.

**Signature:**
```typescript
invoke('get_grid_blocks', { gridId: number })
```

**Returns:**
```typescript
{
  blocks: Array<{
    id: number,
    dimensions: [number, number, number]  // [imax, jmax, kmax]
  }>
}
```

---

#### `get_solution_fields`
Lists available solution fields and their metadata.

**Signature:**
```typescript
invoke('get_solution_fields', { gridId: number })
```

**Returns:**
```typescript
{
  fields: Array<{
    index: number,
    name: string,
    description?: string
  }>
}
```
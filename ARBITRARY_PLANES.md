# Arbitrary Cutting Planes - User Guide

## Overview
The arbitrary cutting plane feature allows you to slice through 3D PLOT3D grids using custom-defined planes, not limited to the I, J, or K grid planes.

## How to Use

### 1. Adding an Arbitrary Plane
1. Load a PLOT3D grid file
2. In the grid list sidebar, click the "+ Add Slice" button for a grid
3. Select "Arbitrary" from the plane type dropdown
4. The UI will show additional controls for plane definition

### 2. Defining the Plane
An arbitrary plane is defined by:
- **Point**: A 3D coordinate (X, Y, Z) that lies on the plane
- **Normal**: A vector (X, Y, Z) perpendicular to the plane

The normal vector is automatically normalized, so you can use any non-zero values.

### 3. Example Configurations

#### Horizontal Plane
- Point: `[0, 0, 5]` (any point at your desired Z height)
- Normal: `[0, 0, 1]` (pointing upward)
- Result: Horizontal slice at Z=5

#### Vertical Plane (X-Z)
- Point: `[0, 3, 0]` (any point at your desired Y position)
- Normal: `[0, 1, 0]` (pointing along Y axis)
- Result: Vertical slice perpendicular to Y axis

#### Diagonal Plane
- Point: `[0, 0, 0]` (origin)
- Normal: `[1, 1, 1]` (45° to all axes)
- Result: Diagonal slice through the grid

#### Tilted Plane
- Point: `[5, 5, 5]` (center of grid)
- Normal: `[1, 0.5, 0]` (tilted from vertical)
- Result: Plane tilted from the X-Y plane

## Technical Details

### Algorithm
The implementation now uses a welded polygon reconstruction algorithm:
1. Each hexahedral cell is intersected with the plane using all 12 cell edges, including coplanar edge/vertex handling
2. Intersection points in each cell are deduplicated and ordered in the plane basis (2D angle sort)
3. Cell polygons are triangulated with consistent orientation against the plane normal
4. Vertices are globally welded across all intersected cells in a grid using a scale-aware tolerance
5. Duplicate coplanar triangles are removed to avoid overlapping internal faces
6. The final result is a per-grid triangulated surface mesh with consistent winding and no seam cracks at shared cell boundaries

Important: slicing is computed independently for each grid. If multiple grids are loaded, the result is multiple unconnected surfaces (one per grid), and no geometry is stitched between grids.

### Solution Field Interpolation
For solution field visualization on arbitrary planes:
1. Each intersection point is tracked with its source hexahedral cell indices (i, j, k)
2. Linear interpolation weights are computed for the 8 corners of each cell
3. Solution values (density, momentum, energy, etc.) are interpolated using these weights
4. Scalar fields (pressure, temperature, Mach number, etc.) are computed from the interpolated conservative variables
5. Colors are mapped from the scalar field values using the selected color scheme

### Performance
- Complexity: O(n) where n is the number of cells
- Suitable for grids up to ~1M cells
- Future: GPU-accelerated version for larger datasets

### Current Features
- **Solution field coloring**: Arbitrary planes now support density/pressure/velocity/energy field visualization with interpolated colors
- **Triangulated surface**: The plane intersection creates a welded triangulated mesh for solid rendering
- **Interpolation tracking**: Each vertex on the plane tracks its position within the source hexahedral cell for accurate solution value interpolation
- **Per-grid separation**: Outputs remain disconnected between different grids by design
- **Consistent winding**: Triangle winding is aligned to the selected plane normal for stable shading
- **Coplanar support**: Faces/edges/vertices that lie on the plane are included in the intersection

### Limitations (Current Version)
- **Manual input only**: No interactive drag handles or rotation widgets (planned for future)
- **No caching**: Plane is recomputed on every parameter change
- **Tolerance-dependent welding**: Very ill-conditioned grids can still be sensitive to floating-point tolerance selection

## Troubleshooting

### "No intersection found between plane and grid"
- Verify your plane point is near the grid bounds
- Check that the plane actually passes through the grid volume
- Try adjusting the point or normal to ensure intersection
- If your plane is exactly aligned with a grid face or edge, the algorithm now robustly detects these cases. If you still see this error, check for floating-point precision issues and try slightly adjusting the plane parameters.

### "Plane normal vector has zero magnitude"
- All normal components are zero
- Set at least one component to a non-zero value (e.g., `[0, 0, 1]`)

### Unexpected Results
- Verify your coordinate system matches the grid's coordinate system
- Check the grid bounds in the metadata
- Ensure the normal vector points in the intended direction
- If slices appear incomplete or missing, ensure your plane is not exactly coincident with a grid boundary; the algorithm now handles these cases, but floating-point precision may affect results.

## Future Enhancements
- [ ] Interactive 3D widgets for plane manipulation (drag/rotate)
- [ ] Preset planes (XY at Z, YZ at X, etc.)
- [ ] Multiple arbitrary planes with boolean operations
- [ ] Plane equation display (ax + by + cz = d)
- [ ] Visual plane indicator in the 3D viewport
- [ ] Caching of computed plane slices for better performance

## API Reference

### Tauri Commands

#### Basic Plane Slicing (Geometry Only)
```rust
slice_arbitrary_plane(
    grid: Plot3DGrid,
    plane_point: [f32; 3],
    plane_normal: [f32; 3],
) -> Result<MeshGeometry, String>
```
Creates a basic plane intersection mesh without solution coloring.

#### Plane Slicing with Solution Colors
```rust
compute_solution_colors_arbitrary_plane(
    grid: Plot3DGrid,
    grid_index: usize,
    field: String,
    color_scheme: String,
    plane_point: [f32; 3],
    plane_normal: [f32; 3],
) -> Result<MeshGeometry, String>
```
Creates a plane intersection mesh with interpolated solution field colors.
- `field`: "density", "pressure", "velocity", "mach", "temperature", or "energy"
- `color_scheme`: "viridis", "plasma", "inferno", "magma", "turbo", "jet", "rainbow", or "grayscale"

*Note: Both algorithms use triangle-based intersection and robustly handle faces/edges aligned with the plane.*

### TypeScript Interface
```typescript
interface GridSlice {
    id: string;
    plane: 'I' | 'J' | 'K' | 'ARBITRARY';
    index: number;              // For I/J/K only
    planePoint?: [number, number, number];   // For ARBITRARY
    planeNormal?: [number, number, number];  // For ARBITRARY
}
```

## See Also
- [ROADMAP.md](ROADMAP.md) - Development roadmap
- [PLOT3D_COMMANDS.md](PLOT3D_COMMANDS.md) - PLOT3D format reference

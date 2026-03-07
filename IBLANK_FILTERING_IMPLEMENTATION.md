# IBLANK Filtering Implementation Plan

**Document Date**: March 5, 2026  
**Status**: ✅ Implementation Verified (Code + Tests + Visual)  
**Related Issue**: Toggle "Ignore IBLANK" was previously non-functional

---

## Table of Contents
1. [Executive Summary](#executive-summary)
2. [Current Understanding](#current-understanding)
3. [Requirements](#requirements)
4. [Technical Details](#technical-details)
5. [Implementation Steps](#implementation-steps)
6. [Verification & Testing](#verification--testing)
7. [Known Limitations / Not in Scope](#known-limitations--not-in-scope)
8. [Future Enhancements: Dual IBLANK Filter Modes](#future-enhancements)

---

## Executive Summary

IBLANK filtering is now implemented end-to-end in frontend and Rust backend for index slices and arbitrary planes. The "Ignore IBLANK" toggle now affects generated geometry as intended, with backend safety normalization to prevent invalid flag combinations from hiding fringe points when IBLANK is ignored.

**Goal**: Ensure when the toggle is OFF (default), points with IBLANK=0 are excluded from visualization, creating physical holes in mesh geometry; when toggle is ON, IBLANK blanking is ignored.

**Approach**: Filter vertices and indices at the mesh geometry generation stage (output to frontend), not at the grid cache level. Grid cache remains unchanged in Rust backend.

**UI Decision Implemented (March 6, 2026)**:
- When `Ignore IBLANK` is ON (`respectIblank=false`), `Show Fringe Points` is disabled (greyed out) in the menu
- The prior fringe preference is preserved (no forced check/uncheck)
- Backend normalizes flags so fringe points remain visible whenever `respectIblank=false`

---

## Current Understanding

### IBLANK Data Definition

IBLANK is a per-point array in PLOT3D grid files that indicates a point's status:

| Value | Meaning | Visualization Behavior |
|-------|---------|------------------------|
| `0` | Hole point (blanked/off) | Hide when `respect_iblank=true`; show when `ignore_iblank=true` |
| `1` | Normal visible point | Always show |
| `-n` | Fringe point connecting to grid `n` | Always show (treat as normal point) |
| `2` | Solid wall boundary point | Always show (treat as normal point) |

### IBLANK Data Flow (Current Implementation)

#### 1. **Data Loading** ([src-tauri/src/plot3d.rs](src-tauri/src/plot3d.rs#L2160-L2315))
- Loaded from PLOT3D grid files in 4 format variations (f32/f64 coordinates, i32/byte IBLANK)
- Stored in `Plot3DGrid` struct field: `pub iblank: Option<Vec<i32>>`
- Metadata flag: `pub has_iblank: bool` indicates presence of IBLANK data

#### 2. **Grid Slicing** ([src-tauri/src/plot3d.rs](src-tauri/src/plot3d.rs#L157-L280))
- Index slices: `slice_grid()` extracts 2D subgrids (constant I/J/K planes)
  - Preserves IBLANK data: `let mut iblank_vec = self.iblank.as_ref().map(|_| Vec::with_capacity(...))`
  - Returns `Plot3DGrid` with sliced IBLANK array
- Arbitrary plane slices: `slice_arbitrary_plane_with_solution()` includes IBLANK checks
  - Uses `cell_has_blanked_corner()` to skip hexahedral cells with any blanked corner

#### 3. **Mesh Geometry Generation** ([src-tauri/src/plot3d.rs](src-tauri/src/plot3d.rs#L916-L1000))
- **Index slices**: `to_mesh_surface_geometry_decimated()` converts 2D grid to wireframe/solid geometry
  - Has `is_blanked()` helper but doesn't properly filter vertices
  - Currently skips quad indices if any corner has `iblank[idx] == 0`
  - **Issue**: Still includes blanked vertices in output array
  
- **Arbitrary planes**: Skips entire cells but doesn't filter individual vertices

#### 4. **Frontend Toggle** ([src/App.tsx](src/App.tsx#L93), [src/App.tsx](src/App.tsx#L237-L243))
- Toggle state: `const [ignoreIblank, setIgnoreIblank] = useState(false)`
- Menu item: `CheckMenuItem` with id `"ignore-iblank"`
- Enabled only when grid has IBLANK data
- UI Text: "Ignore IBLANK"

#### 5. **Frontend Slice Request** ([src/components/Viewer3D.tsx](src/components/Viewer3D.tsx#L599), [src/components/Viewer3D.tsx](src/components/Viewer3D.tsx#L679-L712))
- Index slices: `invoke('convert_grid_to_mesh', { grid: slicedGrid, respect_iblank: !ignoreIblank })`
- Arbitrary planes: `invoke('slice_arbitrary_plane_by_id', { ..., respect_iblank: !ignoreIblank })`
- **Note**: Parameter inverted—UI text is "Ignore IBLANK" but backend receives `respect_iblank` boolean

---

## Requirements

### Functional Requirements

1. **IBLANK Value Interpretation**
   - `iblank[idx] == 0`: Hide (hole point) when `respect_iblank=true`
   - `iblank[idx] == 1`: Always show (normal point)
   - `iblank[idx] < 0` (fringe points): Always show, treat as normal points
   - `iblank[idx] == 2` (wall boundaries): Always show, never filter
   
2. **Vertex-Level Filtering**
   - Skip individual blanked vertices from mesh geometry output
   - This creates **physical holes** in the mesh at blanked point locations
   - **Not** global cell/quad removal
   - All edges/quads referencing blanked vertices should also be excluded from indices
   
3. **Mesh Output Consistency**
   - Grid cache in Rust remains unchanged (no indexing issues)
   - Filtering only affects `MeshGeometry` arrays sent to frontend:
     - `vertices: Vec<f32>` (x, y, z interleaved)
     - `line_indices: Vec<u32>` or `triangle_indices: Vec<u32>`
   - Must maintain vertex-to-index mapping integrity

4. **Solution Data & Coloring**
   - When a vertex is blanked, its position AND solution value should be excluded
   - Blanked locations should be empty/transparent (no color data)
   - Solution data must only map to non-blanked vertices

5. **Slice Type Coverage**
   - **Index slices (I/J/K planes)**: Skip blanked vertices when generating mesh
   - **Arbitrary planes**: Skip blanked vertices from intersection computation
   - **Decimation interaction**: Decimation applied first, then blanking filter (creates gaps)

### Non-Functional Requirements

1. **Correctness over Performance**: Prioritize correct visualization first; optimize later if needed
2. **Edge Cases**:
   - Empty slices (all vertices blanked): Show nothing at all
   - Decimated slices with blanking: Create gaps where blanked vertices would be
   - Multiple grids with different IBLANK configurations: Handle independently

---

## Technical Details

### Key Code Locations

#### Rust Backend
- **Grid loading**: [src-tauri/src/plot3d.rs#L2160-L2315](src-tauri/src/plot3d.rs#L2160-L2315)
- **Grid slicing**: [src-tauri/src/plot3d.rs#L157-L280](src-tauri/src/plot3d.rs#L157-L280)
- **Arbitrary plane slicing**: [src-tauri/src/plot3d.rs#L288-L725](src-tauri/src/plot3d.rs#L288-L725)
- **Index slice mesh generation**: [src-tauri/src/plot3d.rs#L916-L1000](src-tauri/src/plot3d.rs#L916-L1000)
- **Cell blanking check**: [src-tauri/src/plot3d.rs#L385-L404](src-tauri/src/plot3d.rs#L385-L404)
- **Solution mapping**: [src-tauri/src/lib.rs#L1006-L1100](src-tauri/src/lib.rs#L1006-L1100) (index slices) and [src-tauri/src/lib.rs#L1183-L1350](src-tauri/src/lib.rs#L1183-L1350) (arbitrary planes)

#### Frontend
- **Toggle definition**: [src/App.tsx#L93](src/App.tsx#L93)
- **Toggle UI**: [src/App.tsx#L237-L243](src/App.tsx#L237-L243)
- **Slice invocation**: [src/components/Viewer3D.tsx#L599](src/components/Viewer3D.tsx#L599), [src/components/Viewer3D.tsx#L679-L712](src/components/Viewer3D.tsx#L679-L712)

### Current is_blanked() Logic (Index Slices)

```rust
let is_blanked = |idx: usize| -> bool {
    if respect_iblank {
        if let Some(ref iblank) = self.iblank {
            return iblank[idx] == 0;  // Only exclude if == 0
        }
    }
    false
};
```

**Issue**: Only checks `iblank[idx] == 0`. Doesn't distinguish IBLANK=1 (always show), IBLANK=2 (always show), or IBLANK<0 (always show). Currently treats negative values as "not blanked" but logic is implicit.

### Current Vertex Generation (Index Slices)

```rust
// All vertices extracted from decimated grid (including blanked ones)
let mut vertices = Vec::with_capacity(i_decimated * j_decimated * 3);
for j_step in 0..j_decimated {
    let j_idx = (j_step * decimation).min(j - 1);
    for i_step in 0..i_decimated {
        let i_idx = (i_step * decimation).min(i - 1);
        let idx = Self::linear_index(i_idx, j_idx, k_idx, i, j);
        vertices.push(self.x_coords[idx]);
        vertices.push(self.y_coords[idx]);
        vertices.push(self.z_coords[idx]);
    }
}

// Indices selectively generated (skips quads with blanked corners)
// BUT: vertices array still contains blanked points!
```

**Problem**: Blanked vertices are still in the output mesh vertices array. Only the quad indices that reference them are skipped, leaving orphaned vertices.

### Arbitrary Plane Blanking Check

```rust
let cell_has_blanked_corner = |i_idx: usize, j_idx: usize, k_idx: usize| -> bool {
    if !respect_iblank { return false; }
    let Some(iblank) = self.iblank.as_ref() else { return false; };
    
    let corners = [/* 8 hexahedral cell corners */];
    corners.iter().any(|&idx| iblank[idx] == 0)  // Only checks for == 0
};
```

**Issue**: Same as index slices—only excludes IBLANK=0, doesn't explicitly handle other values. Works correctly by accident but should be explicit.

---

## Implementation Steps

### Phase 1: Verify Flag Propagation ✅ COMPLETE

**Goal**: Confirm the `respect_iblank` flag is properly passed through the entire chain and that toggling triggers regeneration.

**Status**: ✅ VERIFIED - Frontend toggle state properly propagates through slice system, re-renders on toggle change

1. ✅ **Check frontend toggle dependency**
   - Verified `ignoreIblank` state change in `App.tsx` triggers slice regeneration in `Viewer3D.tsx`
   - Confirmed `sliceKey` dependency includes `ignoreIblank` for regeneration
   - Fixed parameter naming: `respect_iblank` (snake_case) → `respectIblank` (camelCase) in all 8 invoke calls

2. ✅ **Verify Tauri commands receive the parameter**
   - Confirmed `convert_grid_to_mesh` command accepts `respectIblank: bool`
   - Confirmed `slice_arbitrary_plane_by_id` command accepts `respectIblank: bool`
   - Verified command handlers pass parameter to Rust functions

3. ✅ **Test with debug output**
   - Frontend build successful with corrected camelCase parameter names
   - Slice regeneration confirmed (toggles trigger re-render)

### Phase 2: Fix Index Slice Vertex/Index Generation ✅ COMPLETE

**Goal**: Skip blanked vertices entirely from mesh output while maintaining index integrity.

**Status**: ✅ IMPLEMENTED - Vertex mapping and filtering applied to index slices

**File**: [src-tauri/src/plot3d.rs](src-tauri/src/plot3d.rs)

**Function**: `to_mesh_surface_geometry_decimated()`

**Changes Implemented**:

1. ✅ **Update is_blanked() helper** to be explicit about all IBLANK values
   - Implemented predicate: `iblank[idx] == 0` when `respect_iblank=true`
   - Correctly handles: IBLANK=1 (always show), IBLANK=2 (always show), IBLANK<0 (always show)

2. ✅ **Build vertex mapping** before generating indices
   - Created `vertex_index_map: Vec<Option<u32>>` mapping old_vertex_idx → new_vertex_idx
   - Tracks which decimated vertices are non-blanked

3. ✅ **Generate filtered vertices array**
   - Iterates through decimated grid points, skipping blanked ones
   - Output vertices array contains only non-blanked coordinates
   - Mapping tracks grid index to output mesh index

4. ✅ **Generate filtered indices**
   - Checks if any quad corner is blanked
   - Skips entire quad if any corner blanked (no triangle indices generated)
   - Uses NEW vertex indices from mapping for all quads (ensures no orphaned vertices)

### Phase 3: Fix Arbitrary Plane Vertex Filtering ✅ COMPLETE

**Goal**: Skip vertices from blanked cells during intersection computation.

**Status**: ✅ IMPLEMENTED - Edge-level blanking checks applied to arbitrary planes

**File**: [src-tauri/src/plot3d.rs](src-tauri/src/plot3d.rs)

**Function**: `slice_arbitrary_plane_with_solution()`

**Changes Implemented**:

1. ✅ **Update cell_has_blanked_corner()** to be explicit
   - Implemented: `iblank[idx] == 0` check for 8 hexahedral corners
   - Correctly treats IBLANK<0 and IBLANK≥1 as always-visible

2. ✅ **Skip blanked cells early**
   - Blanked cells skipped during plane intersection computation
   - Prevents adding any vertices/edges from blanked regions

3. ✅ **Verify edge-plane intersections**
   - Added `corner_blanked` array to track blanked corners per cell
   - Skip edges where either endpoint is blanked
   - Only add corner vertices from non-blanked corners

### Phase 4: Verify Solution Coloring ✅ COMPLETE

**Goal**: Ensure solution data is only applied to non-blanked vertices.

**Status**: ✅ IMPLEMENTED - Color filtering applied to both slice types

**Files**: [src-tauri/src/lib.rs](src-tauri/src/lib.rs)

**Functions**: 
- `compute_solution_colors()` (full-surface slices)
- `compute_solution_colors_sliced()` (index slices)

**Changes Implemented**:

1. ✅ **Index slices** ([src-tauri/src/lib.rs](src-tauri/src/lib.rs))
   - Solution coloring filtered to match non-blanked vertices only
   - RGB color array length matches filtered vertex count
   - No color data for blanked regions

2. ✅ **Arbitrary planes** ([src-tauri/src/lib.rs](src-tauri/src/lib.rs))
   - Solution interpolation only includes non-blanked cell contributions
   - Colors applied only to visible vertices
   - Blanked regions have no color data

### Phase 5: Frontend Integration ✅ COMPLETE

**Goal**: Ensure frontend correctly transmits IBLANK filtering state to backend.

**Status**: ✅ IMPLEMENTED - Parameter naming corrected, frontend rebuilt

**File**: [src/components/Viewer3D.tsx](src/components/Viewer3D.tsx)

**Changes Implemented**:

1. ✅ **Fixed parameter naming** (critical fix)
   - Changed `respect_iblank` (snake_case) → `respectIblank` (camelCase)
   - Updated 8 invoke calls: lines 599, 617, 626, 679, 700, 712, 811, 821
   - Reason: Tauri JSON serialization requires camelCase for proper deserialization
   - Backend now receives toggle value correctly

2. ✅ **Verified slice regeneration**
   - Toggle triggers re-rendering with corrected parameter transmission
   - Frontend dependencies properly configured for toggle-based updates

3. ✅ **Frontend build**
   - Vite build successful: 0 TypeScript errors, 623 modules
   - Ready for integration testing

**File**: [src-tauri/src/plot3d.rs](src-tauri/src/plot3d.rs)

**Function**: `slice_arbitrary_plane_with_solution()`

**Changes**:

1. **Update cell_has_blanked_corner()** to be explicit:
   ```rust
   let cell_has_blanked_corner = |i_idx: usize, j_idx: usize, k_idx: usize| -> bool {
       if !respect_iblank { return false; }
       let Some(iblank) = self.iblank.as_ref() else { return false; };
       
       let corners = [/* 8 hexahedral cell corners */];
       // Only skip if corner has IBLANK == 0
       corners.iter().any(|&idx| iblank[idx] == 0)
   };
   ```

2. **Skip blanked cells early**:
   - Continue to next cell if `cell_has_blanked_corner()` returns true
   - Prevents adding any vertices/edges from blanked regions

3. **Verify edge-plane intersections**:
   - Only add intersection points from non-blanked edges
   - Only add corner vertices from non-blanked corners

### Phase 4: Verify Solution Coloring

**Goal**: Ensure solution data is only applied to non-blanked vertices.

**Files**: [src-tauri/src/lib.rs](src-tauri/src/lib.rs)

**Functions**: 
- `slice_grid_with_solution_by_id()` (index slices)
- `slice_arbitrary_plane_with_solution()` (arbitrary planes)

**Changes**:

1. **Index slices** ([src-tauri/src/lib.rs#L1006-L1100](src-tauri/src/lib.rs#L1006-L1100)):
   - After slicing, solution values are extracted at slice points
   - Verify solution mapping only includes non-blanked vertices
   - Check that solution coloring aligns with filtered mesh geometry

2. **Arbitrary planes** ([src-tauri/src/lib.rs#L1183-L1350](src-tauri/src/lib.rs#L1183-L1350)):
   - Solution is interpolated using `VertexCellData` (cell index + barycentric weights)
   - Verify that blanked cells don't contribute to interpolation data
   - Ensure colors are only applied to visible vertices

### Phase 5: Frontend Integration (if needed)

**File**: [src/components/Viewer3D.tsx](src/components/Viewer3D.tsx)

**Changes** (likely none needed, but verify):
1. Confirm `sliceKey` dependency includes `ignoreIblank`
2. Verify slices regenerate (not cached) when toggle changes
3. Check that solution coloring respects filtered geometry

---

## Verification & Testing

### Test Cases

1. **Manual Visual Testing**
   - Load a PLOT3D file with IBLANK data
   - Toggle "Ignore IBLANK" checkbox on and off
   - Observe holes appear/disappear at IBLANK=0 locations
   - Verify IBLANK=1, =2, <0 points are always visible

2. **Index Slice Testing**
   - Create slices along I, J, and K planes
   - Toggle IBLANK filtering and verify holes appear/disappear
   - Test with decimation enabled (gaps should persist)
   - Test with solution coloring (blanked regions should be transparent)

3. **Arbitrary Plane Testing**
   - Create arbitrary plane slices
   - Toggle IBLANK filtering and verify blanked regions excluded
   - Verify smooth rendering at non-blanked regions
   - Test solution coloring alignment

4. **Edge Cases**
   - Empty slices (all vertices blanked): Should show nothing
   - Partially blanked slices: Should show holes
   - Multiple grids with different IBLANK configurations: Handle independently
   - Grids without IBLANK data: Unaffected (toggle disabled)

### Success Criteria

- [x] Toggle "Ignore IBLANK" changes what's displayed (parameter transmission fixed)
- [x] IBLANK=0 points disappear when toggle is OFF
- [x] IBLANK=0 points reappear when toggle is ON
- [x] IBLANK=1, =2, <0 points always displayed (implemented in filtering logic)
- [x] Holes are at individual blanked vertices, not entire regions (vertex-level filtering implemented)
- [x] Solution coloring matches filtered geometry (color filtering implemented)
- [x] No visual artifacts at blanking boundaries
- [x] Decimation still works correctly with blanking (decimation-before-blanking order preserved)
- [x] `Show Fringe Points` is disabled when `Ignore IBLANK` is ON (preference preserved)
- [x] Backend enforces defensive normalization for `respect_iblank=false`
- [x] Added targeted Rust tests for decimated mesh filtering and arbitrary-plane IBLANK behavior

### Closeout Verification Run Log (March 6, 2026)

#### Automated Checks

| Check | Status | Evidence |
|------|--------|----------|
| Rust normalization tests (`iblank_flag_tests`) | ✅ PASS | `cargo test iblank_flag_tests -- --nocapture` |
| Decimated mesh blanked-vertex filtering test | ✅ PASS | `cargo test test_surface_mesh_decimated_filters_blanked_vertices -- --nocapture` |
| Arbitrary-plane respect_iblank behavior test | ✅ PASS | `cargo test test_arbitrary_plane_respect_iblank_controls_blanked_cells -- --nocapture` |
| Frontend build/typecheck | ✅ PASS | `npm run build` |

#### Manual Visual Checks (UI)

| Check | Status | Notes |
|------|--------|-------|
| Toggle `Ignore IBLANK` OFF/ON and verify visible holes for `IBLANK=0` | ✅ PASS | Verified in interactive app session with IBLANK dataset |
| Verify `Show Fringe Points` is disabled when `Ignore IBLANK` is ON | ✅ PASS | Control is greyed out while ignore mode is active |
| Verify fringe preference is preserved after re-enabling IBLANK respect | ✅ PASS | Previous fringe selection restored after toggle sequence |
| Verify index slices (I/J/K) with decimation still produce expected gaps | ✅ PASS | Visual gaps align with expected blanked regions |
| Verify arbitrary-plane rendering/correlation with solution coloring | ✅ PASS | Plane rendering and coloring remained consistent |

#### Result Summary

- Code changes, automated regression checks, and manual UI validation for implemented IBLANK behavior are all passing.
- Final sign-off is complete for this implementation scope.

---

## Known Limitations / Not in Scope

The following behaviors were explicitly excluded from **Phase 1** (vertex-mode only) implementation scope:

### 1. Cell-Skipping Mode (NOW PLANNED, See "Future Enhancements")

**Phase 1 Behavior**: Only vertex-level filtering available; holes appear at individual blanked points.

**Phase 2 (Planned)**: Dual-mode toggle will allow users to switch to **Cell mode**, which skips entire quads/cells or hexahedral cells with blanked corners.

**See**: [Future Enhancements → Dual IBLANK Filter Modes](#in-progress-dual-iblank-filter-modes-vertex-vs-cell) for full design and locked decisions.

**Timeline**: ~13–20 hours estimated effort; prioritize based on user feedback on Phase 1.

### 2. Arbitrary-Plane Strict "Point-Hole" Mode

**Current Behavior**: Arbitrary-plane slicing uses **cell-level exclusion**—entire hexahedral cells are skipped if any corner vertex has `IBLANK=0`.

**Not Implemented**: Strict **vertex-level filtering** for arbitrary planes that would preserve the exact hole geometry by excluding only the specific edges/triangles touching blanked points.

**Rationale**: 
- Cell-level exclusion is simpler and provides consistent results
- For most use cases, excluding cells with blanked corners adequately represents the IBLANK filtering intent
- Implementing precise point-hole mode for arbitrary planes would require complex edge-intersection logic to handle partial cell visibility
- Index slices (I/J/K planes) use vertex-level filtering, which is sufficient for most blanking visualization needs

**Impact**: Arbitrary-plane slices may show slightly larger holes than the strict point-level blanking would suggest, as entire cells adjacent to blanked points are excluded.

### 2. Independent Fringe Control While Respecting IBLANK

**Current Behavior**: When `Ignore IBLANK` is ON, the `Show Fringe Points` control is disabled (greyed out) and fringe points are always visible.

**Not Implemented**: Allowing users to toggle fringe visibility independently while still respecting IBLANK for normal/blanked points.

**Rationale**:
- Fringe points (negative IBLANK values) typically indicate overset grid boundaries and are almost always desired visible
- Simplifies UI logic and reduces potential for confusing state combinations
- Backend normalization ensures fringe points are never accidentally hidden when IBLANK filtering is disabled

**Impact**: Users cannot hide fringe points while using `Ignore IBLANK` mode. If fringe-hiding is needed, users must re-enable IBLANK respect.

### 3. Mixed Filtering Modes (Per-Grid IBLANK Behavior)

**Current Behavior**: The `Ignore IBLANK` toggle is global—it applies uniformly to all loaded grids.

**Not Implemented**: Per-grid IBLANK control allowing different grids to use different filtering behaviors simultaneously.

**Rationale**:
- Adds significant UI complexity (per-grid menus or grid-specific toggles)
- Most multi-grid datasets have consistent IBLANK usage across all grids
- Global toggle provides clear, predictable behavior

**Impact**: Users cannot selectively ignore IBLANK for some grids while respecting it for others in the same visualization session.

---

## Future Enhancements

### IN PROGRESS: Dual IBLANK Filter Modes (Vertex vs Cell)

Add an optional toggle to switch between two filtering modes:

1. **Vertex-skipping mode** (default): Skip individual blanked vertices → creates holes at point locations
2. **Cell-skipping mode**: Skip entire quads/cells or hexahedral cells with any blanked/hidden corner → removes entire regions

**Locked Design Decisions (March 6, 2026)**:

#### Scope & Coverage
- **Apply to all geometry paths**: index slices (I/J/K), full surfaces, arbitrary planes
- **Rationale**: Consistent user experience; same mode behavior regardless of visualization path

#### Mode Transport & API Shape
- **Use string enum** (`"vertex"` | `"cell"`) rather than boolean flag
- **Required parameter** (not optional/fallback)
- **Pattern**: Place mode alongside existing `respect_iblank` and `show_fringe_points` in all Tauri commands
- **Rust parsing**: Implement `from_str()` pattern (similar to `ColorScheme` in [src-tauri/src/solution.rs](src-tauri/src/solution.rs))
- **Rationale**: Explicit, future-proof for additional modes; no silent defaults

#### Color Array Handling in Cell Mode
- **Remapped to surviving vertices only** (smaller array, no waste)
- **Implementation**: After mesh generation drops quads/triangles, reindex color array to only participating vertices
- **Rationale**: Cleaner semantics; avoids unused color entries for dropped geometry

#### Fringe Points + Cell Mode Interaction
- **In Cell mode**: Fringe points (iblank < 0) treated like blanked points when `show_fringe_points=false`
  - If a cell has a fringe corner AND `show_fringe_points=false`, reject the cell
  - Fringe does NOT automatically protect adjacent geometry in Cell mode
- **Rationale**: Consistent with fringe visibility toggle; respects user's intent

#### Decimation Order in Cell Mode
- **Evaluation**: Blanking/fringe evaluated on **full grid first**, then decimation subsampling applied
- **Effect**: Gaps appear where blanked quads would be (matches current vertex-mode behavior)
- **Rationale**: Consistent behavior across both modes; decimation is resolution reduction, not a separate filtering stage

#### Menu UI Design
- **Single SelectItem or Submenu** (not two CheckMenuItems)
- **Placement**: View menu, mutually exclusive mode selector (e.g., "IBLANK Filter: Vertex" vs "IBLANK Filter: Cell")
- **Visual**: Radio-button semantics; only one mode active at a time
- **Rationale**: More professional UX; clearer mutual exclusion vs two toggles

#### Empty Slice Handling
- **Return empty MeshGeometry silently**: 0 vertices, 0 triangles, no error
- **Behavior**: Consistent with vertex mode when all points are blanked
- **Rationale**: Avoids error-handling complexity; frontend already handles empty meshes

#### Test Strategy
- **Update existing tests** to pass default mode parameter (`"vertex"`)
- **Add parallel mode-specific tests** for `"cell"` behavior
- **Single parameterized test suite** (source of truth for both modes)
- **Test coverage**:
  - Vertex mode: maintains current behavior
  - Cell mode: quads/cells rejected when any corner blanked
  - Arbitrary plane: strict 8-corner reject in cell mode
  - Decimation interaction: gaps preserved across both modes
  - Color alignment: colors remapped to surviving vertices in cell mode
  - Fringe interaction: fringe corners treated as blanked in cell mode when `show_fringe_points=false`

#### Arbitrary-Plane Cell-Mode Rule
- **Strict 8-corner rejection**: Skip entire hexahedral cell if any of 8 corners is blanked (iblank=0) or hidden fringe (iblank<0 with show_fringe_points=false)
- **Why strict over intersection-only**: Consistent whole-cell semantics; avoids partial-slice slivers; easier to reason about
- **Implementation**: Early exit before plane-cell intersection computation if cell fails blanking check

---

### Implementation Architecture for Dual Modes

#### Frontend Changes
- **File**: [src/App.tsx](src/App.tsx)
  - Add state: `const [iblankFilterMode, setIblankFilterMode] = useState<'vertex' | 'cell'>('vertex')`
  - Extend menu setup to add mode selector (SelectItem or Submenu pattern)
  
- **File**: [src/components/Viewer3D.tsx](src/components/Viewer3D.tsx)
  - Add to props: `iblankFilterMode: 'vertex' | 'cell'`
  - Update all 6 invoke calls to include mode parameter:
    - `convert_grid_to_mesh` (line 743)
    - `convert_grid_to_mesh_by_id` (line 882)
    - `compute_solution_colors` (line 882)
    - `compute_solution_colors_sliced` (line 782)
    - `slice_arbitrary_plane_by_id` (line 695)
    - `compute_solution_colors_arbitrary_plane` (line 660)

#### Rust Backend Changes
- **File**: [src-tauri/src/lib.rs](src-tauri/src/lib.rs)
  - Add `IblankFilterMode` enum (newtype wrapper around String or dedicated enum)
  - Implement `from_str()` with validation (reject invalid mode strings)
  - Update `normalize_iblank_flags()` to normalize three axes (respect, fringe, mode)
  - Update all 6 Tauri command signatures:
    - `convert_grid_to_mesh(grid, respect_iblank, show_fringe_points, iblank_filter_mode)`
    - `convert_grid_to_mesh_by_id(gridId, respect_iblank, show_fringe_points, iblank_filter_mode)`
    - `compute_solution_colors(gridId, solutionId, field, colorScheme, respect_iblank, show_fringe_points, iblank_filter_mode, globalMin, globalMax)`
    - `compute_solution_colors_sliced(gridId, solutionId, slicePlane, sliceIndex, field, colorScheme, respect_iblank, show_fringe_points, iblank_filter_mode, globalMin, globalMax)`
    - `slice_arbitrary_plane_by_id(gridId, planePoint, planeNormal, respect_iblank, show_fringe_points, iblank_filter_mode)`
    - `compute_solution_colors_arbitrary_plane(gridId, solutionId, field, colorScheme, planePoint, planeNormal, respect_iblank, show_fringe_points, iblank_filter_mode, globalMin, globalMax)`
  - Pass mode through to geometry generation and color functions

- **File**: [src-tauri/src/plot3d.rs](src-tauri/src/plot3d.rs)
  - Update `to_mesh_surface_geometry_decimated()` signature to include `iblank_filter_mode`
  - Add branching logic:
    - **Vertex mode**: Current behavior (vertex map filtering)
    - **Cell mode**: Keep all decimated vertices, reject quads if any corner blanked/hidden
  - Update `slice_arbitrary_plane_with_solution()` signature to include `iblank_filter_mode`
  - Add branching logic:
    - **Vertex mode**: Current behavior (edge-level filtering)
    - **Cell mode**: Skip cell if any of 8 corners blanked/hidden (before intersection computation)
  - Update helper predicates to account for fringe visibility:
    - `is_blanked()`: Returns `true` if `iblank[idx]==0` (hole) OR (`iblank[idx]<0` AND `!show_fringe_points`)

#### Color Generation Alignment
- **File**: [src-tauri/src/lib.rs](src-tauri/src/lib.rs) (compute_solution_colors* functions)
  - **Vertex mode**: Keep current post-filter approach (colors extracted only for non-blanked vertices)
  - **Cell mode**: After mesh geometry is finalized with dropped triangles, reindex color array to surviving vertices only
  - Ensure color array length always matches final `vertex_count` in returned `MeshGeometry`

#### Test Updates
- **File**: [src-tauri/src/plot3d.rs](src-tauri/src/plot3d.rs)
  - Update existing tests (`test_mesh_geometry_iblank_filtering`, `test_surface_mesh_decimated_filters_blanked_vertices`, `test_arbitrary_plane_respect_iblank_controls_blanked_cells`) to pass `iblank_filter_mode: "vertex"`
  - Add new tests for Cell mode:
    - `test_surface_mesh_cell_mode_rejects_quads_with_blanked_corners`
    - `test_arbitrary_plane_cell_mode_rejects_cells_with_blanked_corners`
    - `test_cell_mode_with_decimation_creates_gaps`
    - `test_cell_mode_fringe_interaction_when_hidden`

- **File**: [src-tauri/src/lib.rs](src-tauri/src/lib.rs)
  - Update normalization tests to include mode validation
  - Add test: `test_mode_parsing_valid_and_invalid_strings`
  - Add test: `test_mode_normalization_with_fringe`

#### Documentation Updates
- **File**: [PLOT3D_COMMANDS.md](PLOT3D_COMMANDS.md)
  - Document all 6 updated Tauri command signatures
  - Add `iblank_filter_mode` parameter description and valid values
  - Explain mode behavior differences for each geometry type

- **File**: [IBLANK_FILTERING_IMPLEMENTATION.md](IBLANK_FILTERING_IMPLEMENTATION.md) (this file)
  - Document the dual-mode design in a new "Dual IBLANK Filter Modes" section
  - Explain mode semantics, differences, and use cases
  - Link to implementation architecture section

---

### Phase 3 Status (March 6, 2026)

**Status**: ✅ COMPLETE - Index Slices, Full Surfaces, and Arbitrary Planes

**Completed**:
- ✅ Public `IblankFilterMode` enum defined in lib.rs with `from_str()` parsing
- ✅ `to_mesh_surface_geometry_decimated()` refactored to dispatcher (routes to mode-specific implementations)
- ✅ `to_mesh_surface_geometry_decimated_vertex_mode()` implemented with full vertex-skipping logic
- ✅ `to_mesh_surface_geometry_decimated_cell_mode()` implemented with cell-rejection logic
- ✅ All 4 mesh function calls in lib commands updated to pass `effective_filter_mode`
- ✅ Tests updated to pass mode parameter and verify both modes work correctly

**Arbitrary Plane Additions Completed**:
1. ✅ Added `iblank_filter_mode` parameter to `slice_arbitrary_plane()` and `slice_arbitrary_plane_with_solution()`
2. ✅ Implemented cell-mode early rejection (skip cell if any of 8 corners blanked/hidden before intersection)
3. ✅ Updated arbitrary plane tests to verify both modes
4. ✅ Updated arbitrary plane command handlers in lib.rs (`slice_arbitrary_plane_by_id`, `compute_solution_colors_arbitrary_plane`)

**Key Files Modified**:
- `/Users/cwj5/software/overview/src-tauri/src/lib.rs` - Command signatures and calls
- `/Users/cwj5/software/overview/src-tauri/src/plot3d.rs` - Dual-mode geometry generation for surfaces

---

### Implementation Checklist

**Phase 1: Frontend State & Menu** ✅ COMPLETE (~2–3 hours)
- [x] Add `iblankFilterMode` state in App.tsx
- [x] Extend View menu with mutually exclusive mode selector
- [x] Add prop to Viewer3DProps
- [x] Thread mode through all 6 invoke calls

**Phase 2: Rust Enums & Command Signatures** ✅ COMPLETE (~1–2 hours)
- [x] Define `IblankFilterMode` enum with `from_str()` in lib.rs
- [x] Update normalize_iblank_flags to handle mode
- [x] Update all 6 command signatures
- [x] Pass mode through to geometry/color functions (parameter accepted, will be used in Phase 3)

**Phase 3: Mesh Geometry Branching** ✅ COMPLETE (~4–6 hours)
- [x] Refactored `to_mesh_surface_geometry_decimated()` with mode branching dispatcher
- [x] Created `to_mesh_surface_geometry_decimated_vertex_mode()` stub for existing implementation
- [x] Implemented `to_mesh_surface_geometry_decimated_cell_mode()` function
- [x] Updated all 4 command calls in lib.rs to pass effective_filter_mode
- [x] Refactor `slice_arbitrary_plane_with_solution()`/`slice_arbitrary_plane()` with mode parameter
- [x] Implement Cell-mode 8-corner rejection logic for arbitrary planes
- [x] Update helper predicates for fringe visibility in arbitrary-plane path

**Phase 4: Color Alignment** ✅ COMPLETE (~2–3 hours)
- [x] Implement color remapping logic for Cell mode (compact to surviving vertices only)
- [x] Update compute_solution_colors* surface functions to align colors with surviving vertices
- [x] Validate color array length in tests (new unit tests for vertex fringe filtering and cell compaction)

**Phase 5: Test Updates** ✅ COMPLETE (~3–4 hours)
- [x] Updated existing tests to pass default mode
- [x] Added new mode-specific tests (vertex + cell for decimated surfaces)
- [x] Added mode parsing and normalization tests
- [x] Run full test suite: `cargo test` (passes except pre-existing unicode logger test)
- [x] Add arbitrary plane mode tests (including cell-mode reject and fringe interaction)

**Phase 6: Documentation** ✅ COMPLETE (~1–2 hours)
- [x] Update PLOT3D_COMMANDS.md with new command signatures
- [x] Add mode semantics and use-case documentation to IBLANK_FILTERING_IMPLEMENTATION.md
- [x] Document fringe interaction and decimation order

**Total Estimated Effort**: ~13–20 hours (including testing and documentation)

---

### Mode Behavior Reference Table

| Scenario | Vertex Mode | Cell Mode |
|----------|-------------|-----------|
| **Blanked vertex (iblank=0)** | Vertex removed; quads dropped; hole appears | Vertex kept; quad dropped if any corner blanked; hole appears |
| **Fringe vertex (iblank<0) with show_fringe=false** | Vertex removed; quads dropped | Vertex kept; cell dropped if fringe corner participates |
| **Decimated grid** | Decimate → filter vertices | Decimate → evaluate blanking on decimated positions |
| **Color array** | Remapped to non-blanked vertices | Remapped to vertices in surviving triangles |
| **Arbitrary plane all-blanked cell** | Skip edges to blanked corners | Skip entire cell (no intersection attempted) |
| **Empty slice result** | Empty MeshGeometry (0 vertices) | Empty MeshGeometry (0 vertices) |



---

## Notes & Assumptions

- Grid cache in Rust backend remains unchanged; filtering applies only to mesh geometry output
- IBLANK data is optional; code must handle both files with and without IBLANK
- Negative IBLANK values are treated as fringe points connecting to other grids; treated as visible
- IBLANK=2 (wall boundaries) are treated as normal points; always visible
- Decimation is applied before blanking filter (decimated grid → filtered mesh)
- Empty index slices are handled gracefully (show nothing)
- Fully filtered arbitrary-plane intersections may return "No intersection found between plane and grid"
- Performance is secondary to correctness for this implementation

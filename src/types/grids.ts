import type { Plot3DGrid, Plot3DSolution } from "./plot3d";

// Per-grid slice (I/J/K plane at specific index)
export interface GridSlice {
    id: string; // Unique ID for this slice
    plane: 'I' | 'J' | 'K';
    index: number;
}

// Global arbitrary cutting plane (affects all grids)
export interface ArbitrarySlice {
    id: string;
    name: string; // User-friendly name
    planePoint: [number, number, number]; // Point on the plane
    planeNormal: [number, number, number]; // Normal vector (will be normalized)
    enabled: boolean; // Allow toggling without deletion
    applied: boolean; // Only render when applied=true
    applyVersion: number; // Incremented when user clicks Apply
    dirty: boolean; // Parameters changed since last apply
}

export interface GridItem {
    id: string;
    grid?: Plot3DGrid; // Deprecated: Full grid data (for backward compatibility)
    gridCacheId?: string; // New: Grid cache ID from backend
    filePath: string;
    fileName: string;
    gridIndex: number;
    dimensions: { i: number; j: number; k: number }; // Always available
    hasIblank: boolean; // Always available
    color: string;
    visible: boolean;
    solution?: Plot3DSolution; // Deprecated: Full solution data
    solutionCacheId?: string; // New: Solution cache ID from backend
    hasSolution: boolean; // Always available
}

export interface GridFileGroup {
    filePath: string;
    fileName: string;
    grids: GridItem[];
}

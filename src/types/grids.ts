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
    grid: Plot3DGrid;
    filePath: string;
    fileName: string;
    gridIndex: number;
    color: string;
    visible: boolean;
    solution?: Plot3DSolution; // Optional solution data for this grid
}

export interface GridFileGroup {
    filePath: string;
    fileName: string;
    grids: GridItem[];
}

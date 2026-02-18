import type { Plot3DGrid, Plot3DSolution } from "./plot3d";

export interface GridSlice {
    id: string; // Unique ID for this slice
    plane: 'I' | 'J' | 'K';
    index: number;
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

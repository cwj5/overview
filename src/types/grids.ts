import type { Plot3DGrid } from "./plot3d";

export interface GridItem {
    id: string;
    grid: Plot3DGrid;
    filePath: string;
    fileName: string;
    gridIndex: number;
    color: string;
    visible: boolean;
}

export interface GridFileGroup {
    filePath: string;
    fileName: string;
    grids: GridItem[];
}

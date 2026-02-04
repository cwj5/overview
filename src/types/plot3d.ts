export interface Plot3DGrid {
    dimensions: { i: number; j: number; k: number };
    x_coords: number[];
    y_coords: number[];
    z_coords: number[];
    iblank?: number[]; // Optional blanking array (0=blanked, 1=visible)
}

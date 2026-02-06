export interface Plot3DGrid {
    dimensions: { i: number; j: number; k: number };
    x_coords: number[];
    y_coords: number[];
    z_coords: number[];
    iblank?: number[]; // Optional blanking array (0=blanked, 1=visible)
}

export interface Plot3DSolution {
    grid_index: number;
    dimensions: { i: number; j: number; k: number };
    rho: number[];  // Density
    rhou: number[]; // Momentum X
    rhov: number[]; // Momentum Y
    rhow: number[]; // Momentum Z
    rhoe: number[]; // Energy
}

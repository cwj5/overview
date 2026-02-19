/**
 * Utilities for computing derived quantities from PLOT3D solution data
 * PLOT3D solution variables: rho, rhou, rhov, rhow, rhoe
 * (Density, Momentum X, Momentum Y, Momentum Z, Energy)
 */

import type { Plot3DSolution } from "../types/plot3d";
import { DEFAULT_GAMMA } from "./constants";

/**
 * Supported scalar fields for visualization
 */
export type ScalarField = 'none' | 'density' | 'velocity_magnitude' | 'momentum_x' | 'momentum_y' | 'momentum_z' | 'pressure' | 'energy';

export interface ScalarFieldInfo {
    field: ScalarField;
    name: string;
    unit: string;
    description: string;
}

export const SCALAR_FIELDS: ScalarFieldInfo[] = [
    {
        field: 'none',
        name: 'Grid ID',
        unit: '',
        description: 'Color by grid number (no solution visualization)'
    },
    {
        field: 'density',
        name: 'Density',
        unit: 'ρ',
        description: 'Fluid density'
    },
    {
        field: 'pressure',
        name: 'Pressure',
        unit: 'p',
        description: 'Static pressure (computed from energy equation using γ from file)'
    },
    {
        field: 'velocity_magnitude',
        name: 'Velocity Magnitude',
        unit: '|V|',
        description: 'Total velocity magnitude sqrt(u² + v² + w²)'
    },
    {
        field: 'momentum_x',
        name: 'Momentum X',
        unit: 'ρu',
        description: 'X-component of momentum'
    },
    {
        field: 'momentum_y',
        name: 'Momentum Y',
        unit: 'ρv',
        description: 'Y-component of momentum'
    },
    {
        field: 'momentum_z',
        name: 'Momentum Z',
        unit: 'ρw',
        description: 'Z-component of momentum'
    },
    {
        field: 'energy',
        name: 'Total Energy',
        unit: 'ρe',
        description: 'Total energy per unit volume'
    },
];

/**
 * Compute velocity magnitude from momentum components
 */
function computeVelocityMagnitude(rho: number, rhou: number, rhov: number, rhow: number): number {
    if (rho <= 0) return 0;

    const u = rhou / rho;
    const v = rhov / rho;
    const w = rhow / rho;
    return Math.sqrt(u * u + v * v + w * w);
}

/**
 * Compute pressure from conservative variables
 */
function computePressure(
    rho: number,
    rhou: number,
    rhov: number,
    rhow: number,
    rhoe: number,
    gamma: number
): number {
    if (rho <= 0) return 0;

    const u = rhou / rho;
    const v = rhov / rho;
    const w = rhow / rho;
    const kinetic_energy = 0.5 * rho * (u * u + v * v + w * w);
    const internal_energy = rhoe - kinetic_energy;
    return (gamma - 1) * internal_energy;
}

/**
 * Compute a scalar field from solution data
 */
export function computeScalarField(solution: Plot3DSolution, field: ScalarField): Float32Array {
    const totalPoints = solution.rho.length;
    const result = new Float32Array(totalPoints);

    switch (field) {
        case 'none':
            // Return zeros for grid ID mode
            return new Float32Array(totalPoints);

        case 'density':
            return new Float32Array(solution.rho);

        case 'velocity_magnitude':
            for (let i = 0; i < totalPoints; i++) {
                result[i] = computeVelocityMagnitude(
                    solution.rho[i],
                    solution.rhou[i],
                    solution.rhov[i],
                    solution.rhow[i]
                );
            }
            return result;

        case 'pressure':
            for (let i = 0; i < totalPoints; i++) {
                const gamma = solution.gamma ? solution.gamma[i] : DEFAULT_GAMMA;
                result[i] = computePressure(
                    solution.rho[i],
                    solution.rhou[i],
                    solution.rhov[i],
                    solution.rhow[i],
                    solution.rhoe[i],
                    gamma
                );
            }
            return result;

        case 'momentum_x':
            return new Float32Array(solution.rhou);

        case 'momentum_y':
            return new Float32Array(solution.rhov);

        case 'momentum_z':
            return new Float32Array(solution.rhow);

        case 'energy':
            return new Float32Array(solution.rhoe);

        default:
            return new Float32Array(solution.rho);
    }
}

/**
 * Get statistics for a scalar field
 */
export interface FieldStats {
    min: number;
    max: number;
    mean: number;
    stdDev: number;
}

export function getFieldStats(values: Float32Array): FieldStats {
    if (values.length === 0) {
        return { min: 0, max: 0, mean: 0, stdDev: 0 };
    }

    let min = values[0];
    let max = values[0];
    let sum = 0;

    // Find min, max, and sum
    for (let i = 0; i < values.length; i++) {
        const v = values[i];
        if (v < min) min = v;
        if (v > max) max = v;
        sum += v;
    }

    const mean = sum / values.length;

    // Calculate standard deviation
    let sumSquaredDiff = 0;
    for (let i = 0; i < values.length; i++) {
        const diff = values[i] - mean;
        sumSquaredDiff += diff * diff;
    }
    const stdDev = Math.sqrt(sumSquaredDiff / values.length);

    return { min, max, mean, stdDev };
}

/**
 * Get the display name and unit for a scalar field
 */
export function getFieldInfo(field: ScalarField): ScalarFieldInfo {
    const info = SCALAR_FIELDS.find(f => f.field === field);
    return info || SCALAR_FIELDS[0];
}

/**
 * Format a numeric value for display
 */
export function formatValue(value: number, decimals: number = 3): string {
    if (!isFinite(value)) return 'N/A';

    const abs = Math.abs(value);

    if (abs === 0) {
        return '0';
    } else if (abs < 0.001) {
        return value.toExponential(decimals - 1);
    } else if (abs < 1) {
        return value.toFixed(decimals);
    } else if (abs < 1000) {
        return value.toFixed(Math.max(0, decimals - Math.floor(Math.log10(abs)) - 1));
    } else {
        return value.toExponential(decimals - 1);
    }
}

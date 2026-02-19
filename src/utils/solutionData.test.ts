/**
 * Unit tests for solution data utilities
 */

import { describe, it, expect } from 'vitest';
import { computeScalarField, getFieldStats, formatValue, getFieldInfo, SCALAR_FIELDS } from './solutionData';
import type { Plot3DSolution } from '../types/plot3d';

describe('solutionData', () => {
    // Helper to create a simple test solution
    const createTestSolution = (size: number = 4, includeGamma: boolean = false): Plot3DSolution => {
        const solution: Plot3DSolution = {
            grid_index: 0,
            dimensions: { i: 2, j: 2, k: 1 },
            rho: new Array(size).fill(0),
            rhou: new Array(size).fill(0),
            rhov: new Array(size).fill(0),
            rhow: new Array(size).fill(0),
            rhoe: new Array(size).fill(0),
        };

        // Fill with test data
        for (let i = 0; i < size; i++) {
            solution.rho[i] = 1.0 + i * 0.1;      // 1.0, 1.1, 1.2, 1.3
            solution.rhou[i] = 0.5 * solution.rho[i];  // 0.5, 0.55, 0.6, 0.65
            solution.rhov[i] = 0.3 * solution.rho[i];  // 0.3, 0.33, 0.36, 0.39
            solution.rhow[i] = 0.2 * solution.rho[i];  // 0.2, 0.22, 0.24, 0.26
            solution.rhoe[i] = 2.5 * solution.rho[i];  // 2.5, 2.75, 3.0, 3.25
        }

        if (includeGamma) {
            solution.gamma = new Array(size).fill(0).map((_, i) => 1.4 + i * 0.01);
        }

        return solution;
    };

    describe('computeScalarField', () => {
        it('should compute density field', () => {
            const solution = createTestSolution(4);
            const result = computeScalarField(solution, 'density');

            expect(result.length).toBe(4);
            expect(result[0]).toBeCloseTo(1.0);
            expect(result[1]).toBeCloseTo(1.1);
            expect(result[2]).toBeCloseTo(1.2);
            expect(result[3]).toBeCloseTo(1.3);
        });

        it('should compute velocity magnitude', () => {
            const solution = createTestSolution(4);
            const result = computeScalarField(solution, 'velocity_magnitude');

            expect(result.length).toBe(4);
            // For point 0: u=0.5, v=0.3, w=0.2 -> |V| = sqrt(0.25 + 0.09 + 0.04) = sqrt(0.38) ≈ 0.6164
            expect(result[0]).toBeCloseTo(0.6164, 3);
        });

        it('should compute pressure with gamma from solution', () => {
            const solution = createTestSolution(4, true);
            const result = computeScalarField(solution, 'pressure');

            expect(result.length).toBe(4);

            // For point 0: rho=1.0, u=0.5, v=0.3, w=0.2, rhoe=2.5, gamma=1.4
            // KE = 0.5 * 1.0 * (0.25 + 0.09 + 0.04) = 0.19
            // IE = 2.5 - 0.19 = 2.31
            // p = (1.4 - 1) * 2.31 = 0.924
            expect(result[0]).toBeCloseTo(0.924, 2);
        });

        it('should compute pressure with default gamma when not provided', () => {
            const solution = createTestSolution(4, false);
            const result = computeScalarField(solution, 'pressure');

            expect(result.length).toBe(4);

            // Should use DEFAULT_GAMMA = 1.4
            // Same calculation as above
            expect(result[0]).toBeCloseTo(0.924, 2);
        });

        it('should handle zero density gracefully', () => {
            const solution = createTestSolution(2);
            solution.rho[0] = 0;

            const velocity = computeScalarField(solution, 'velocity_magnitude');
            expect(velocity[0]).toBe(0);

            const pressure = computeScalarField(solution, 'pressure');
            expect(pressure[0]).toBe(0);
        });

        it('should compute momentum components', () => {
            const solution = createTestSolution(4);

            const momX = computeScalarField(solution, 'momentum_x');
            expect(momX[0]).toBeCloseTo(0.5);

            const momY = computeScalarField(solution, 'momentum_y');
            expect(momY[0]).toBeCloseTo(0.3);

            const momZ = computeScalarField(solution, 'momentum_z');
            expect(momZ[0]).toBeCloseTo(0.2);
        });

        it('should compute energy field', () => {
            const solution = createTestSolution(4);
            const result = computeScalarField(solution, 'energy');

            expect(result.length).toBe(4);
            expect(result[0]).toBeCloseTo(2.5);
            expect(result[3]).toBeCloseTo(3.25);
        });
    });

    describe('getFieldStats', () => {
        it('should compute correct statistics', () => {
            const values = new Float32Array([1.0, 2.0, 3.0, 4.0, 5.0]);
            const stats = getFieldStats(values);

            expect(stats.min).toBe(1.0);
            expect(stats.max).toBe(5.0);
            expect(stats.mean).toBe(3.0);
            expect(stats.stdDev).toBeCloseTo(1.4142, 3);
        });

        it('should handle single value', () => {
            const values = new Float32Array([42.0]);
            const stats = getFieldStats(values);

            expect(stats.min).toBe(42.0);
            expect(stats.max).toBe(42.0);
            expect(stats.mean).toBe(42.0);
            expect(stats.stdDev).toBe(0.0);
        });

        it('should handle empty array', () => {
            const values = new Float32Array([]);
            const stats = getFieldStats(values);

            expect(stats.min).toBe(0);
            expect(stats.max).toBe(0);
            expect(stats.mean).toBe(0);
            expect(stats.stdDev).toBe(0);
        });

        it('should handle uniform values', () => {
            const values = new Float32Array([3.14, 3.14, 3.14, 3.14]);
            const stats = getFieldStats(values);

            expect(stats.min).toBeCloseTo(3.14, 2);
            expect(stats.max).toBeCloseTo(3.14, 2);
            expect(stats.mean).toBeCloseTo(3.14, 2);
            expect(stats.stdDev).toBeCloseTo(0, 6);
        });

        it('should handle negative values', () => {
            const values = new Float32Array([-10, -5, 0, 5, 10]);
            const stats = getFieldStats(values);

            expect(stats.min).toBe(-10);
            expect(stats.max).toBe(10);
            expect(stats.mean).toBe(0);
        });

        it('should handle empty array', () => {
            const values = new Float32Array([]);
            const stats = getFieldStats(values);

            expect(stats.min).toBe(0);
            expect(stats.max).toBe(0);
            expect(stats.mean).toBe(0);
            expect(stats.stdDev).toBe(0);
        });
    });

    describe('formatValue', () => {
        it('should format zero correctly', () => {
            expect(formatValue(0)).toBe('0');
        });

        it('should format very small values in scientific notation', () => {
            const result = formatValue(0.000001);
            expect(result).toContain('e');
        });

        it('should format small values with decimals', () => {
            const result = formatValue(0.5, 3);
            const parsed = parseFloat(result);
            expect(parsed).toBeCloseTo(0.5, 1);
        });

        it('should format normal values appropriately', () => {
            const result = formatValue(12.3456, 3);
            const parsed = parseFloat(result);
            expect(parsed).toBeCloseTo(12.3456, 1);
        });

        it('should format large values in scientific notation', () => {
            const result = formatValue(12345.6);
            expect(result).toContain('e');
        });

        it('should handle NaN and Infinity', () => {
            expect(formatValue(NaN)).toBe('N/A');
            expect(formatValue(Infinity)).toBe('N/A');
            expect(formatValue(-Infinity)).toBe('N/A');
        });

        it('should respect decimals parameter', () => {
            const result1 = formatValue(0.123456, 2);
            const result2 = formatValue(0.123456, 5);
            // With fewer decimals, we should lose precision
            expect(result1.length).toBeLessThanOrEqual(result2.length);
        });
    });

    describe('getFieldInfo', () => {
        it('should return info for valid field', () => {
            const info = getFieldInfo('density');
            expect(info.field).toBe('density');
            expect(info.name).toBeDefined();
            expect(info.name.length).toBeGreaterThan(0);
        });

        it('should return default info for invalid field', () => {
            const info = getFieldInfo('invalid' as any);
            expect(info).toBeDefined();
            expect(info.field).toBe('none');
        });

        it('should have different info for each field', () => {
            const density = getFieldInfo('density');
            const pressure = getFieldInfo('pressure');

            expect(density.name).not.toBe(pressure.name);
            expect(density.unit).not.toBe(pressure.unit);
        });
    });

    describe('SCALAR_FIELDS', () => {
        it('should define all expected fields', () => {
            const fieldNames = SCALAR_FIELDS.map(f => f.field);

            expect(fieldNames).toContain('density');
            expect(fieldNames).toContain('pressure');
            expect(fieldNames).toContain('velocity_magnitude');
            expect(fieldNames).toContain('momentum_x');
            expect(fieldNames).toContain('momentum_y');
            expect(fieldNames).toContain('momentum_z');
            expect(fieldNames).toContain('energy');
        });

        it('should have proper metadata for each field', () => {
            SCALAR_FIELDS.forEach(field => {
                expect(field.field).toBeDefined();
                expect(field.name).toBeDefined();
                expect(field.unit).toBeDefined();
                expect(field.description).toBeDefined();
                expect(field.name.length).toBeGreaterThan(0);
                expect(field.description.length).toBeGreaterThan(0);
            });
        });
    });
});

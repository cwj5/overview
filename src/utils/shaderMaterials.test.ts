/**
 * Unit tests for shader material utilities
 */

import { describe, it, expect } from 'vitest';
import {
    createSolidVertexColorMaterial,
    createWireframeVertexColorMaterial,
    updateMaterialOpacity,
    detectColorNormalization,
    normalizeColorData,
} from './shaderMaterials';
import { MESH_RENDERING } from './constants';

describe('shaderMaterials', () => {
    describe('createSolidVertexColorMaterial', () => {
        it('should create a valid shader material', () => {
            const material = createSolidVertexColorMaterial();

            expect(material).toBeDefined();
            expect(material.uniforms).toBeDefined();
            expect(material.uniforms.opacity).toBeDefined();
            expect(material.uniforms.opacity.value).toBe(MESH_RENDERING.DEFAULT_OPACITY);
        });

        it('should have depthWrite and depthTest enabled', () => {
            const material = createSolidVertexColorMaterial();

            expect(material.depthWrite).toBe(true);
            expect(material.depthTest).toBe(true);
        });

        it('should not be transparent initially', () => {
            const material = createSolidVertexColorMaterial();

            expect(material.transparent).toBe(false);
        });
    });

    describe('createWireframeVertexColorMaterial', () => {
        it('should create a valid shader material', () => {
            const material = createWireframeVertexColorMaterial();

            expect(material).toBeDefined();
            expect(material.uniforms).toBeDefined();
            expect(material.uniforms.opacity).toBeDefined();
        });

        it('should be transparent', () => {
            const material = createWireframeVertexColorMaterial();

            expect(material.transparent).toBe(true);
        });
    });

    describe('updateMaterialOpacity', () => {
        it('should update opacity to dimmed value when true', () => {
            const material = createSolidVertexColorMaterial();

            updateMaterialOpacity(material, true);

            expect(material.uniforms.opacity.value).toBe(MESH_RENDERING.DIMMED_OPACITY);
            expect(material.transparent).toBe(true);
        });

        it('should update opacity to full value when false', () => {
            const material = createSolidVertexColorMaterial();

            updateMaterialOpacity(material, true);
            updateMaterialOpacity(material, false);

            expect(material.uniforms.opacity.value).toBe(MESH_RENDERING.DEFAULT_OPACITY);
            expect(material.transparent).toBe(false);
        });
    });

    describe('detectColorNormalization', () => {
        it('should detect need for normalization when max > 1', () => {
            const colors = [0, 128, 255];
            const result = detectColorNormalization(colors);

            expect(result.shouldNormalize).toBe(true);
            expect(result.maxSample).toBe(255);
        });

        it('should not detect need for normalization when max <= 1', () => {
            const colors = [0, 0.5, 1.0];
            const result = detectColorNormalization(colors);

            expect(result.shouldNormalize).toBe(false);
            expect(result.maxSample).toBeLessThanOrEqual(1.0);
        });

        it('should sample correctly from array', () => {
            const colors = new Array(1000).fill(50);
            colors[100] = 200;

            const result = detectColorNormalization(colors);

            expect(result.shouldNormalize).toBe(true);
            expect(result.maxSample).toBeGreaterThan(1);
        });
    });

    describe('normalizeColorData', () => {
        it('should normalize 0-255 to 0-1 range', () => {
            const colors = [0, 127.5, 255];
            const normalized = normalizeColorData(colors);

            expect(normalized[0]).toBeCloseTo(0);
            expect(normalized[1]).toBeCloseTo(0.5, 1);
            expect(normalized[2]).toBeCloseTo(1.0);
        });

        it('should handle empty arrays', () => {
            const colors: number[] = [];
            const normalized = normalizeColorData(colors);

            expect(normalized.length).toBe(0);
        });

        it('should preserve array length', () => {
            const colors = new Array(1000).fill(100);
            const normalized = normalizeColorData(colors);

            expect(normalized.length).toBe(1000);
        });

        it('should return Float32Array', () => {
            const colors = [0, 128, 255];
            const normalized = normalizeColorData(colors);

            expect(normalized).toBeInstanceOf(Float32Array);
        });
    });
});

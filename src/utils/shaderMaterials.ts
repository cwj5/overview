/**
 * Shader material factory and utilities for 3D rendering
 */

import { ShaderMaterial, DoubleSide } from 'three';
import { MESH_RENDERING, LIGHT_SOURCES } from './constants';

/**
 * Create a shader material with vertex colors for solid meshes
 */
export function createSolidVertexColorMaterial(): ShaderMaterial {
    return new ShaderMaterial({
        transparent: false,
        depthWrite: true,
        depthTest: true,
        side: DoubleSide,
        uniforms: {
            opacity: { value: MESH_RENDERING.DEFAULT_OPACITY },
        },
        vertexShader: `
            attribute vec3 color;
            varying vec3 vColor;
            varying vec3 vNormal;
            void main() {
                vColor = color;
                vNormal = normalize(normalMatrix * normal);
                gl_Position = projectionMatrix * modelViewMatrix * vec4(position, 1.0);
            }
        `,
        fragmentShader: `
            uniform float opacity;
            varying vec3 vColor;
            varying vec3 vNormal;
            void main() {
                // Multiple light sources for better global illumination
                vec3 light1 = normalize(vec3(${LIGHT_SOURCES.light1.x}, ${LIGHT_SOURCES.light1.y}, ${LIGHT_SOURCES.light1.z}));
                vec3 light2 = normalize(vec3(${LIGHT_SOURCES.light2.x}, ${LIGHT_SOURCES.light2.y}, ${LIGHT_SOURCES.light2.z}));
                vec3 light3 = normalize(vec3(${LIGHT_SOURCES.light3.x}, ${LIGHT_SOURCES.light3.y}, ${LIGHT_SOURCES.light3.z}));
                vec3 normal = normalize(vNormal);
                
                // Check if this is a backface
                float facing = gl_FrontFacing ? 1.0 : -1.0;
                normal *= facing;
                
                // Apply lighting from multiple sources
                float diffuse1 = max(dot(normal, light1), 0.0);
                float diffuse2 = max(dot(normal, light2), 0.0) * ${LIGHT_SOURCES.light2.multiplier};
                float diffuse3 = max(dot(normal, light3), 0.0) * ${LIGHT_SOURCES.light3.multiplier};
                float diffuse = diffuse1 + diffuse2 + diffuse3;
                
                if (gl_FrontFacing) {
                    diffuse = max(diffuse, ${MESH_RENDERING.FRONT_AMBIENT}); // Front faces have ambient
                } else {
                    diffuse *= ${MESH_RENDERING.BACKFACE_MULTIPLIER}; // Backfaces are nearly black
                }
                
                vec3 finalColor = vColor * diffuse;
                gl_FragColor = vec4(finalColor, opacity);
            }
        `,
    });
}

/**
 * Create a shader material with vertex colors for wireframe meshes
 */
export function createWireframeVertexColorMaterial(): ShaderMaterial {
    return new ShaderMaterial({
        transparent: true,
        uniforms: {
            opacity: { value: MESH_RENDERING.DEFAULT_OPACITY },
        },
        vertexShader: `
            attribute vec3 color;
            varying vec3 vColor;
            void main() {
                vColor = color;
                gl_Position = projectionMatrix * modelViewMatrix * vec4(position, 1.0);
            }
        `,
        fragmentShader: `
            uniform float opacity;
            varying vec3 vColor;
            void main() {
                gl_FragColor = vec4(vColor, opacity);
            }
        `,
    });
}

/**
 * Update material opacity based on dimmed state
 */
export function updateMaterialOpacity(
    material: ShaderMaterial,
    isDimmed: boolean
): void {
    const opacity = isDimmed ? MESH_RENDERING.DIMMED_OPACITY : MESH_RENDERING.DEFAULT_OPACITY;
    material.uniforms.opacity.value = opacity;
    material.transparent = isDimmed;
    material.needsUpdate = true;
}

/**
 * Detect if color data needs normalization from 0-255 to 0-1
 */
export function detectColorNormalization(colors: number[]): { shouldNormalize: boolean; maxSample: number } {
    let maxSample = 0;
    const sampleCount = Math.min(colors.length, MESH_RENDERING.CHUNK_SIZE);

    for (let i = 0; i < sampleCount; i += 1) {
        const v = colors[i];
        if (v > maxSample) maxSample = v;
    }

    return {
        shouldNormalize: maxSample > 1.0,
        maxSample,
    };
}

/**
 * Normalize color data from 0-255 to 0-1
 */
export function normalizeColorData(colors: number[]): Float32Array {
    const normalized = new Float32Array(colors.length);
    for (let i = 0; i < colors.length; i += 1) {
        normalized[i] = colors[i] / 255.0;
    }
    return normalized;
}

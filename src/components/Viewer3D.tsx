import { Canvas } from '@react-three/fiber';
import { OrbitControls } from '@react-three/drei';
import { useState, useEffect, useMemo, useRef } from 'react';
import { BufferGeometry, BufferAttribute, ShaderMaterial, DoubleSide } from 'three';
import { invoke } from '@tauri-apps/api/core';
import { logger } from '../utils/logger';
import type { GridItem, GridSlice, ArbitrarySlice } from '../types/grids';
import type { ColorScheme } from '../utils/colorMapping';
import type { ScalarField } from '../utils/solutionData';
import { getVisibleGridItems } from '../utils/gridUtils';

interface MeshGeometry {
    vertices: number[];
    indices: number[];
    triangle_indices: number[];
    normals: number[];
    vertex_count: number;
    face_count: number;
    colors?: number[];
}

interface SerializableGrid {
    dimensions: { i: number; j: number; k: number };
    x_coords: number[];
    y_coords: number[];
    z_coords: number[];
    original_indices?: number[]; // Maps sliced points back to original grid indices
}

interface Viewer3DProps {
    grids: GridItem[];
    selectedGridIds: string[];
    isolateSelected: boolean;
    ignoreIblank: boolean;
    scalarField?: ScalarField;
    colorScheme?: ColorScheme;
    showWireframe?: boolean;
    shadingMode?: 'none' | 'flat' | 'smooth';
    sliceEnabled?: boolean;
    gridSlices?: Record<string, GridSlice[]>;
    arbitrarySlices?: ArbitrarySlice[];
    onSlicesChange?: (slices: Record<string, GridSlice[]>) => void;
    onLoadingChange?: (isLoading: boolean) => void;
}

function SolidMeshRenderer({
    meshGeometry,
    color,
    dimmed,
    flatShading = false,
}: {
    meshGeometry: MeshGeometry;
    color: string;
    dimmed: boolean;
    flatShading?: boolean;
}) {
    // Shader for field quantity colors (vertex colors) - both sides equally visible
    const vertexColorMaterial = useMemo(() => {
        return new ShaderMaterial({
            transparent: false,
            depthWrite: true,
            depthTest: true,
            side: 2, // DoubleSide
            uniforms: {
                opacity: { value: 1.0 },
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
                    vec3 light1 = normalize(vec3(0.5, 0.5, 1.0));
                    vec3 light2 = normalize(vec3(-0.5, -0.3, 0.8));
                    vec3 light3 = normalize(vec3(0.0, 1.0, 0.3));
                    vec3 normal = normalize(vNormal);
                    
                    // Check if this is a backface
                    float facing = gl_FrontFacing ? 1.0 : -1.0;
                    normal *= facing;
                    
                    // Apply lighting from multiple sources
                    float diffuse1 = max(dot(normal, light1), 0.0);
                    float diffuse2 = max(dot(normal, light2), 0.0) * 0.5;
                    float diffuse3 = max(dot(normal, light3), 0.0) * 0.3;
                    float diffuse = diffuse1 + diffuse2 + diffuse3;
                    
                    // Both sides equally visible for field quantity visualization
                    diffuse = max(diffuse, 0.7);
                    
                    vec3 finalColor = vColor * diffuse;
                    gl_FragColor = vec4(finalColor, opacity);
                }
            `,
        });
    }, []);

    // Shader for grid ID colors (solid color) - backfaces darker for depth perception
    const solidColorMaterial = useMemo(() => {
        const hexColor = parseInt(color.replace('#', ''), 16);
        const r = ((hexColor >> 16) & 255) / 255;
        const g = ((hexColor >> 8) & 255) / 255;
        const b = (hexColor & 255) / 255;

        return new ShaderMaterial({
            transparent: false,
            depthWrite: true,
            depthTest: true,
            side: 2, // DoubleSide
            uniforms: {
                opacity: { value: 1.0 },
                baseColor: { value: [r, g, b] },
            },
            vertexShader: `
                varying vec3 vNormal;
                void main() {
                    vNormal = normalize(normalMatrix * normal);
                    gl_Position = projectionMatrix * modelViewMatrix * vec4(position, 1.0);
                }
            `,
            fragmentShader: `
                uniform float opacity;
                uniform vec3 baseColor;
                varying vec3 vNormal;
                void main() {
                    // Multiple light sources for better global illumination
                    vec3 light1 = normalize(vec3(0.5, 0.5, 1.0));
                    vec3 light2 = normalize(vec3(-0.5, -0.3, 0.8));
                    vec3 light3 = normalize(vec3(0.0, 1.0, 0.3));
                    vec3 normal = normalize(vNormal);
                    
                    // Check if this is a backface
                    float facing = gl_FrontFacing ? 1.0 : -1.0;
                    normal *= facing;
                    
                    // Apply lighting from multiple sources
                    float diffuse1 = max(dot(normal, light1), 0.0);
                    float diffuse2 = max(dot(normal, light2), 0.0) * 0.5;
                    float diffuse3 = max(dot(normal, light3), 0.0) * 0.3;
                    float diffuse = diffuse1 + diffuse2 + diffuse3;
                    
                    // Differentiate front and back faces for depth perception
                    if (gl_FrontFacing) {
                        diffuse = max(diffuse, 0.7); // Front faces have ambient
                    } else {
                        diffuse *= 0.3; // Backfaces are darker
                    }
                    
                    vec3 finalColor = baseColor * diffuse;
                    gl_FragColor = vec4(finalColor, opacity);
                }
            `,
        });
    }, [color]);

    useEffect(() => {
        vertexColorMaterial.transparent = dimmed;
        vertexColorMaterial.depthWrite = !dimmed;
        vertexColorMaterial.uniforms.opacity.value = dimmed ? 0.35 : 1.0;
        vertexColorMaterial.needsUpdate = true;
    }, [dimmed, vertexColorMaterial]);

    useEffect(() => {
        solidColorMaterial.transparent = dimmed;
        solidColorMaterial.depthWrite = !dimmed;
        solidColorMaterial.uniforms.opacity.value = dimmed ? 0.35 : 1.0;
        solidColorMaterial.needsUpdate = true;
    }, [dimmed, solidColorMaterial]);

    const geometry = useMemo(() => {
        const geo = new BufferGeometry();
        geo.setAttribute(
            'position',
            new BufferAttribute(new Float32Array(meshGeometry.vertices), 3)
        );

        // Add normals for smooth shading
        geo.setAttribute(
            'normal',
            new BufferAttribute(new Float32Array(meshGeometry.normals), 3)
        );

        const colors = meshGeometry.colors;
        const hasColors = !!colors && colors.length === meshGeometry.vertices.length;

        // Add vertex colors if available and length matches vertices
        if (hasColors) {
            let colorArray = colors;

            // Detect 0-255 color data and normalize to 0-1 if needed
            let maxSample = 0;
            const sampleCount = Math.min(colors.length, 3000);
            for (let i = 0; i < sampleCount; i += 1) {
                const v = colors[i];
                if (v > maxSample) maxSample = v;
            }

            if (maxSample > 1.0) {
                const normalized = new Float32Array(colors.length);
                for (let i = 0; i < colors.length; i += 1) {
                    normalized[i] = colors[i] / 255.0;
                }
                colorArray = Array.from(normalized);
            }

            geo.setAttribute(
                'color',
                new BufferAttribute(new Float32Array(colorArray), 3)
            );
        }

        geo.setIndex(new BufferAttribute(new Uint32Array(meshGeometry.triangle_indices), 1));

        // Compute bounding sphere for frustum culling
        geo.computeBoundingSphere();

        return geo;
    }, [meshGeometry]);

    // Use vertex colors if available, otherwise use single color
    const hasColors = !!meshGeometry.colors && meshGeometry.colors.length === meshGeometry.vertices.length;

    return (
        <mesh geometry={geometry} frustumCulled={true}>
            {hasColors ? (
                <primitive object={vertexColorMaterial} attach="material" />
            ) : (
                <primitive object={solidColorMaterial} attach="material" />
            )}
        </mesh>
    );
}

function MeshRenderer({
    meshGeometry,
    color,
    dimmed,
}: {
    meshGeometry: MeshGeometry;
    color: string;
    dimmed: boolean;
}) {
    const vertexColorMaterial = useMemo(() => {
        return new ShaderMaterial({
            transparent: true,
            uniforms: {
                opacity: { value: 1.0 },
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
    }, []);

    useEffect(() => {
        vertexColorMaterial.uniforms.opacity.value = dimmed ? 0.35 : 1.0;
        vertexColorMaterial.needsUpdate = true;
    }, [dimmed, vertexColorMaterial]);

    const geometry = useMemo(() => {
        const geo = new BufferGeometry();
        geo.setAttribute(
            'position',
            new BufferAttribute(new Float32Array(meshGeometry.vertices), 3)
        );

        const colors = meshGeometry.colors;
        const hasColors = !!colors && colors.length === meshGeometry.vertices.length;

        // Add vertex colors if available and length matches vertices
        if (hasColors) {
            let colorArray = colors;

            // Detect 0-255 color data and normalize to 0-1 if needed
            let maxSample = 0;
            const sampleCount = Math.min(colors.length, 3000);
            for (let i = 0; i < sampleCount; i += 1) {
                const v = colors[i];
                if (v > maxSample) maxSample = v;
            }

            if (maxSample > 1.0) {
                const normalized = new Float32Array(colors.length);
                for (let i = 0; i < colors.length; i += 1) {
                    normalized[i] = colors[i] / 255.0;
                }
                colorArray = Array.from(normalized);
                logger.warn('Detected 0-255 color data. Normalizing to 0-1.', 'MeshRenderer');
            }

            geo.setAttribute(
                'color',
                new BufferAttribute(new Float32Array(colorArray), 3)
            );
        } else if (colors && colors.length > 0) {
            logger.warn(
                `Color array length (${colors.length}) does not match vertex array length (${meshGeometry.vertices.length}). Ignoring colors.`,
                'MeshRenderer'
            );
        }

        geo.setIndex(new BufferAttribute(new Uint32Array(meshGeometry.indices), 1));

        // Compute bounding sphere for frustum culling
        geo.computeBoundingSphere();

        return geo;
    }, [meshGeometry]);

    // Use vertex colors if available, otherwise use single color
    const hasColors = !!meshGeometry.colors && meshGeometry.colors.length === meshGeometry.vertices.length;

    return (
        <lineSegments geometry={geometry} frustumCulled={true}>
            {hasColors ? (
                <primitive object={vertexColorMaterial} attach="material" />
            ) : (
                <lineBasicMaterial
                    color={color}
                    transparent={dimmed}
                    opacity={dimmed ? 0.35 : 1}
                />
            )}
        </lineSegments>
    );
}

export default function Viewer3D({
    grids,
    selectedGridIds,
    isolateSelected,
    ignoreIblank,
    scalarField = 'none',
    colorScheme = 'viridis',
    showWireframe = true,
    shadingMode = 'none',
    sliceEnabled = false,
    gridSlices = {},
    arbitrarySlices = [],
    onSlicesChange,
    onLoadingChange
}: Viewer3DProps) {
    const [meshById, setMeshById] = useState<Record<string, MeshGeometry>>({});
    const [loadingById, setLoadingById] = useState<Record<string, number>>({});
    const [error, setError] = useState<string | null>(null);
    const cleanGridCacheRef = useRef<Record<string, SerializableGrid>>({});
    const autoCreatedSlicesRef = useRef(false);

    type MeshResult = { id: string; mesh: MeshGeometry } | { id: string; error: string };

    // Notify parent when loading state changes
    useEffect(() => {
        const isLoading = Object.keys(loadingById).length > 0;
        onLoadingChange?.(isLoading);
    }, [loadingById, onLoadingChange]);

    const gridIdKey = useMemo(() => grids.map((grid) => grid.id).join('|'), [grids]);

    // Clear meshes when grids change
    useEffect(() => {
        autoCreatedSlicesRef.current = false;
        if (grids.length === 0) {
            setMeshById({});
            setLoadingById({});
            setError(null);
        }
    }, [gridIdKey, grids.length]);

    const lastColorKeyRef = useRef<string>('');
    const lastSliceKeyRef = useRef<string>('');
    const requestIdRef = useRef(0);

    // Memoize a key based on applied slices (updates only on Apply)
    const appliedSlicesKey = useMemo(
        () => (arbitrarySlices || [])
            .filter(s => s.applied)
            .map(s => `${s.id}:${s.applyVersion}`)
            .join('|'),
        [arbitrarySlices]
    );

    // Generate or regenerate meshes as needed
    // When field/scheme changes, regenerate grids with solutions
    useEffect(() => {
        const effectStart = performance.now();
        void invoke('frontend_log', {
            message: `[Viewer3D] effect start grids=${grids.length} field=${scalarField} scheme=${colorScheme} ignoreIblank=${ignoreIblank}`
        });
        if (grids.length === 0) {
            return;
        }

        const currentColorKey = `${scalarField}|${colorScheme}`;
        // Only include APPLIED slices in the slice key to avoid reprocessing while editing
        const sliceKey = `${sliceEnabled}|${JSON.stringify(gridSlices)}|${appliedSlicesKey}`;
        const shouldRecolor = lastColorKeyRef.current !== currentColorKey;
        const shouldReslice = lastSliceKeyRef.current !== sliceKey;

        void invoke('frontend_log', {
            message: `[Viewer3D] Color key check: last="${lastColorKeyRef.current}" current="${currentColorKey}" shouldRecolor=${shouldRecolor}`
        });
        void invoke('frontend_log', {
            message: `[Viewer3D] Slice key check: shouldReslice=${shouldReslice}`
        });

        const hasSlices = Object.keys(gridSlices).length > 0;

        // Auto-create default K=1 slices only once when a new grid set is first loaded
        if (!hasSlices && !autoCreatedSlicesRef.current) {
            void invoke('frontend_log', { message: '[Viewer3D] Creating default K=1 slices on initial grid load' });
            const newSlices: Record<string, GridSlice[]> = {};
            grids.forEach(grid => {
                newSlices[grid.id] = [{
                    id: `${grid.id}_default`,
                    plane: 'K',
                    index: 0
                }];
            });
            autoCreatedSlicesRef.current = true;
            onSlicesChange?.(newSlices);
            return; // Return to let the effect re-run with the new slices
        }

        // No slices means nothing is rendered, even if slicing is disabled
        if (!hasSlices) {
            requestIdRef.current += 1; // Cancel any in-flight mesh work
            setLoadingById({});
            setMeshById((prev) => (Object.keys(prev).length > 0 ? {} : prev));
            lastSliceKeyRef.current = sliceKey;
            return;
        }

        // Clear meshes when slicing is disabled - no rendering when slices are off
        if (!sliceEnabled) {
            requestIdRef.current += 1; // Cancel any in-flight mesh work
            setLoadingById({});
            setMeshById((prev) => (Object.keys(prev).length > 0 ? {} : prev));
            lastSliceKeyRef.current = sliceKey;
            return;
        }

        const gridsWithSlices = grids.filter((grid) => (gridSlices[grid.id]?.length ?? 0) > 0);
        const hasAppliedArbitrarySlices = (arbitrarySlices || []).some(s => s.applied);

        if (sliceEnabled) {
            // Only return early if there are NO slices at all (neither I/J/K nor arbitrary)
            if (gridsWithSlices.length === 0 && !hasAppliedArbitrarySlices) {
                if (Object.keys(meshById).length > 0) {
                    setMeshById({});
                }
                lastSliceKeyRef.current = sliceKey;
                return;
            }

            // Clean up I/J/K meshes for grids without slices
            const gridsWithoutSlices = grids.filter((grid) => (gridSlices[grid.id]?.length ?? 0) === 0);
            const hasStaleMeshes = gridsWithoutSlices.some((grid) => meshById[grid.id]);
            if (hasStaleMeshes) {
                setMeshById((prev) => {
                    const next = { ...prev };
                    gridsWithoutSlices.forEach((grid) => {
                        delete next[grid.id];
                    });
                    return next;
                });
            }

            // Clean up arbitrary meshes only when slices are removed or no longer applied
            const appliedArbitraryIds = new Set((arbitrarySlices || []).filter(s => s.applied).map(s => s.id));
            const staleArbitraryMeshes = Object.keys(meshById).filter(id => {
                if (id.startsWith('arbitrary::')) {
                    const parts = id.split('::');
                    const sliceId = parts[1];
                    return !appliedArbitraryIds.has(sliceId);
                }
                // Legacy format: arbitrary_${sliceId}_${gridId} (cannot reliably parse), remove
                if (id.startsWith('arbitrary_')) {
                    return true;
                }
                return false;
            });

            if (staleArbitraryMeshes.length > 0) {
                setMeshById((prev) => {
                    const next = { ...prev };
                    staleArbitraryMeshes.forEach((id: string) => {
                        delete next[id];
                    });
                    return next;
                });
            }
        }

        // targetGrids: grids that need I/J/K slice processing
        // For arbitrary slices, we always process ALL grids regardless of I/J/K slices
        const targetGrids = sliceEnabled ? gridsWithSlices : grids;

        // Determine which grids need to be regenerated
        // 1. On color/field change: regenerate all grids to avoid stale colors
        // 2. On slice change: regenerate all grids affected by slice changes
        // 3. Otherwise: only grids without any mesh
        let missing = shouldRecolor
            ? targetGrids
            : shouldReslice
                ? targetGrids  // Regenerate all grids when slice config changes
                : targetGrids.filter((grid) => !meshById[grid.id]);

        void invoke('frontend_log', {
            message: `[Viewer3D] Missing grids: ${missing.length} of ${targetGrids.length} (shouldRecolor=${shouldRecolor}, shouldReslice=${shouldReslice})`
        });

        // Regenerate arbitrary slices if config changed, field/color changed, or they're newly enabled
        const needArbitraryRegen = hasAppliedArbitrarySlices && (shouldReslice || shouldRecolor);

        void invoke('frontend_log', {
            message: `[Viewer3D] Arbitrary check: applied=${hasAppliedArbitrarySlices} shouldReslice=${shouldReslice} shouldRecolor=${shouldRecolor} needRegen=${needArbitraryRegen}`
        });

        if (missing.length === 0 && !needArbitraryRegen) {
            void invoke('frontend_log', { message: '[Viewer3D] effect no-op (missing=0, no reslice needed)' });
            lastSliceKeyRef.current = sliceKey;
            return;
        }

        let isCancelled = false;
        const requestId = requestIdRef.current + 1;
        requestIdRef.current = requestId;
        setError(null);
        setLoadingById((prev) => {
            const next = { ...prev };
            missing.forEach((grid) => {
                next[grid.id] = requestId;
            });
            return next;
        });

        const getCleanGrid = async (gridItem: GridItem): Promise<SerializableGrid> => {
            const cached = cleanGridCacheRef.current[gridItem.id];
            if (cached) {
                return cached;
            }
            if (!gridItem.grid.x_coords || !gridItem.grid.y_coords || !gridItem.grid.z_coords) {
                throw new Error(
                    `Missing coordinate arrays: x:${!!gridItem.grid.x_coords}, y:${!!gridItem.grid.y_coords}, z:${!!gridItem.grid.z_coords}`
                );
            }
            await new Promise((resolve) => setTimeout(resolve, 0));
            const cleanGrid = {
                dimensions: {
                    i: gridItem.grid.dimensions.i,
                    j: gridItem.grid.dimensions.j,
                    k: gridItem.grid.dimensions.k,
                },
                x_coords: Array.from(gridItem.grid.x_coords),
                y_coords: Array.from(gridItem.grid.y_coords),
                z_coords: Array.from(gridItem.grid.z_coords),
            };
            cleanGridCacheRef.current[gridItem.id] = cleanGrid;
            return cleanGrid;
        };

        // If arbitrary planes need regeneration, clear existing arbitrary meshes first to avoid remnants
        if (needArbitraryRegen) {
            setMeshById((prev) => {
                const next = { ...prev };
                Object.keys(next).forEach((id) => {
                    if (id.startsWith('arbitrary::') || id.startsWith('arbitrary_')) {
                        delete next[id];
                    }
                });
                return next;
            });
        }

        // Process arbitrary cutting planes (global - affect ALL grids, not just those with I/J/K slices)
        // Only process if they don't exist yet or if slices have changed
        const arbitrarySlicePromises = needArbitraryRegen
            ? (arbitrarySlices || [])
                .filter((slice) => slice.enabled && slice.applied)
                .map((arbitrarySlice) => {
                    void invoke('frontend_log', {
                        message: `[Viewer3D] Processing arbitrary slice: ${arbitrarySlice.name} against ${grids.length} grids`
                    });
                    return Promise.all(
                        grids.map(async (gridItem) => {
                            try {
                                const cleanGrid = await getCleanGrid(gridItem);
                                let mesh: MeshGeometry;

                                // Try to apply solution colors if available
                                if (gridItem.solution && scalarField !== 'none') {
                                    try {
                                        logger.debug(
                                            `Attempting solution coloring for arbitrary plane '${arbitrarySlice.name}' on grid ${gridItem.id}`,
                                            'Viewer3D'
                                        );
                                        mesh = await invoke<MeshGeometry>('compute_solution_colors_arbitrary_plane', {
                                            grid: cleanGrid,
                                            gridIndex: gridItem.gridIndex,
                                            field: scalarField,
                                            colorScheme: colorScheme,
                                            planePoint: arbitrarySlice.planePoint,
                                            planeNormal: arbitrarySlice.planeNormal,
                                        });
                                        const hasColors = mesh.colors && mesh.colors.length > 0;
                                        void invoke('frontend_log', {
                                            message: `[Viewer3D] Arbitrary plane '${arbitrarySlice.name}': Colors ${hasColors ? 'YES' : 'NO'} (${mesh.colors?.length || 0} values)`
                                        });
                                    } catch (colorErr) {
                                        // Fall back to non-colored geometry if solution coloring fails
                                        void invoke('frontend_log', {
                                            message: `[Viewer3D] Solution coloring FAILED on arbitrary plane '${arbitrarySlice.name}': ${colorErr}`
                                        });
                                        logger.error(`Solution coloring on arbitrary plane failed: ${colorErr}`, 'Viewer3D');
                                        mesh = await invoke<MeshGeometry>('slice_arbitrary_plane', {
                                            grid: cleanGrid,
                                            planePoint: arbitrarySlice.planePoint,
                                            planeNormal: arbitrarySlice.planeNormal,
                                        });
                                    }
                                } else {
                                    // No solution data - use base geometry
                                    mesh = await invoke<MeshGeometry>('slice_arbitrary_plane', {
                                        grid: cleanGrid,
                                        planePoint: arbitrarySlice.planePoint,
                                        planeNormal: arbitrarySlice.planeNormal,
                                    });
                                }

                                logger.debug(
                                    `Arbitrary plane '${arbitrarySlice.name}' intersected grid ${gridItem.id}`,
                                    'Viewer3D'
                                );
                                // Use special ID format for arbitrary slices
                                return {
                                    id: `arbitrary::${arbitrarySlice.id}::${gridItem.id}`,
                                    mesh,
                                };
                            } catch (err) {
                                // plane doesn't intersect this grid - expected, not an error
                                return null;
                            }
                        })
                    ).then((results) => results.filter((r) => r !== null));
                })
            : [];

        void invoke('frontend_log', {
            message: `[Viewer3D] Arbitrary slice promises: ${arbitrarySlicePromises.length}`
        });

        // Process per-grid I/J/K slices
        const gridSlicePromises = Promise.all(
            missing.map(async (gridItem) => {
                try {
                    const gridStart = performance.now();
                    let cleanGrid = await getCleanGrid(gridItem);
                    let mesh: MeshGeometry;

                    // Apply all I/J/K slices for this grid if enabled
                    if (sliceEnabled && gridSlices[gridItem.id] && gridSlices[gridItem.id].length > 0) {
                        try {
                            // Render each I/J/K slice independently from the original grid
                            const slicePromises = gridSlices[gridItem.id].map(async (slice) => {
                                const slicedGrid = await invoke<SerializableGrid>('slice_grid', {
                                    grid: cleanGrid,
                                    plane: slice.plane,
                                    index: slice.index,
                                });
                                logger.debug(`Applied ${slice.plane} slice at index ${slice.index}`, 'Viewer3D');
                                return { sliceId: slice.id, grid: slicedGrid, slice };
                            });

                            const sliceResults = await Promise.all(slicePromises);

                            // Generate meshes for each slice with solution colors if available
                            const sliceMeshes = await Promise.all(
                                sliceResults.map(async ({ sliceId, grid, slice }) => {
                                    let sliceMesh: MeshGeometry;
                                    // Try to apply solution colors to sliced geometry
                                    if (gridItem.solution && scalarField !== 'none') {
                                        try {
                                            logger.debug(`Attempting solution coloring for slice ${slice.plane}${slice.index}...`, 'Viewer3D');
                                            sliceMesh = await invoke<MeshGeometry>('compute_solution_colors_sliced', {
                                                slicedGrid: grid,
                                                originalGrid: cleanGrid,
                                                gridIndex: gridItem.gridIndex,
                                                field: scalarField,
                                                colorScheme: colorScheme,
                                                slicePlane: slice.plane,
                                                sliceIndex: slice.index,
                                            });
                                            const hasColors = sliceMesh.colors && sliceMesh.colors.length > 0;
                                            void invoke('frontend_log', {
                                                message: `[Viewer3D] Slice ${slice.plane}${slice.index}: Colors ${hasColors ? 'YES' : 'NO'} (${sliceMesh.colors?.length || 0} values for ${sliceMesh.vertices.length} bytes vertices)`
                                            });
                                        } catch (colorErr) {
                                            // Fall back to non-colored geometry if solution coloring fails
                                            void invoke('frontend_log', {
                                                message: `[Viewer3D] Solution coloring FAILED on slice ${slice.plane}${slice.index}: ${colorErr}`
                                            });
                                            logger.error(`Solution coloring on slice (${slice.plane}${slice.index}) failed: ${colorErr}`, 'Viewer3D');
                                            sliceMesh = await invoke<MeshGeometry>('convert_grid_to_mesh', {
                                                grid: grid,
                                                respect_iblank: !ignoreIblank
                                            });
                                        }
                                    } else {
                                        // No solution data - use base geometry
                                        sliceMesh = await invoke<MeshGeometry>('convert_grid_to_mesh', {
                                            grid: grid,
                                            respect_iblank: !ignoreIblank
                                        });
                                    }
                                    return { sliceId, mesh: sliceMesh };
                                })
                            );

                            // Merge all slice meshes into one
                            if (sliceMeshes.length > 0) {
                                const mergedMesh: MeshGeometry = {
                                    vertices: [],
                                    indices: [],
                                    triangle_indices: [],
                                    normals: [],
                                    colors: undefined,
                                    vertex_count: 0,
                                    face_count: 0,
                                };

                                // Check if all slices have colors before processing them
                                const allHaveColors = sliceMeshes.every(({ mesh }) => mesh.colors && mesh.colors.length > 0);
                                void invoke('frontend_log', {
                                    message: `[Viewer3D] Merging ${sliceMeshes.length} slices, allHaveColors=${allHaveColors}`
                                });
                                sliceMeshes.forEach((sm, idx) => {
                                    void invoke('frontend_log', {
                                        message: `[Viewer3D]  Slice ${idx}: verts=${sm.mesh.vertices.length / 3 | 0}, colors=${sm.mesh.colors?.length || 0}`
                                    });
                                });

                                if (allHaveColors) {
                                    mergedMesh.colors = [];
                                }

                                for (const { mesh: sliceMesh } of sliceMeshes) {
                                    const vertexOffset = mergedMesh.vertices.length / 3;

                                    // Append vertices and normals
                                    mergedMesh.vertices.push(...sliceMesh.vertices);
                                    mergedMesh.normals.push(...sliceMesh.normals);

                                    // Append colors only if we're collecting them from all slices
                                    if (mergedMesh.colors && sliceMesh.colors && sliceMesh.colors.length > 0) {
                                        mergedMesh.colors.push(...sliceMesh.colors);
                                    }

                                    // Append indices (offset by vertex count)
                                    mergedMesh.indices.push(...sliceMesh.indices.map(idx => idx + vertexOffset));
                                    mergedMesh.triangle_indices.push(...sliceMesh.triangle_indices.map(idx => idx + vertexOffset));

                                    // Update counts
                                    mergedMesh.vertex_count += sliceMesh.vertex_count;
                                    mergedMesh.face_count += sliceMesh.face_count;
                                }

                                // Verify colors array matches vertices
                                logger.debug(`Merged: vertices=${mergedMesh.vertices.length}, colors=${mergedMesh.colors?.length ?? 0}`, 'Viewer3D');
                                if (mergedMesh.colors && mergedMesh.colors.length > 0) {
                                    const expectedColorLength = mergedMesh.vertices.length;
                                    void invoke('frontend_log', {
                                        message: `[Viewer3D] Color validation: have ${mergedMesh.colors.length} need ${expectedColorLength}`
                                    });
                                    if (mergedMesh.colors.length !== expectedColorLength) {
                                        void invoke('frontend_log', {
                                            message: `[Viewer3D] MISMATCH: discarding colors`
                                        });
                                        logger.warn(`Color array length mismatch: have ${mergedMesh.colors.length} but need ${expectedColorLength}. This likely means slices have different vertex counts or color computation failed. Discarding colors.`, 'Viewer3D');
                                        mergedMesh.colors = undefined;
                                    } else {
                                        void invoke('frontend_log', {
                                            message: `[Viewer3D] Color validation PASSED`
                                        });
                                        logger.debug(`Color validation PASSED: ${mergedMesh.colors.length} colors for ${mergedMesh.vertices.length} vertices`, 'Viewer3D');
                                    }
                                } else {
                                    void invoke('frontend_log', {
                                        message: `[Viewer3D] No colors in merged mesh`
                                    });
                                    logger.debug(`No colors in merged mesh (expected for uncolored slices)`, 'Viewer3D');
                                }

                                mesh = mergedMesh;
                            } else {
                                throw new Error('No slice meshes generated');
                            }
                        } catch (sliceErr) {
                            const sliceMsg = String(sliceErr);
                            logger.error(`Slicing failed: ${sliceMsg}`, 'Viewer3D');
                            throw sliceErr;
                        }
                    } else {
                        // No slicing - use the original grid
                        if (gridItem.solution && scalarField !== 'none') {
                            try {
                                mesh = await invoke<MeshGeometry>('compute_solution_colors_cached', {
                                    grid: cleanGrid,
                                    gridIndex: gridItem.gridIndex,
                                    field: scalarField,
                                    colorScheme: colorScheme,
                                });
                            } catch (invokeErr) {
                                const invokeMsg = String(invokeErr);
                                logger.error(`[${gridItem.id}] compute_solution_colors_cached FAILED: ${invokeMsg}`, 'Viewer3D');
                                throw invokeErr;
                            }
                        } else {
                            mesh = await invoke<MeshGeometry>('convert_grid_to_mesh', {
                                grid: cleanGrid,
                                respect_iblank: !ignoreIblank
                            });
                        }
                    }

                    void invoke('frontend_log', {
                        message: `[Viewer3D] grid done id=${gridItem.id} ms=${Math.round(performance.now() - gridStart)}`
                    });

                    return { id: gridItem.id, mesh };
                } catch (err) {
                    const errorMsg = String(err);
                    logger.error(`Grid ${gridItem.id} FAILED: ${errorMsg}`, 'Viewer3D');
                    return { id: gridItem.id, error: errorMsg };
                }
            })
        );

        // Wait for both per-grid slices and arbitrary slices to complete
        Promise.all([gridSlicePromises, Promise.all(arbitrarySlicePromises)])
            .then(([gridResults, arbitraryResults]: [MeshResult[], MeshResult[][]]) => {
                if (isCancelled || requestId !== requestIdRef.current) {
                    return;
                }

                // Flatten arbitrary results (array of arrays)
                const flatArbitraryResults = arbitraryResults.flat();

                void invoke('frontend_log', {
                    message: `[Viewer3D] effect done ms=${Math.round(performance.now() - effectStart)} gridResults=${gridResults.length} arbitraryResults=${flatArbitraryResults.length}`
                });

                lastColorKeyRef.current = currentColorKey;
                lastSliceKeyRef.current = sliceKey;

                // Combine both result sets
                const allResults = [...gridResults, ...flatArbitraryResults];

                const errors = allResults.filter((result) => "error" in result) as { id: string; error: string }[];
                if (errors.length > 0) {
                    const errorDetails = errors.map(e => `${e.id}: ${e.error}`).join('\n');
                    const errorMsg = `Failed to convert ${errors.length} grid(s) to mesh:\n${errorDetails}`;
                    logger.error(errorMsg, 'Viewer3D');
                    setError(errorMsg);
                }

                setMeshById((prev) => {
                    const next = { ...prev };
                    allResults.forEach((result) => {
                        if ("mesh" in result) {
                            next[result.id] = result.mesh;
                        }
                    });
                    return next;
                });

                setLoadingById((prev) => {
                    const next = { ...prev };
                    allResults.forEach((result) => {
                        if (next[result.id] === requestId) {
                            delete next[result.id];
                        }
                    });
                    return next;
                });
            });

        return () => {
            isCancelled = true;
            setLoadingById((prev) => {
                const next = { ...prev };
                missing.forEach((grid) => {
                    if (next[grid.id] === requestId) {
                        delete next[grid.id];
                    }
                });
                return next;
            });
            void invoke('frontend_log', {
                message: `[Viewer3D] effect cancelled ms=${Math.round(performance.now() - effectStart)}`
            });
        };
    }, [grids, ignoreIblank, scalarField, colorScheme, sliceEnabled, gridSlices, appliedSlicesKey]);

    const visibleGrids = useMemo(
        () => getVisibleGridItems(grids, selectedGridIds, isolateSelected),
        [grids, isolateSelected, selectedGridIds]
    );

    const enabledArbitraryIds = useMemo(
        () => new Set((arbitrarySlices || []).filter(s => s.applied && s.enabled).map(s => s.id)),
        [arbitrarySlices]
    );

    const stats = useMemo(() => {
        return visibleGrids.reduce(
            (acc, grid) => {
                const mesh = meshById[grid.id];
                if (mesh) {
                    acc.vertices += mesh.vertex_count;
                    acc.edges += mesh.face_count;
                }
                return acc;
            },
            { vertices: 0, edges: 0 }
        );
    }, [meshById, visibleGrids]);

    const isLoading = Object.keys(loadingById).length > 0;

    return (
        <div style={{ width: '100%', height: '100%', position: 'relative' }}>
            <Canvas camera={{ position: [5, 5, 5], fov: 50 }}>
                <ambientLight intensity={0.5} />
                <directionalLight position={[10, 10, 5]} intensity={1} />

                {/* Render mesh based on selected mode */}
                {visibleGrids.map((gridItem) => {
                    const mesh = meshById[gridItem.id];
                    if (!mesh) {
                        return null;
                    }
                    const dimmed = selectedGridIds.length > 0 && !selectedGridIds.includes(gridItem.id) && !isolateSelected;

                    return (
                        <group key={gridItem.id}>
                            {/* Render smooth shaded surface */}
                            {shadingMode === 'smooth' && (
                                <SolidMeshRenderer
                                    meshGeometry={mesh}
                                    color={gridItem.color}
                                    dimmed={dimmed}
                                    flatShading={false}
                                />
                            )}
                            {/* Render flat shaded surface */}
                            {shadingMode === 'flat' && (
                                <SolidMeshRenderer
                                    meshGeometry={mesh}
                                    color={gridItem.color}
                                    dimmed={dimmed}
                                    flatShading={true}
                                />
                            )}
                            {/* Render wireframe */}
                            {showWireframe && (
                                <MeshRenderer
                                    meshGeometry={mesh}
                                    color={gridItem.color}
                                    dimmed={dimmed}
                                />
                            )}
                        </group>
                    );
                })}

                {/* Render arbitrary cutting plane meshes */}
                {Object.entries(meshById)
                    .filter(([id]) => {
                        if (!id.startsWith('arbitrary::')) return false;
                        const parts = id.split('::');
                        const sliceId = parts[1];
                        return enabledArbitraryIds.has(sliceId);
                    })
                    .map(([id, mesh]) => {
                        // Arbitrary slices use a fixed color (light blue)
                        const sliceColor = '#60a5fa';
                        return (
                            <group key={id}>
                                {shadingMode === 'smooth' && (
                                    <SolidMeshRenderer
                                        meshGeometry={mesh}
                                        color={sliceColor}
                                        dimmed={false}
                                        flatShading={false}
                                    />
                                )}
                                {shadingMode === 'flat' && (
                                    <SolidMeshRenderer
                                        meshGeometry={mesh}
                                        color={sliceColor}
                                        dimmed={false}
                                        flatShading={true}
                                    />
                                )}
                                {showWireframe && (
                                    <MeshRenderer
                                        meshGeometry={mesh}
                                        color={sliceColor}
                                        dimmed={false}
                                    />
                                )}
                            </group>
                        );
                    })}

                {/* Camera controls */}
                <OrbitControls enableDamping dampingFactor={0.05} />
            </Canvas>

            {/* UI Controls */}
            <div
                style={{
                    position: 'absolute',
                    top: 10,
                    right: 10,
                    background: 'rgba(0,0,0,0.7)',
                    padding: '10px',
                    borderRadius: '5px',
                    color: 'white',
                    zIndex: 10,
                }}
            >
                {isLoading && <div>Loading mesh...</div>}

                {visibleGrids.length > 0 && (
                    <div style={{ marginTop: isLoading ? '10px' : '0', fontSize: '0.9em' }}>
                        Visible grids: {visibleGrids.length}
                        <br />
                        Vertices: {stats.vertices}
                        <br />
                        Edges: {stats.edges}
                    </div>
                )}
            </div>

            {/* Error Modal/Popup */}
            {error && (
                <div
                    style={{
                        position: 'fixed',
                        top: 0,
                        left: 0,
                        right: 0,
                        bottom: 0,
                        backgroundColor: 'rgba(0, 0, 0, 0.5)',
                        display: 'flex',
                        alignItems: 'center',
                        justifyContent: 'center',
                        zIndex: 1000,
                    }}
                    onClick={() => setError(null)}
                >
                    <div
                        style={{
                            backgroundColor: 'white',
                            borderRadius: '8px',
                            padding: '20px',
                            maxWidth: '500px',
                            boxShadow: '0 4px 12px rgba(0, 0, 0, 0.15)',
                        }}
                        onClick={(e) => e.stopPropagation()}
                    >
                        <div style={{ marginBottom: '15px', fontWeight: 'bold', color: '#333' }}>
                            Error
                        </div>
                        <div style={{ marginBottom: '20px', color: '#666', whiteSpace: 'pre-wrap', wordBreak: 'break-word' }}>
                            {error}
                        </div>
                        <button
                            onClick={() => setError(null)}
                            style={{
                                padding: '8px 16px',
                                backgroundColor: '#ef4444',
                                color: 'white',
                                border: 'none',
                                borderRadius: '4px',
                                cursor: 'pointer',
                                float: 'right',
                            }}
                        >
                            Close
                        </button>
                        <div style={{ clear: 'both' }} />
                    </div>
                </div>
            )}
        </div>
    );
}

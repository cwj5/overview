import { Canvas } from '@react-three/fiber';
import { OrbitControls } from '@react-three/drei';
import { useState, useEffect, useMemo, useRef } from 'react';
import { BufferGeometry, BufferAttribute, ShaderMaterial, DoubleSide } from 'three';
import { invoke } from '@tauri-apps/api/core';
import { logger } from '../utils/logger';
import type { GridItem } from '../types/grids';
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

interface Viewer3DProps {
    grids: GridItem[];
    selectedGridIds: string[];
    isolateSelected: boolean;
    ignoreIblank: boolean;
    scalarField?: ScalarField;
    colorScheme?: ColorScheme;
    showWireframe?: boolean;
    shadingMode?: 'none' | 'flat' | 'smooth';
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
                    
                    if (gl_FrontFacing) {
                        diffuse = max(diffuse, 0.7); // Front faces have ambient
                    } else {
                        diffuse *= 0.3; // Backfaces are nearly black (5% of diffuse)
                    }
                    
                    vec3 finalColor = vColor * diffuse;
                    gl_FragColor = vec4(finalColor, opacity);
                }
            `,
        });
    }, []);

    useEffect(() => {
        vertexColorMaterial.transparent = dimmed;
        vertexColorMaterial.depthWrite = !dimmed;
        vertexColorMaterial.uniforms.opacity.value = dimmed ? 0.35 : 1.0;
        vertexColorMaterial.needsUpdate = true;
    }, [dimmed, vertexColorMaterial]);

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
        return geo;
    }, [meshGeometry]);

    // Use vertex colors if available, otherwise use single color
    const hasColors = !!meshGeometry.colors && meshGeometry.colors.length === meshGeometry.vertices.length;

    return (
        <mesh geometry={geometry}>
            {hasColors ? (
                <primitive object={vertexColorMaterial} attach="material" />
            ) : (
                <meshStandardMaterial
                    color={color}
                    transparent={dimmed}
                    opacity={dimmed ? 0.35 : 1.0}
                    flatShading={flatShading}
                    depthWrite={!dimmed}
                    depthTest={true}
                    side={DoubleSide}
                />
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
        return geo;
    }, [meshGeometry]);

    // Use vertex colors if available, otherwise use single color
    const hasColors = !!meshGeometry.colors && meshGeometry.colors.length === meshGeometry.vertices.length;

    return (
        <lineSegments geometry={geometry}>
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
    shadingMode = 'none'
}: Viewer3DProps) {
    const [meshById, setMeshById] = useState<Record<string, MeshGeometry>>({});
    const [loadingIds, setLoadingIds] = useState<Set<string>>(new Set());
    const [error, setError] = useState<string | null>(null);

    type MeshResult = { id: string; mesh: MeshGeometry } | { id: string; error: string };

    // Clear meshes when grids change
    useEffect(() => {
        if (grids.length === 0) {
            setMeshById({});
            setLoadingIds(new Set());
            setError(null);
        }
    }, [grids.length]);

    const lastColorKeyRef = useRef<string>('');

    // Generate or regenerate meshes as needed
    // When field/scheme changes, regenerate grids with solutions
    useEffect(() => {
        if (grids.length === 0) {
            return;
        }

        const currentColorKey = `${scalarField}|${colorScheme}`;
        const shouldRecolor = lastColorKeyRef.current !== currentColorKey;

        // Determine which grids need to be regenerated
        // 1. On color/field change: regenerate all grids to avoid stale colors
        // 2. Otherwise: only grids without any mesh
        const missing = shouldRecolor
            ? grids
            : grids.filter((grid) => !meshById[grid.id]);

        if (missing.length === 0) {
            return;
        }

        let isCancelled = false;
        setError(null);
        setLoadingIds((prev) => {
            const next = new Set(prev);
            missing.forEach((grid) => next.add(grid.id));
            return next;
        });

        Promise.all(
            missing.map(async (gridItem) => {
                try {
                    // Check if coordinate arrays exist before creating clean copy
                    if (!gridItem.grid.x_coords || !gridItem.grid.y_coords || !gridItem.grid.z_coords) {
                        throw new Error(`Missing coordinate arrays: x:${!!gridItem.grid.x_coords}, y:${!!gridItem.grid.y_coords}, z:${!!gridItem.grid.z_coords}`);
                    }

                    // Create a clean copy of the grid data to ensure proper serialization
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

                    let mesh: MeshGeometry;

                    // Use compute_solution_colors if solution data is available AND user selected a field
                    if (gridItem.solution && scalarField !== 'none') {
                        try {
                            mesh = await invoke<MeshGeometry>('compute_solution_colors', {
                                grid: cleanGrid,
                                solution: gridItem.solution,
                                field: scalarField,
                                colorScheme: colorScheme,
                            });
                        } catch (invokeErr) {
                            const invokeMsg = String(invokeErr);
                            logger.error(`[${gridItem.id}] compute_solution_colors FAILED: ${invokeMsg}`, 'Viewer3D');
                            throw invokeErr;
                        }
                    } else {
                        mesh = await invoke<MeshGeometry>('convert_grid_to_mesh', {
                            grid: cleanGrid,
                            respect_iblank: !ignoreIblank
                        });
                    }

                    return { id: gridItem.id, mesh };
                } catch (err) {
                    const errorMsg = String(err);
                    logger.error(`Grid ${gridItem.id} FAILED: ${errorMsg}`, 'Viewer3D');
                    return { id: gridItem.id, error: errorMsg };
                }
            })
        ).then((results: MeshResult[]) => {
            if (isCancelled) {
                return;
            }

            lastColorKeyRef.current = currentColorKey;

            const errors = results.filter((result) => "error" in result) as { id: string; error: string }[];
            if (errors.length > 0) {
                const errorDetails = errors.map(e => `${e.id}: ${e.error}`).join('\n');
                const errorMsg = `Failed to convert ${errors.length} grid(s) to mesh:\n${errorDetails}`;
                logger.error(errorMsg, 'Viewer3D');
                setError(errorMsg);
            }

            setMeshById((prev) => {
                const next = { ...prev };
                results.forEach((result) => {
                    if ("mesh" in result) {
                        next[result.id] = result.mesh;
                    }
                });
                return next;
            });

            setLoadingIds((prev) => {
                const next = new Set(prev);
                results.forEach((result) => next.delete(result.id));
                return next;
            });
        });

        return () => {
            isCancelled = true;
        };
    }, [grids, ignoreIblank, scalarField, colorScheme, meshById]);

    const visibleGrids = useMemo(
        () => getVisibleGridItems(grids, selectedGridIds, isolateSelected),
        [grids, isolateSelected, selectedGridIds]
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

    const isLoading = loadingIds.size > 0;

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

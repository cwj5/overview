import { Canvas } from '@react-three/fiber';
import { OrbitControls } from '@react-three/drei';
import { useState, useEffect, useMemo } from 'react';
import { BufferGeometry, BufferAttribute } from 'three';
import { invoke } from '@tauri-apps/api/core';
import { logger } from '../utils/logger';
import type { GridItem } from '../types/grids';
import { getVisibleGridItems } from '../utils/gridUtils';

interface MeshGeometry {
    vertices: number[];
    indices: number[];
    normals: number[];
    vertex_count: number;
    face_count: number;
}

interface Viewer3DProps {
    grids: GridItem[];
    selectedGridId: string | null;
    isolateSelected: boolean;
}

function MeshRenderer({
    meshGeometry,
    wireframe,
    color,
    dimmed,
}: {
    meshGeometry: MeshGeometry;
    wireframe: boolean;
    color: string;
    dimmed: boolean;
}) {
    const geometry = useMemo(() => {
        const geo = new BufferGeometry();
        geo.setAttribute(
            'position',
            new BufferAttribute(new Float32Array(meshGeometry.vertices), 3)
        );
        geo.setAttribute(
            'normal',
            new BufferAttribute(new Float32Array(meshGeometry.normals), 3)
        );
        geo.setIndex(new BufferAttribute(new Uint32Array(meshGeometry.indices), 1));
        return geo;
    }, [meshGeometry]);

    return (
        <mesh geometry={geometry}>
            <meshStandardMaterial
                color={color}
                wireframe={wireframe}
                transparent={dimmed}
                opacity={dimmed ? 0.35 : 1}
            />
        </mesh>
    );
}

export default function Viewer3D({ grids, selectedGridId, isolateSelected }: Viewer3DProps) {
    const [wireframe, setWireframe] = useState(true);
    const [meshById, setMeshById] = useState<Record<string, MeshGeometry>>({});
    const [loadingIds, setLoadingIds] = useState<Set<string>>(new Set());
    const [error, setError] = useState<string | null>(null);

    type MeshResult = { id: string; mesh: MeshGeometry } | { id: string; error: string };

    useEffect(() => {
        if (grids.length === 0) {
            setMeshById({});
            setLoadingIds(new Set());
            setError(null);
        }
    }, [grids.length]);

    useEffect(() => {
        if (grids.length === 0) {
            return;
        }

        const missing = grids.filter((grid) => !meshById[grid.id]);
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

                    const mesh = await invoke<MeshGeometry>('convert_grid_to_mesh', { grid: cleanGrid });
                    return { id: gridItem.id, mesh };
                } catch (err) {
                    const errorMsg = String(err);
                    logger.error(`Failed to convert grid ${gridItem.id}: ${errorMsg}`, 'Viewer3D');
                    return { id: gridItem.id, error: errorMsg };
                }
            })
        ).then((results: MeshResult[]) => {
            if (isCancelled) {
                return;
            }

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
    }, [grids]);

    const visibleGrids = useMemo(
        () => getVisibleGridItems(grids, selectedGridId, isolateSelected),
        [grids, isolateSelected, selectedGridId]
    );

    const stats = useMemo(() => {
        return visibleGrids.reduce(
            (acc, grid) => {
                const mesh = meshById[grid.id];
                if (mesh) {
                    acc.vertices += mesh.vertex_count;
                    acc.faces += mesh.face_count;
                }
                return acc;
            },
            { vertices: 0, faces: 0 }
        );
    }, [meshById, visibleGrids]);

    const isLoading = loadingIds.size > 0;

    return (
        <div style={{ width: '100%', height: '100%', position: 'relative' }}>
            <Canvas camera={{ position: [5, 5, 5], fov: 50 }}>
                <ambientLight intensity={0.5} />
                <directionalLight position={[10, 10, 5]} intensity={1} />

                {/* Render actual mesh */}
                {visibleGrids.map((gridItem) => {
                    const mesh = meshById[gridItem.id];
                    if (!mesh) {
                        return null;
                    }
                    const dimmed = !!selectedGridId && gridItem.id !== selectedGridId && !isolateSelected;
                    return (
                        <MeshRenderer
                            key={gridItem.id}
                            meshGeometry={mesh}
                            wireframe={wireframe}
                            color={gridItem.color}
                            dimmed={dimmed}
                        />
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
                <label>
                    <input
                        type="checkbox"
                        checked={wireframe}
                        onChange={(e) => setWireframe(e.target.checked)}
                    />
                    {' '}Wireframe
                </label>

                {isLoading && <div style={{ marginTop: '10px' }}>Loading mesh...</div>}

                {visibleGrids.length > 0 && (
                    <div style={{ marginTop: '10px', fontSize: '0.9em' }}>
                        Visible grids: {visibleGrids.length}
                        <br />
                        Vertices: {stats.vertices}
                        <br />
                        Faces: {stats.faces}
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

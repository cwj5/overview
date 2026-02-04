import { Canvas } from '@react-three/fiber';
import { OrbitControls, Grid } from '@react-three/drei';
import { useState, useEffect, useMemo } from 'react';
import { BufferGeometry, BufferAttribute } from 'three';
import { invoke } from '@tauri-apps/api/core';
import { logger } from '../utils/logger';

interface MeshGeometry {
    vertices: number[];
    indices: number[];
    normals: number[];
    vertex_count: number;
    face_count: number;
}

interface Plot3DGrid {
    dimensions: { i: number; j: number; k: number };
    x_coords: number[];
    y_coords: number[];
    z_coords: number[];
}

interface Viewer3DProps {
    gridData?: Plot3DGrid;
}

function MeshRenderer({
    meshGeometry,
    wireframe,
}: {
    meshGeometry: MeshGeometry;
    wireframe: boolean;
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
            <meshStandardMaterial color="#6366f1" wireframe={wireframe} />
        </mesh>
    );
}

export default function Viewer3D({ gridData }: Viewer3DProps) {
    const [wireframe, setWireframe] = useState(true);
    const [meshGeometry, setMeshGeometry] = useState<MeshGeometry | null>(null);
    const [isLoading, setIsLoading] = useState(false);
    const [error, setError] = useState<string | null>(null);

    useEffect(() => {
        if (!gridData) {
            setMeshGeometry(null);
            setError(null);
            return;
        }

        setIsLoading(true);
        setError(null);

        console.log('Grid data type:', typeof gridData);
        console.log('Is array:', Array.isArray(gridData));
        console.log('Calling convert_grid_to_mesh with:', gridData);
        logger.debug('Converting grid to mesh geometry', 'Viewer3D');

        invoke<MeshGeometry>('convert_grid_to_mesh', { grid: gridData })
            .then((mesh) => {
                console.log('Mesh generated successfully:', mesh);
                logger.info(`Mesh generated: ${mesh.vertex_count} vertices, ${mesh.face_count} faces`, 'Viewer3D');
                setMeshGeometry(mesh);
                setIsLoading(false);
            })
            .catch((err) => {
                const errorMsg = String(err);
                console.error('Error converting grid to mesh:', errorMsg);
                logger.error(`Failed to convert grid to mesh: ${errorMsg}`, 'Viewer3D');
                setError(errorMsg);
                setIsLoading(false);
            });
    }, [gridData]);

    return (
        <div style={{ width: '100%', height: '100vh', position: 'relative' }}>
            <Canvas camera={{ position: [5, 5, 5], fov: 50 }}>
                <ambientLight intensity={0.5} />
                <directionalLight position={[10, 10, 5]} intensity={1} />

                {/* Grid for reference */}
                <Grid args={[10, 10]} />

                {/* Render actual mesh or placeholder */}
                {meshGeometry ? (
                    <MeshRenderer meshGeometry={meshGeometry} wireframe={wireframe} />
                ) : (
                    <mesh>
                        <boxGeometry args={[1, 1, 1]} />
                        <meshStandardMaterial
                            color="#6366f1"
                            wireframe={wireframe}
                        />
                    </mesh>
                )}

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

                {meshGeometry && (
                    <div style={{ marginTop: '10px', fontSize: '0.9em' }}>
                        Vertices: {meshGeometry.vertex_count}
                        <br />
                        Faces: {meshGeometry.face_count}
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

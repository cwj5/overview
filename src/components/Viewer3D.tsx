import { Canvas } from '@react-three/fiber';
import { OrbitControls, Grid } from '@react-three/drei';
import { useState } from 'react';

interface Viewer3DProps {
    gridData?: any; // Will be properly typed once we define PLOT3D data structure
}

export default function Viewer3D({ gridData }: Viewer3DProps) {
    const [wireframe, setWireframe] = useState(true);

    // TODO: Use gridData to render actual PLOT3D mesh
    console.log('Grid data:', gridData);

    return (
        <div style={{ width: '100%', height: '100vh' }}>
            <Canvas camera={{ position: [5, 5, 5], fov: 50 }}>
                <ambientLight intensity={0.5} />
                <directionalLight position={[10, 10, 5]} intensity={1} />

                {/* Grid for reference */}
                <Grid args={[10, 10]} />

                {/* Placeholder mesh - will be replaced with PLOT3D grid data */}
                <mesh>
                    <boxGeometry args={[1, 1, 1]} />
                    <meshStandardMaterial
                        color="#6366f1"
                        wireframe={wireframe}
                    />
                </mesh>

                {/* Camera controls */}
                <OrbitControls
                    enableDamping
                    dampingFactor={0.05}
                />
            </Canvas>

            {/* UI Controls */}
            <div style={{
                position: 'absolute',
                top: 10,
                right: 10,
                background: 'rgba(0,0,0,0.7)',
                padding: '10px',
                borderRadius: '5px',
                color: 'white'
            }}>
                <label>
                    <input
                        type="checkbox"
                        checked={wireframe}
                        onChange={(e) => setWireframe(e.target.checked)}
                    />
                    {' '}Wireframe
                </label>
            </div>
        </div>
    );
}

import React from 'react';
import { useGLTF, Environment, OrbitControls } from '@react-three/drei';
import { Canvas } from '@react-three/fiber';

interface GlbRendererProps {
    modelUrl: string;
    avatarState: 'idle' | 'thinking' | 'speaking' | 'learning' | 'meditating' | 'awakened';
}

const GlbRenderer: React.FC<GlbRendererProps> = ({ modelUrl }) => {
    // Note: useGLTF caches the model automatically
    const { scene } = useGLTF(modelUrl);

    return (
        <Canvas camera={{ position: [0, 0.5, 3], fov: 40 }} style={{ width: '100%', height: '100%', background: 'transparent' }}>
            <ambientLight intensity={0.5} />
            <directionalLight position={[10, 10, 5]} intensity={1} />
            <Environment preset="studio" />
            <primitive object={scene} />
            <OrbitControls enablePan={false} enableZoom={true} />
        </Canvas>
    );
};

export default GlbRenderer;

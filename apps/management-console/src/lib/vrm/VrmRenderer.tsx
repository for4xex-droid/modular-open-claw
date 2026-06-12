/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import React, { Suspense } from 'react';
import { Canvas } from '@react-three/fiber';
import * as THREE from 'three';
import { MeshReflectorMaterial, Sparkles, Float } from '@react-three/drei';
import CharacterBillboard from './CharacterBillboard';
import FluidAura from './FluidAura';
import { cssVar } from '../../utils/cssVar';

interface VrmRendererProps {
    modelUrl: string;
    avatarState: string;
}

const VrmRenderer: React.FC<VrmRendererProps> = ({ modelUrl, avatarState }) => {
    const isThinking = avatarState === 'thinking';
    const isAwakened = avatarState === 'awakened';

    // Dynamic accent color
    const accentColor = isAwakened ? cssVar('--accent-amber') : isThinking ? cssVar('--accent-purple') : cssVar('--accent-cyan');

    return (
        <Canvas
            camera={{ position: [0, 0.25, 3.5], fov: 35 }}
            gl={{ alpha: true, antialias: true, preserveDrawingBuffer: true }}
            style={{ background: 'transparent', pointerEvents: 'none' }}
            onCreated={({ gl }) => {
                gl.setClearColor(new THREE.Color(cssVar('--bg-primary')), 1);
                gl.toneMapping = THREE.ACESFilmicToneMapping;
                gl.toneMappingExposure = 1.4;
            }}
        >
            <fog attach="fog" args={[cssVar('--bg-primary'), 3, 10]} />

            {/* === Lighting Setup === */}
            <ambientLight intensity={0.3} color={cssVar('--bg-dark-obsidian')} />

            {/* Main key light — dramatic top-right */}
            <spotLight
                position={[3, 6, 4]}
                angle={0.2}
                penumbra={0.8}
                intensity={200}
                color={accentColor}
            />

            {/* Fill light — left side, softer */}
            <spotLight
                position={[-4, 3, 2]}
                angle={0.3}
                penumbra={1}
                intensity={80}
                color={cssVar('--accent-purple')}
            />

            {/* Rim light — back, creates edge glow */}
            <pointLight position={[0, 3, -3]} intensity={40} color={cssVar('--accent-cyan')} />

            {/* Under-glow for dramatic effect */}
            <pointLight position={[0, -1, 1]} intensity={15} color={accentColor} />

            {/* Scene offset — shift model down so halo is visible, floor may clip (OK) */}
            <group position={[0, -0.25, 0]}>
                {/* === Character Billboard — The New "Living" Avatar === */}
                <Float speed={1.5} rotationIntensity={0.02} floatIntensity={0.1}>
                    <Suspense fallback={null}>
                        <CharacterBillboard url={modelUrl} avatarState={avatarState} />
                    </Suspense>
                </Float>

                {/* Fluid Aura — organic warmth around the avatar */}
                <FluidAura avatarState={avatarState} />

                {/* === Reflector Floor === */}
                <mesh rotation={[-Math.PI / 2, 0, 0]} position={[0, -0.62, 0]}>
                    <planeGeometry args={[30, 30]} />
                    <MeshReflectorMaterial
                        blur={[400, 200]}
                        resolution={1024}
                        mixBlur={1}
                        mixStrength={80}
                        roughness={0.85}
                        depthScale={1.5}
                        minDepthThreshold={0.3}
                        maxDepthThreshold={1.5}
                        color={cssVar('--bg-primary')}
                        metalness={0.6}
                        mirror={0.15}
                    />
                </mesh>

                {/* === Particles === */}
                <Sparkles
                    count={isAwakened ? 120 : 50}
                    scale={[6, 4, 6]}
                    size={isAwakened ? 3 : 2}
                    speed={0.3}
                    color={accentColor}
                    opacity={0.5}
                />
                <Sparkles
                    count={30}
                    scale={[4, 3, 4]}
                    size={1}
                    speed={0.15}
                    color={cssVar('--text-primary')}
                    opacity={0.15}
                />
            </group>
        </Canvas>
    );
};

export default VrmRenderer;

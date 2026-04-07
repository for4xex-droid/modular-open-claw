/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import React, { Suspense } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { X } from 'lucide-react';
import { Canvas } from '@react-three/fiber';
import { OrbitControls, Float, MeshReflectorMaterial, Sparkles } from '@react-three/drei';
import CharacterBillboard from '../../lib/vrm/CharacterBillboard';
import * as THREE from 'three';
import ErrorBoundary from '../common/ErrorBoundary';
import GlbRenderer from '../../lib/glb/GlbRenderer';
import InxRenderer from '../../lib/inx/InxRenderer';

const _cssVarCache: Record<string, string> = {};
const cssVar = (name: string, fallback: string) => {
    if (typeof document === 'undefined') return fallback;
    if (_cssVarCache[name]) return _cssVarCache[name];
    const val = getComputedStyle(document.documentElement).getPropertyValue(name).trim();
    if (val) _cssVarCache[name] = val;
    return val || fallback;
};

interface AvatarViewerModalProps {
    isOpen: boolean;
    onClose: () => void;
    modelUrl: string;
    avatarState: 'idle' | 'thinking' | 'speaking' | 'learning' | 'meditating' | 'awakened';
    mode: 'vrm' | 'inx' | 'glb' | 'off' | 'lite';
}

const AvatarViewerModal: React.FC<AvatarViewerModalProps> = ({ isOpen, onClose, modelUrl, avatarState, mode }) => {
    return (
        <AnimatePresence>
            {isOpen && (
                <motion.div
                    className="avatar-viewer-modal"
                    initial={{ opacity: 0 }}
                    animate={{ opacity: 1 }}
                    exit={{ opacity: 0 }}
                    style={{
                        position: 'fixed',
                        inset: 0,
                        backgroundColor: 'rgba(6, 8, 12, 0.95)',
                        backdropFilter: 'blur(10px)',
                        zIndex: 9999,
                        display: 'flex',
                        flexDirection: 'column'
                    }}
                >
                    <div style={{ padding: '1.5rem', display: 'flex', justifyContent: 'flex-end', position: 'absolute', top: 0, right: 0, left: 0, zIndex: 10 }}>
                        <button
                            className="close-viewer-btn"
                            onClick={(e) => {
                                e.stopPropagation();
                                onClose();
                            }}
                            style={{
                                background: 'rgba(255,255,255,0.1)',
                                border: '1px solid rgba(255,255,255,0.2)',
                                borderRadius: '50%',
                                width: '40px',
                                height: '40px',
                                display: 'flex',
                                alignItems: 'center',
                                justifyContent: 'center',
                                color: 'var(--text-primary)',
                                cursor: 'pointer',
                                transition: 'all 0.2s'
                            }}
                        >
                            <X size={20} />
                        </button>
                    </div>

                    <div style={{ flex: 1, position: 'relative' }}>
                            {mode === 'vrm' && (
                                <ErrorBoundary fallback={<div style={{color:'red', padding:'2rem', textAlign:'center'}}>Avatar 3D rendering failed.</div>}>
                                    <Canvas
                                        camera={{ position: [0, 0.45, 5.5], fov: 35 }}
                                        gl={{ alpha: true, antialias: true, preserveDrawingBuffer: false }}
                                        onCreated={({ gl }) => {
                                            gl.setClearColor(0x06080c, 1);
                                            gl.toneMapping = THREE.ACESFilmicToneMapping;
                                            gl.toneMappingExposure = 1.4;
                                        }}
                                    >
                                        <fog attach="fog" args={[cssVar('--bg-dark-sidebar', '#06080c'), 3, 10]} />
                                        
                                        <ambientLight intensity={0.5} color="var(--bg-dark)" />
                                        <spotLight position={[3, 6, 4]} angle={0.2} penumbra={0.8} intensity={200} color="var(--accent-cyan)" />
                                        <spotLight position={[-4, 3, 2]} angle={0.3} penumbra={1} intensity={80} color="var(--accent-purple)" />
                                        <pointLight position={[0, 3, -3]} intensity={40} color="var(--accent-cyan)" />
                                        <pointLight position={[0, -1, 1]} intensity={15} color="var(--accent-cyan)" />

                                        <Float speed={1.5} rotationIntensity={0.02} floatIntensity={0.1}>
                                            <Suspense fallback={null}>
                                                <CharacterBillboard url={modelUrl} avatarState={avatarState} />
                                            </Suspense>
                                        </Float>

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
                                                color={cssVar('--bg-dark', '#080808')}
                                                metalness={0.6}
                                                mirror={0.15}
                                            />
                                        </mesh>

                                        <Sparkles count={80} scale={[6, 4, 6]} size={2} speed={0.3} color="var(--accent-cyan)" opacity={0.5} />
                                        <Sparkles count={40} scale={[4, 3, 4]} size={1} speed={0.15} color="var(--text-primary)" opacity={0.15} />

                                        <OrbitControls enablePan={false} maxPolarAngle={Math.PI / 2} minDistance={2} maxDistance={8} />
                                    </Canvas>
                                </ErrorBoundary>
                            )}
                            
                            {mode === 'glb' && (
                                <ErrorBoundary fallback={<div style={{color:'red', padding:'2rem', textAlign:'center'}}>GLB Model failed to load.</div>}>
                                    <GlbRenderer modelUrl={modelUrl} avatarState={avatarState} />
                                </ErrorBoundary>
                            )}

                            {mode === 'inx' && (
                                <ErrorBoundary fallback={<div style={{color:'red', padding:'2rem', textAlign:'center'}}>Inochi2D failed to load.</div>}>
                                    <InxRenderer modelUrl={modelUrl} avatarState={avatarState} />
                                </ErrorBoundary>
                            )}
                        </div>
                    </motion.div>
                )}
            </AnimatePresence>
        );
    };

    export default AvatarViewerModal;

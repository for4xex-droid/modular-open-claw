/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import React, { useEffect, useRef, useState } from 'react';

/**
 * Prop definitions for InxRenderer
 */
interface InxRendererProps {
    modelUrl: string;
    avatarState?: 'idle' | 'thinking' | 'speaking' | 'learning' | 'meditating' | 'awakened';
}

/**
 * Inochi2D Renderer (WASM Integration Placeholder)
 * Loads .inx files using WebGL and applies parameters sent via SSE.
 */
const InxRenderer: React.FC<InxRendererProps> = ({ modelUrl, avatarState }) => {
    const canvasRef = useRef<HTMLCanvasElement>(null);
    const [isLoaded, setIsLoaded] = useState(false);

    useEffect(() => {
        let active = true;
        const canvas = canvasRef.current;
        if (!canvas) return;

        // Initialize WebGL Context
        const gl = canvas.getContext('webgl2') || canvas.getContext('webgl');
        if (!gl) {
            console.error('WebGL is not supported in this browser.');
            return;
        }

        let wasmInstance: any = null;

        const loadModel = async () => {
            console.log(`[InxRenderer] Initiating payload load for ${modelUrl}...`);
            try {
                // [Phase 2 Deferred]: The @nicebyte/inochi2d-es package is currently 
                // unavailable on the public NPM registry. Actual integration of the 
                // Inochi2D WASM runtime is moved to Phase 2 (or later) when an official 
                // release or alternative packaging method is established.
                // 
                // Future Implementation:
                // const { Inochi2D } = await import('@nicebyte/inochi2d-es');
                // await Inochi2D.init();
                // const res = await fetch(modelUrl);
                // const buffer = await res.arrayBuffer();
                // wasmInstance = new Inochi2D();
                // wasmInstance.loadModel(buffer);
                
                // Simulate network/WASM latency for Phase 1
                await new Promise((resolve) => setTimeout(resolve, 800));

                if (active) {
                    console.log(`[InxRenderer] Successfully loaded ${modelUrl}`);
                    setIsLoaded(true);
                }
            } catch (err) {
                console.error(`[InxRenderer] WASM initialization failed:`, err);
            }
        };

        loadModel();

        return () => {
            active = false;
            console.log(`[InxRenderer] Destroying Inochi2D instance for ${modelUrl}`);
            if (wasmInstance) {
                // wasmInstance.destroy();
            }
        };
    }, [modelUrl]);

    useEffect(() => {
        if (!isLoaded) return;
        // Apply AvatarState animations/parameters to the model
        console.log(`[InxRenderer] Applying state: ${avatarState}`);
    }, [avatarState, isLoaded]);

    return (
        <div style={{ position: 'relative', width: '100%', height: '100%', pointerEvents: 'none' }}>
            <canvas
                ref={canvasRef}
                style={{
                    width: '100%',
                    height: '100%',
                    display: isLoaded ? 'block' : 'none'
                }}
            />
            {!isLoaded && (
                <div className="font-mono" style={{
                    position: 'absolute',
                    top: 0, left: 0, right: 0, bottom: 0,
                    display: 'flex',
                    alignItems: 'center',
                    justifyContent: 'center',
                    color: 'var(--text-secondary)'
                }}>
                    [ Inochi2D Runtime Initializing... ]
                </div>
            )}
        </div>
    );
};

export default InxRenderer;

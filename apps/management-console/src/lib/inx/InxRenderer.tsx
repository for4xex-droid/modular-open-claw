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

        // Mock Loading Process
        console.log(`[InxRenderer] Initiating payload load for ${modelUrl}...`);
        
        // TODO: Import @inochi2d/inochi2d-wasm and parse model buffer
        setTimeout(() => {
            if (active) {
                console.log(`[InxRenderer] Successfully loaded ${modelUrl}`);
                setIsLoaded(true);
            }
        }, 1000);

        return () => {
            active = false;
            console.log(`[InxRenderer] Destroying Inochi2D instance for ${modelUrl}`);
            // TODO: Inochi2d Cleanup goes here
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
                <div style={{
                    position: 'absolute',
                    top: 0, left: 0, right: 0, bottom: 0,
                    display: 'flex',
                    alignItems: 'center',
                    justifyContent: 'center',
                    color: 'rgba(255, 255, 255, 0.5)',
                    fontFamily: 'monospace'
                }}>
                    [ Inochi2D Runtime Initializing... ]
                </div>
            )}
        </div>
    );
};

export default InxRenderer;

/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import { Canvas } from '@react-three/fiber';
import { BiomeBackground } from './BiomeBackground';
import { BiomeLighting } from './BiomeLighting';
import { BiomeBorder } from './BiomeBorder';
import { BiomeCellGrid } from './BiomeCellGrid';
import { BiomeSparkles } from './BiomeSparkles';
import { BiomePostEffects } from './BiomePostEffects';
import { BiomeCanvasProps } from './biomeTypes';
import { useState, useCallback } from 'react';

export function BiomeCanvas({
  width,
  height,
  renderView,
  rarity = 0,
  effectType = 'none',
  effectIntensity = 0.5,
  effectCenter = [0.5, 0.5],
  onClick,
  onHover,
  bloomEnabled = true,
  injectionMarks = [],
}: BiomeCanvasProps) {
  const [hoverCell, setHoverCell] = useState<{ x: number; y: number } | null>(null);

  const handlePointerMove = useCallback((e: any) => {
    const x = Math.floor(e.point.x);
    const y = Math.floor(e.point.y);
    if (x >= 0 && x < 128 && y >= 0 && y < 128) {
      setHoverCell(prev => {
        if (prev && prev.x === x && prev.y === y) return prev;
        const coord = { x, y };
        if (onHover) onHover(coord);
        return coord;
      });
    } else {
      setHoverCell(prev => {
        if (prev === null) return prev;
        if (onHover) onHover(null);
        return null;
      });
    }
  }, [onHover]);

  const handlePointerLeave = useCallback(() => {
    setHoverCell(prev => {
      if (prev === null) return prev;
      if (onHover) onHover(null);
      return null;
    });
  }, [onHover]);

  const handlePointerDown = useCallback((e: any) => {
    const x = Math.floor(e.point.x);
    const y = Math.floor(e.point.y);
    if (x >= 0 && x < 128 && y >= 0 && y < 128) {
      if (onClick) onClick({ x, y });
    }
  }, [onClick]);

  return (
    <div style={{
      width: `${width}px`,
      height: `${height}px`,
      position: 'relative',
      overflow: 'hidden',
    }}>
      <Canvas
        gl={{ antialias: true, alpha: false }}
        dpr={[1, 2]}
        orthographic
        camera={{
          left: 0,
          right: 128,
          top: 128,
          bottom: 0,
          near: 0.1,
          far: 1000,
          position: [0, 0, 10],
        }}
      >
        <BiomeBackground />
        <BiomeLighting rarity={rarity} />
        <BiomeBorder />
        <BiomeCellGrid
          renderView={renderView}
          rarity={rarity}
          injectionMarks={injectionMarks}
          hoverCell={hoverCell}
        />
        {/* 入力判定Plane */}
        <mesh
          position={[64, 64, 1]}
          onPointerMove={handlePointerMove}
          onPointerDown={handlePointerDown}
          onPointerOut={handlePointerLeave}
        >
          <planeGeometry args={[128, 128]} />
          <meshBasicMaterial visible={false} />
        </mesh>
        <BiomeSparkles rarity={rarity} />
        <BiomePostEffects
          effectType={effectType}
          effectIntensity={effectIntensity}
          effectCenter={effectCenter}
          bloomEnabled={bloomEnabled}
        />
      </Canvas>
    </div>
  );
}

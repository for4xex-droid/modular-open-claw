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
import { BiomeFieldRenderer } from './BiomeFieldRenderer';
import { BiomeSparkles } from './BiomeSparkles';
import { BiomePostEffects } from './BiomePostEffects';
import { BiomeCanvasProps } from './biomeTypes';
import { useState, useCallback, useRef } from 'react';

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
  structureBonus = false,
  injectionMarks: _injectionMarks = [],
  dragPaint = false,
}: BiomeCanvasProps) {
  void _injectionMarks;
  const [, setHoverCell] = useState<{ x: number; y: number } | null>(null);
  const isPaintingRef = useRef(false);
  const lastPaintedRef = useRef<{ x: number; y: number } | null>(null);

  const handlePointerMove = useCallback((e: any) => {
    const x = Math.floor(e.point.x);
    const y = Math.floor(e.point.y);
    if (x >= 0 && x < 128 && y >= 0 && y < 128) {
      // ドラッグ中はセルが変わるたびに塗る（環境ペン時のみ。種まきでの氾濫を防ぐ）
      if (dragPaint && isPaintingRef.current && onClick) {
        const last = lastPaintedRef.current;
        if (!last || last.x !== x || last.y !== y) {
          lastPaintedRef.current = { x, y };
          onClick({ x, y });
        }
      }
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
  }, [onHover, onClick, dragPaint]);

  const handlePointerLeave = useCallback(() => {
    isPaintingRef.current = false;
    lastPaintedRef.current = null;
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
      isPaintingRef.current = true;
      lastPaintedRef.current = { x, y };
      if (onClick) onClick({ x, y });
    }
  }, [onClick]);

  const handlePointerUp = useCallback(() => {
    isPaintingRef.current = false;
    lastPaintedRef.current = null;
  }, []);

  return (
    <div style={{
      width: `${width}px`,
      height: `${height}px`,
      position: 'relative',
      overflow: 'hidden',
    }}>
      <Canvas
        gl={{
          antialias: true,
          alpha: false,
          // preserveDrawingBuffer は Safari で多量のメモリを保持し OOM（タブ再読込）を
          // 誘発するため本番では無効。Playwright のピクセル読取時のみ ?e2e で有効化。
          preserveDrawingBuffer:
            typeof window !== 'undefined' && window.location.search.includes('e2e'),
        }}
        dpr={1}
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
        <BiomeFieldRenderer renderView={renderView} />
        {/* 入力判定Plane */}
        <mesh
          position={[64, 64, 1]}
          onPointerMove={handlePointerMove}
          onPointerDown={handlePointerDown}
          onPointerUp={handlePointerUp}
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
          structureBonus={structureBonus}
        />
      </Canvas>
    </div>
  );
}

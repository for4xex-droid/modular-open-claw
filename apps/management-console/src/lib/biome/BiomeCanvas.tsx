import { Canvas } from '@react-three/fiber';
import { OrthographicCamera } from '@react-three/drei';
import { BiomeBackground } from './BiomeBackground';
import { BiomeLighting } from './BiomeLighting';
import { BiomeCellGrid } from './BiomeCellGrid';
import { BiomeSparkles } from './BiomeSparkles';
import { BiomePostEffects } from './BiomePostEffects';
import { BiomeCanvasProps } from './biomeTypes';
import { useState } from 'react';

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

  const handlePointerMove = (e: any) => {
    // Canvas上のraycast交点からグリッド座標(0-127)を取得
    const x = Math.floor(e.point.x);
    const y = Math.floor(e.point.y);
    if (x >= 0 && x < 128 && y >= 0 && y < 128) {
      const coord = { x, y };
      if (!hoverCell || hoverCell.x !== x || hoverCell.y !== y) {
        setHoverCell(coord);
        if (onHover) onHover(coord);
      }
    } else {
      handlePointerLeave();
    }
  };

  const handlePointerLeave = () => {
    setHoverCell(null);
    if (onHover) onHover(null);
  };

  const handlePointerDown = (e: any) => {
    const x = Math.floor(e.point.x);
    const y = Math.floor(e.point.y);
    if (x >= 0 && x < 128 && y >= 0 && y < 128) {
      if (onClick) onClick({ x, y });
    }
  };

  return (
    <div style={{ width, height, position: 'relative' }}>
      <Canvas
        gl={{ antialias: true }}
      >
        <OrthographicCamera
          makeDefault
          left={0}
          right={128}
          top={128}
          bottom={0}
          near={0.1}
          far={1000}
          position={[0, 0, 10]}
          manual
        />
        <BiomeBackground />
        <BiomeLighting rarity={rarity} />
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


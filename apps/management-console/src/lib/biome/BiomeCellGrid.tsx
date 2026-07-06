/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import { useRef, useMemo, useEffect } from 'react';

import { useFrame, extend } from '@react-three/fiber';
import { shaderMaterial } from '@react-three/drei';
import * as THREE from 'three';
import { createCellGeometry } from './cellGeometries';
import { CELL_COUNT, RENDER_STRIDE, GRID_WIDTH, MORPH_COUNT, InjectionMark } from './biomeTypes';
import { biomeCellVertexShader, biomeCellFragmentShader } from './shaders/biomeCell';
import ThemeBridge from './ThemeBridge';
import { cssVar } from '../../utils/cssVar';

// --- shaderMaterial の定義 ---
const BiomeCellMaterial = shaderMaterial(
  {
    u_time: 0,
    u_morphType: 0,
    u_rarity: 0,
  },
  biomeCellVertexShader,
  biomeCellFragmentShader
);
extend({ BiomeCellMaterial });

interface BiomeCellGridProps {
  renderView: Float32Array;
  rarity: number;
  injectionMarks: InjectionMark[];
  hoverCell: { x: number; y: number } | null;
}

const PI = Math.PI;
const ELEMENT_KEYS = ['c', 'n', 'p', 'h', 'o', 's', 'fe', 'si'] as const;

// 8元素比率からHSLを導出してRGBカラーを返す (grid.frag L27-78 の移植)
function computeElementColor(tempColor: THREE.Color, renderView: Float32Array, offset: number) {
  const C = renderView[offset + 4];
  const N = renderView[offset + 5];
  const P = renderView[offset + 6];
  const H = renderView[offset + 7];
  const O = renderView[offset + 8];
  const S = renderView[offset + 9];
  const Fe = renderView[offset + 10];
  const Si = renderView[offset + 11];

  const total = C + N + P + H + O + S + Fe + Si;
  if (total < 1.0) {
    tempColor.copy(ThemeBridge.getElementColor('fallback'));
    return;
  }

  const c = C / total;
  const n = N / total;
  const p = P / total;
  const h = H / total;
  const o = O / total;
  const s = S / total;
  const fe = Fe / total;
  const si = Si / total;

  const vx = c * Math.cos(0.0)
           + n * Math.cos(PI * 0.25)
           + p * Math.cos(PI * 0.5)
           + h * Math.cos(PI * 0.75)
           + o * Math.cos(PI)
           + s * Math.cos(PI * 1.25)
           + fe * Math.cos(PI * 1.5)
           + si * Math.cos(PI * 1.75);

  const vy = c * Math.sin(0.0)
           + n * Math.sin(PI * 0.25)
           + p * Math.sin(PI * 0.5)
           + h * Math.sin(PI * 0.75)
           + o * Math.sin(PI)
           + s * Math.sin(PI * 1.25)
           + fe * Math.sin(PI * 1.5)
           + si * Math.sin(PI * 1.75);

  let hue = Math.atan2(vy, vx) / (2.0 * PI);
  if (hue < 0.0) hue += 1.0;

  const len = Math.sqrt(vx * vx + vy * vy);
  const sat = Math.max(0.0, Math.min(1.0, len * 0.8 + 0.2));

  // 明度は総量に応じて動的変化 (最大120,000として0.4〜0.7にクランプ)
  const l = Math.max(0.4, Math.min(0.7, total / 120000.0));

  tempColor.setHSL(hue, sat, l);
}

export function BiomeCellGrid({ renderView, rarity, injectionMarks, hoverCell }: BiomeCellGridProps) {
  // レアリティ変更時にジオメトリを差し替え
  const geometries = useMemo(() => {
    return Array.from({ length: MORPH_COUNT }, (_, morph) =>
      createCellGeometry(morph, rarity)
    );
  }, [rarity]);

  const meshRefs = useRef<(THREE.InstancedMesh | null)[]>(new Array(MORPH_COUNT).fill(null));
  const materialRefs = useRef<any[]>(new Array(MORPH_COUNT).fill(null));
  const hoverRingRef = useRef<THREE.Mesh | null>(null);
  const dummy = useMemo(() => new THREE.Object3D(), []);
  const tempColor = useMemo(() => new THREE.Color(), []);

  // マウント/ジオメトリ変更時に、全インスタンスの初期スケールを 0（非表示）にして 1フレーム目の残留バグを防止
  // さらに、マテリアルの vertexColors プロパティを確実に true に設定してシェーダーエラーを防ぐ
  useEffect(() => {
    const tempMatrix = new THREE.Matrix4().makeScale(0, 0, 0);
    for (let m = 0; m < MORPH_COUNT; m++) {
      const mesh = meshRefs.current[m];
      if (mesh && mesh.instanceMatrix) {
        for (let i = 0; i < CELL_COUNT; i++) {
          mesh.setMatrixAt(i, tempMatrix);
        }
        mesh.instanceMatrix.needsUpdate = true;
        mesh.count = 0;
      }

      const mat = materialRefs.current[m];
      if (mat) {
        mat.vertexColors = true;
        mat.needsUpdate = true;
      }
    }
  }, [rarity]);


  useFrame(({ clock }) => {

    if (!renderView || renderView.length === 0) return;
    const time = clock.getElapsedTime();
    const counts = new Array(MORPH_COUNT).fill(0);

    // マテリアルの time uniform 更新
    for (let m = 0; m < MORPH_COUNT; m++) {
      if (materialRefs.current[m]) {
        materialRefs.current[m].u_time = time;
      }
    }

    for (let i = 0; i < CELL_COUNT; i++) {
      const offset = i * RENDER_STRIDE;
      const cx = i % GRID_WIDTH;
      const cy = Math.floor(i / GRID_WIDTH);
      const active = renderView[offset + 2];
      const morph = Math.floor(renderView[offset + 3]);
      const isPrismatic = active > 1.5;

      if (active < 0.5 || morph < 0 || morph >= MORPH_COUNT) continue;

      const mesh = meshRefs.current[morph];
      if (!mesh) continue;
      const idx = counts[morph];

      dummy.position.set(cx + 0.5, cy + 0.5, 0);
      if (isPrismatic) {
        dummy.rotation.set(time * 1.2 + i * 0.02, time * 0.8 + i * 0.015, 0);
      } else if (rarity >= 3) {
        dummy.rotation.set(time * 0.5 + i * 0.01, time * 0.3 + i * 0.007, 0);
      } else {
        dummy.rotation.set(0, 0, 0);
      }
      const baseScale = 0.65 + rarity * 0.05;
      const scale = isPrismatic
        ? baseScale * (1.0 + 0.15 * Math.sin(time * 3 + i * 0.1))
        : baseScale;
      dummy.scale.setScalar(scale);
      dummy.updateMatrix();
      mesh.setMatrixAt(idx, dummy.matrix);

      // 色計算（Prismatic は虹彩 HSL で上書き）
      if (isPrismatic) {
        const hue = (time * 0.15 + i * 0.01) % 1.0;
        tempColor.setHSL(hue, 0.95, 0.65);
      } else {
        computeElementColor(tempColor, renderView, offset);
      }

      // 注入リップル
      for (const mark of injectionMarks) {
        const dist = Math.abs(cx - mark.x) + Math.abs(cy - mark.y);
        if (dist < 5) {
          const ripple = (1 - mark.age) * (1 - dist / 5) * 0.4;
          tempColor.lerp(ThemeBridge.getElementColor(ELEMENT_KEYS[mark.elementIdx] ?? 'fallback'), ripple);
        }
      }

      mesh.setColorAt(idx, tempColor);
      counts[morph] = idx + 1;
    }

    for (let m = 0; m < MORPH_COUNT; m++) {
      const mesh = meshRefs.current[m];
      if (!mesh) continue;
      mesh.count = counts[m];
      if (mesh.instanceMatrix) mesh.instanceMatrix.needsUpdate = true;
      if (mesh.instanceColor) mesh.instanceColor.needsUpdate = true;
    }

    // ホバーハイライト
    if (hoverRingRef.current) {
      if (hoverCell) {
        hoverRingRef.current.visible = true;
        hoverRingRef.current.position.set(hoverCell.x + 0.5, hoverCell.y + 0.5, 0.1);
        hoverRingRef.current.rotation.z = time * 2;
      } else {
        hoverRingRef.current.visible = false;
      }
    }
  });

  return (
    <group>

      {geometries.map((geo, morph) => (
        <instancedMesh
          key={`${morph}_${rarity}`}
          ref={(el) => { meshRefs.current[morph] = el; if (el) el.count = 0; }}
          args={[geo, undefined, CELL_COUNT]}
          frustumCulled={false}
        >
          {/* @ts-expect-error - drei shaderMaterial */}
          <biomeCellMaterial
            ref={(el: any) => { materialRefs.current[morph] = el; }}
            u_morphType={morph}
            u_rarity={rarity}
            vertexColors
          />
        </instancedMesh>
      ))}

      {/* ホバーハイライト */}
      <mesh ref={hoverRingRef} visible={false}>
        <ringGeometry args={[0.35, 0.5, 16]} />
        <meshBasicMaterial color={cssVar('--accent-cyan')} transparent opacity={0.7} side={THREE.DoubleSide} />
      </mesh>
    </group>
  );
}

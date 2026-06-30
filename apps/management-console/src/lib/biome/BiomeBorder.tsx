/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import { useRef, useMemo } from 'react';
import { useFrame } from '@react-three/fiber';
import * as THREE from 'three';

export function BiomeBorder() {
  const lineMaterialRef = useRef<THREE.LineBasicMaterial | null>(null);

  // 128x128 の枠線と四隅のコーナーマーク (L字) のジオメトリデータを構築
  const borderGeometry = useMemo(() => {
    const points: THREE.Vector3[] = [];
    
    // メインの枠線 (四角形ループ)
    // 境界を綺麗に見せるため、少しだけ外側 (幅128, 高さ128, 中心64, 64)
    const min = 0;
    const max = 128;

    // 四角形の辺の頂点
    points.push(new THREE.Vector3(min, min, 0.05));
    points.push(new THREE.Vector3(max, min, 0.05));
    
    points.push(new THREE.Vector3(max, min, 0.05));
    points.push(new THREE.Vector3(max, max, 0.05));
    
    points.push(new THREE.Vector3(max, max, 0.05));
    points.push(new THREE.Vector3(min, max, 0.05));
    
    points.push(new THREE.Vector3(min, max, 0.05));
    points.push(new THREE.Vector3(min, min, 0.05));

    // コーナーマークのL字 (長さ 6)
    const len = 6;
    const offset = 0.5; // 少し外側に離す

    // 左下
    points.push(new THREE.Vector3(min - offset, min - offset + len, 0.05));
    points.push(new THREE.Vector3(min - offset, min - offset, 0.05));
    points.push(new THREE.Vector3(min - offset, min - offset, 0.05));
    points.push(new THREE.Vector3(min - offset + len, min - offset, 0.05));

    // 右下
    points.push(new THREE.Vector3(max + offset, min - offset + len, 0.05));
    points.push(new THREE.Vector3(max + offset, min - offset, 0.05));
    points.push(new THREE.Vector3(max + offset, min - offset, 0.05));
    points.push(new THREE.Vector3(max + offset - len, min - offset, 0.05));

    // 右上
    points.push(new THREE.Vector3(max + offset, max + offset - len, 0.05));
    points.push(new THREE.Vector3(max + offset, max + offset, 0.05));
    points.push(new THREE.Vector3(max + offset, max + offset, 0.05));
    points.push(new THREE.Vector3(max + offset - len, max + offset, 0.05));

    // 左上
    points.push(new THREE.Vector3(min - offset, max + offset - len, 0.05));
    points.push(new THREE.Vector3(min - offset, max + offset, 0.05));
    points.push(new THREE.Vector3(min - offset, max + offset, 0.05));
    points.push(new THREE.Vector3(min - offset + len, max + offset, 0.05));

    return new THREE.BufferGeometry().setFromPoints(points);
  }, []);

  // 脈動アニメーション (不透明度を 0.4 から 0.8 の間でゆらゆらさせる)
  useFrame(({ clock }) => {
    if (lineMaterialRef.current) {
      const elapsed = clock.getElapsedTime();
      const pulse = 0.6 + Math.sin(elapsed * 2.5) * 0.2;
      lineMaterialRef.current.opacity = pulse;
    }
  });

  return (
    <lineSegments geometry={borderGeometry}>
      <lineBasicMaterial
        ref={lineMaterialRef}
        color="#00f0ff"
        transparent
        opacity={0.6}
        linewidth={2} // WebGL規格によっては1固定になるが定義しておく
        depthWrite={false}
      />
    </lineSegments>
  );
}

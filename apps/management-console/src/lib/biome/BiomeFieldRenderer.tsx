/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import { useRef, useMemo } from 'react';
import { useFrame } from '@react-three/fiber';
import * as THREE from 'three';
import { CELL_COUNT, GRID_HEIGHT, GRID_WIDTH, RENDER_STRIDE } from './biomeTypes';

const fieldVertexShader = /* glsl */ `
  varying vec2 vUv;
  void main() {
    vUv = uv;
    gl_Position = projectionMatrix * modelViewMatrix * vec4(position, 1.0);
  }
`;

const fieldFragmentShader = /* glsl */ `
  uniform sampler2D uField;
  uniform float uTime;
  varying vec2 vUv;

  vec3 hsl2rgb(vec3 hsl) {
    vec3 rgb = clamp(abs(mod(hsl.x * 6.0 + vec3(0.0, 4.0, 2.0), 6.0) - 3.0) - 1.0, 0.0, 1.0);
    return hsl.z + hsl.y * (rgb - 0.5) * (1.0 - abs(2.0 * hsl.z - 1.0));
  }

  void main() {
    vec4 tex = texture2D(uField, vUv);
    float a = tex.a;
    // alpha バンド: 0=空, 60=壁, 90=養分, 120=毒, 180=生命, 255=Prismatic
    if (a < 0.15) {
      gl_FragColor = vec4(0.02, 0.03, 0.06, 1.0);
      return;
    }
    // 環境ペン地形（生命より暗く、格子状の質感で「地形」と分かるように）
    if (a < 0.28) {
      // 壁: 青灰色
      gl_FragColor = vec4(0.30, 0.34, 0.40, 1.0);
      return;
    }
    if (a < 0.40) {
      // 養分: 深緑
      gl_FragColor = vec4(0.10, 0.32, 0.16, 1.0);
      return;
    }
    if (a < 0.55) {
      // 毒: 暗紫
      gl_FragColor = vec4(0.32, 0.08, 0.22, 1.0);
      return;
    }
    // 生命体
    float prismatic = step(0.9, a);
    vec3 rgb = tex.rgb;
    if (prismatic > 0.5) {
      float hue = fract(uTime * 0.15 + vUv.x * 2.0 + vUv.y);
      rgb = hsl2rgb(vec3(hue, 0.85, 0.55));
    } else {
      // 強度ベース配色（Lenia 3ch 同値時の atan(0,0) NaN を回避）
      float v = max(rgb.r, max(rgb.g, rgb.b));
      float spread = max(abs(rgb.r - rgb.g), max(abs(rgb.g - rgb.b), abs(rgb.r - rgb.b)));
      float hue = 0.52 + spread * 0.08;
      float sat = 0.55 + spread * 0.35;
      float lit = 0.25 + 0.6 * v;
      rgb = hsl2rgb(vec3(hue, sat, lit));
    }
    gl_FragColor = vec4(rgb, 1.0);
  }
`;

/**
 * render_buffer active スロットを RGBA8 alpha バンドにエンコードする。
 * 正値=生命（1 活性 / 2 Prismatic）、負値=環境ペン地形（-1 壁 / -2 養分 / -3 毒）。
 */
function encodeActiveAlpha(active: number): number {
  if (active >= 1.5) return 255; // Prismatic
  if (active >= 0.5) return 180; // 生命
  if (active <= -2.5) return 120; // 毒
  if (active <= -1.5) return 90; // 養分
  if (active <= -0.5) return 60; // 壁
  return 0; // 空
}

interface BiomeFieldRendererProps {
  renderView: Float32Array;
}

export function BiomeFieldRenderer({ renderView }: BiomeFieldRendererProps) {
  const textureRef = useRef<THREE.DataTexture | null>(null);
  const pixelBuffer = useMemo(() => new Uint8Array(GRID_WIDTH * GRID_HEIGHT * 4), []);

  const material = useMemo(
    () =>
      new THREE.ShaderMaterial({
        uniforms: {
          uField: { value: null as THREE.Texture | null },
          uTime: { value: 0 },
        },
        vertexShader: fieldVertexShader,
        fragmentShader: fieldFragmentShader,
      }),
    []
  );

  useFrame(({ clock }) => {
    if (!renderView.length) return;

    for (let i = 0; i < CELL_COUNT; i++) {
      const offset = i * RENDER_STRIDE;
      const px = i % GRID_WIDTH;
      const py = Math.floor(i / GRID_WIDTH);
      const ti = (py * GRID_WIDTH + px) * 4;

      const active = renderView[offset + 2];
      pixelBuffer[ti] = Math.round((renderView[offset + 4] / 65535) * 255);
      pixelBuffer[ti + 1] = Math.round((renderView[offset + 5] / 65535) * 255);
      pixelBuffer[ti + 2] = Math.round((renderView[offset + 6] / 65535) * 255);
      pixelBuffer[ti + 3] = encodeActiveAlpha(active);
    }

    if (!textureRef.current) {
      textureRef.current = new THREE.DataTexture(
        pixelBuffer,
        GRID_WIDTH,
        GRID_HEIGHT,
        THREE.RGBAFormat,
        THREE.UnsignedByteType
      );
      textureRef.current.minFilter = THREE.NearestFilter;
      textureRef.current.magFilter = THREE.NearestFilter;
      textureRef.current.needsUpdate = true;
      material.uniforms.uField.value = textureRef.current;
    } else {
      textureRef.current.needsUpdate = true;
    }

    material.uniforms.uTime.value = clock.getElapsedTime();
  });

  return (
    <mesh position={[GRID_WIDTH / 2, GRID_HEIGHT / 2, 0]}>
      <planeGeometry args={[GRID_WIDTH, GRID_HEIGHT]} />
      <primitive object={material} attach="material" />
    </mesh>
  );
}

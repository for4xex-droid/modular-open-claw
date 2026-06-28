/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import { useRef } from 'react';
import { useFrame, extend } from '@react-three/fiber';
import { shaderMaterial } from '@react-three/drei';

const BiomeBackgroundMaterial = shaderMaterial(
  {
    u_time: 0,
  },
  // Vertex Shader
  /* glsl */`
    varying vec2 vUv;
    void main() {
      vUv = uv;
      gl_Position = projectionMatrix * viewMatrix * vec4(position, 1.0);
    }
  `,
  // Fragment Shader
  /* glsl */`
    precision highp float;
    varying vec2 vUv;
    uniform float u_time;

    vec2 hash2(vec2 p) {
      p = vec2(dot(p, vec2(127.1, 311.7)), dot(p, vec2(269.5, 183.3)));
      return fract(sin(p) * 43758.5453);
    }

    float cellularNoise(vec2 x) {
      vec2 n = floor(x);
      vec2 f = fract(x);
      float m_dist = 8.0;
      for (int j = -1; j <= 1; j++) {
        for (int i = -1; i <= 1; i++) {
          vec2 g = vec2(float(i), float(j));
          vec2 o = hash2(n + g);
          // 時間で呼吸するアニメーション (grid.frag L127)
          o = 0.5 + 0.5 * sin(u_time * 0.4 + 6.2831 * o);
          vec2 r = g + o - f;
          float d = dot(r, r);
          if (d < m_dist) m_dist = d;
        }
      }
      return sqrt(m_dist);
    }

    void main() {
      float voronoi = cellularNoise(vUv * 18.0);
      vec3 bgBase = mix(vec3(0.012, 0.016, 0.026), vec3(0.02, 0.035, 0.055), (1.0 - voronoi) * 0.35);
      gl_FragColor = vec4(bgBase, 1.0);
    }
  `
);

extend({ BiomeBackgroundMaterial });

export function BiomeBackground() {
  const materialRef = useRef<any>(null);

  useFrame(({ clock }) => {
    if (materialRef.current) {
      materialRef.current.u_time = clock.getElapsedTime();
    }
  });

  return (
    <mesh position={[64, 64, -1]}>
      <planeGeometry args={[128, 128]} />
      {/* @ts-expect-error - drei shaderMaterial */}
      <biomeBackgroundMaterial ref={materialRef} depthWrite={false} />
    </mesh>
  );
}

/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import React, { useRef, useMemo } from 'react';
import { useFrame } from '@react-three/fiber';
import * as THREE from 'three';
import { cssVar } from '../../utils/cssVar';
import { parseColorToRGB } from '../../utils/colorUtils';

interface FluidAuraProps {
  avatarState: string;
}

const STATE_INTENSITY: Record<string, number> = {
  idle: 0.03,
  thinking: 0.10,
  speaking: 0.18,
  learning: 0.12,
  meditating: 0.05,
  awakened: 0.25
};

const FluidAura: React.FC<FluidAuraProps> = ({ avatarState }) => {
  const meshRef = useRef<THREE.Mesh>(null);
  const materialRef = useRef<THREE.ShaderMaterial>(null);

  // 1. カラーパレットの取得とパース (WebGL への cssVar ブリッジ)
  const colors = useMemo(() => {
    const ivoryStr = cssVar('--fluid-warm-ivory', '#d4c5a9');
    const goldStr = cssVar('--fluid-deep-gold', '#b8965a');
    return {
      c1: new THREE.Color(...parseColorToRGB(ivoryStr)),
      c2: new THREE.Color(...parseColorToRGB(goldStr))
    };
  }, []);

  // 2. ターゲットの強度を取得
  const targetIntensity = useMemo(() => {
    return STATE_INTENSITY[avatarState] ?? STATE_INTENSITY.idle;
  }, [avatarState]);

  // 3. カスタムシェーダーマテリアルの初期定義
  const shaderData = useMemo(() => {
    return {
      vertexShader: `
        varying vec2 vUv;
        void main() {
          vUv = uv;
          gl_Position = projectionMatrix * modelViewMatrix * vec4(position, 1.0);
        }
      `,
      fragmentShader: `
        uniform float uTime;
        uniform float uIntensity;
        uniform vec3 uColor1;
        uniform vec3 uColor2;
        varying vec2 vUv;

        // 疑似乱数・ノイズ関数
        float random (in vec2 _st) {
          return fract(sin(dot(_st.xy, vec2(12.9898,78.233))) * 43758.5453123);
        }

        float noise (in vec2 _st) {
          vec2 i = floor(_st);
          vec2 f = fract(_st);

          // Four corners in 2D of a tile
          float a = random(i);
          float b = random(i + vec2(1.0, 0.0));
          float c = random(i + vec2(0.0, 1.0));
          float d = random(i + vec2(1.0, 1.0));

          vec2 u = f * f * (3.0 - 2.0 * f);

          return mix(a, b, u.x) +
                  (c - a)* u.y * (1.0 - u.x) +
                  (d - b) * u.x * u.y;
        }

        void main() {
          vec2 center = vec2(0.5, 0.5);
          float dist = distance(vUv, center);

          // ドーナツ型（アバターの背後から噴き出すようなオーラ）のグラデーション
          float aura = smoothstep(0.5, 0.2, dist) * smoothstep(0.05, 0.25, dist);

          // 時間経過による流動ノイズ
          vec2 st = vUv * 4.0;
          float n = noise(st + vec2(uTime * 0.2, -uTime * 0.3));
          n += noise(st * 2.0 - vec2(uTime * 0.1, uTime * 0.15)) * 0.5;

          float glow = aura * (0.5 + 0.5 * n) * uIntensity;
          vec3 finalColor = mix(uColor1, uColor2, n);

          gl_FragColor = vec4(finalColor * glow, glow * 0.08);
        }
      `,
      uniforms: {
        uTime: { value: 0 },
        uIntensity: { value: STATE_INTENSITY.idle },
        uColor1: { value: colors.c1 },
        uColor2: { value: colors.c2 }
      }
    };
  }, [colors]);

  // 4. アニメーションフレーム更新 (useFrame 内で uTime/uIntensity を直にいじり、setState を回避。U-005 に準拠)
  useFrame((state) => {
    const material = materialRef.current;
    if (!material) return;

    // 時間更新
    material.uniforms.uTime.value = state.clock.getElapsedTime();

    // 強度のイージング (アバター状態の変化に合わせてじんわり遷移させる)
    const current = material.uniforms.uIntensity.value;
    const diff = targetIntensity - current;
    // 1フレームごとに少しずつ近づける
    material.uniforms.uIntensity.value += diff * 0.05;
  });

  return (
    <mesh ref={meshRef} position={[0, 0, -0.2]}>
      <planeGeometry args={[1.6, 1.6]} />
      <shaderMaterial
        ref={materialRef}
        vertexShader={shaderData.vertexShader}
        fragmentShader={shaderData.fragmentShader}
        uniforms={shaderData.uniforms}
        transparent={true}
        depthWrite={false}
        blending={THREE.AdditiveBlending}
      />
    </mesh>
  );
};

export default FluidAura;

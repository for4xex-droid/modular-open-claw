/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import React, { useEffect, useRef } from 'react';
import { useFluidConfig } from './useFluidConfig';

const FluidBackground: React.FC = () => {
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const config = useFluidConfig();

  useEffect(() => {
    // 1. 環境ガード (テスト環境/非表示)
    if (!config.enabled || typeof window === 'undefined') return;
    if (typeof WebGLRenderingContext === 'undefined') return;

    let cancelled = false;
    let animationFrameId: number | null = null;
    let teardownPointer: (() => void) | null = null;

    // WebGL リソースの参照保持用
    let renderer: any = null;
    let fluid: any = null;
    let material: any = null;
    let pass: any = null;

    const initFluid = async () => {
      const canvas = canvasRef.current;
      if (!canvas) return;

      // 2. three と three-fluid-fx の動的ロード
      const THREE = await import('three');
      const { FluidSimulation, attachPointerSplats, FullscreenPass, FULLSCREEN_VERTEX } = await import('three-fluid-fx');

      if (cancelled) return;

      // 3. WebGLRenderer 初期化
      renderer = new THREE.WebGLRenderer({
        canvas,
        antialias: false, // パフォーマンス重視
        alpha: false,
        depth: false,
        stencil: false,
        powerPreference: 'high-performance'
      });
      renderer.setPixelRatio(Math.min(window.devicePixelRatio, config.maxDpr || 1.5));

      const resize = () => {
        const width = canvas.clientWidth;
        const height = canvas.clientHeight;
        if (canvas.width !== width || canvas.height !== height) {
          renderer.setSize(width, height, false);
          fluid?.resize(width, height);
        }
      };

      // 4. Fluid Simulation の初期化
      fluid = new FluidSimulation(renderer, {
        simResolution: config.simResolution,
        dyeResolution: config.dyeResolution,
        splatRadius: 0.001,
        splatForce: 6,
        densityDissipation: 0.96, // 緩やかに消える
        velocityDissipation: 0.98
      });
      fluid.enableDye = true;

      // 初期サイズ合わせ
      resize();

      // 5. ポインターイベントアタッチ (暖色系カラーカスタマイズ)
      teardownPointer = attachPointerSplats(canvas, fluid, {
        coloredStrokes: true,
        colorize: () => {
          // 暖色系パレット (IVORY: #d4c5a9, GOLD: #b8965a) からランダムに補間
          const mix = Math.random();
          return [
            (212 * mix + 184 * (1 - mix)) / 255,
            (197 * mix + 150 * (1 - mix)) / 255,
            (169 * mix + 90 * (1 - mix)) / 255
          ];
        }
      });

      // 6. フルスクリーン描画用のカスタムシェーダーマテリアル
      material = new THREE.ShaderMaterial({
        vertexShader: FULLSCREEN_VERTEX,
        fragmentShader: `
          varying vec2 vUv;
          uniform sampler2D uTexture;
          void main() {
            vec4 color = texture2D(uTexture, vUv);
            // 密度に基づき背景をほんのり発光させる (黒地に暖色インク)
            gl_FragColor = vec4(color.rgb, 1.0);
          }
        `,
        uniforms: {
          uTexture: { value: null }
        },
        depthWrite: false,
        depthTest: false
      });

      pass = new FullscreenPass(material);

      // 7. リサイズイベントハンドラ
      window.addEventListener('resize', resize);

      // 8. アニメーションループ
      let lastTime = performance.now();
      const tick = (now: number) => {
        if (cancelled) return;
        
        const dt = Math.min((now - lastTime) / 1000, 0.1);
        lastTime = now;

        fluid.step(dt);
        material.uniforms.uTexture.value = fluid.dyeTexture;
        pass.render(renderer);

        animationFrameId = requestAnimationFrame(tick);
      };

      animationFrameId = requestAnimationFrame(tick);
    };

    initFluid();

    // クリーンアップ
    return () => {
      cancelled = true;
      if (animationFrameId !== null) {
        cancelAnimationFrame(animationFrameId);
      }
      if (teardownPointer) {
        teardownPointer();
      }
      window.removeEventListener('resize', () => {});

      // WebGL リソースの確実な廃棄
      if (pass) pass.dispose();
      if (material) material.dispose();
      if (fluid) fluid.dispose();
      if (renderer) {
        renderer.dispose();
        renderer.forceContextLoss();
      }
    };
  }, [config]);

  if (!config.enabled) return null;

  return (
    <canvas
      ref={canvasRef}
      style={{
        position: 'absolute',
        inset: 0,
        width: '100%',
        height: '100%',
        zIndex: 0,
        pointerEvents: 'auto',
        display: 'block'
      }}
    />
  );
};

export default FluidBackground;

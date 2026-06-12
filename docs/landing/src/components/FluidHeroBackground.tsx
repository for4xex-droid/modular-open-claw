import React, { useEffect, useRef } from 'react';
import type * as THREE from 'three';
import { useFluidConfig } from '../hooks/useFluidConfig';

export const FluidHeroBackground: React.FC = () => {
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const pointerRef = useRef<HTMLDivElement | null>(null);
  const config = useFluidConfig();

  useEffect(() => {
    // 1. 環境ガード (reduced-motion, SSR, WebGL非対応環境)
    if (!config.enabled || typeof window === 'undefined') return;
    if (typeof WebGLRenderingContext === 'undefined') return;

    let cancelled = false;
    let animationFrameId: number | null = null;
    let teardownPointer: (() => void) | null = null;

    let renderer: THREE.WebGLRenderer | null = null;
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    let fluid: any = null;
    let material: THREE.ShaderMaterial | null = null;
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    let pass: any = null;

    const initFluid = async () => {
      const canvas = canvasRef.current;
      const pointerTarget = pointerRef.current;
      if (!canvas || !pointerTarget) return;

      // 2. three と three-fluid-fx の動的ロード
      const THREE = await import('three');
      const { FluidSimulation, attachPointerSplats, FullscreenPass, FULLSCREEN_VERTEX } = await import('three-fluid-fx');

      if (cancelled) return;

      // 3. WebGLRenderer 初期化 — alpha: true でキャンバス背景を透明に
      renderer = new THREE.WebGLRenderer({
        canvas,
        antialias: false,
        alpha: true,
        depth: false,
        stencil: false,
        powerPreference: 'high-performance'
      });
      renderer.setClearColor(0x000000, 0); // 完全透明クリア
      renderer.setPixelRatio(Math.min(window.devicePixelRatio, config.maxDpr || 1.5));

      const resize = () => {
        const width = canvas.clientWidth;
        const height = canvas.clientHeight;
        if (canvas.width !== width || canvas.height !== height) {
          renderer!.setSize(width, height, false);
          fluid?.resize(width, height);
        }
      };

      // 4. Fluid Simulation の初期化
      fluid = new FluidSimulation(renderer, {
        simResolution: config.simResolution,
        dyeResolution: config.dyeResolution,
        splatRadius: 0.002,
        splatForce: 8,
        densityDissipation: 0.97,
        velocityDissipation: 0.98
      });
      fluid.enableDye = true;

      resize();

      // 5. ポインターイベント — canvas ではなく透明オーバーレイ (pointerRef) にアタッチ
      //    canvas は mix-blend-mode: screen + pointer-events: none のため、
      //    同じ bounding rect を持つ pointerRef でイベントを捕捉する
      teardownPointer = attachPointerSplats(pointerTarget, fluid, {
        coloredStrokes: true,
        colorize: () => {
          const mix = Math.random();
          const p = config.colors.primaryRGB;
          const s = config.colors.secondaryRGB;
          return [
            p[0] * mix + s[0] * (1 - mix),
            p[1] * mix + s[1] * (1 - mix),
            p[2] * mix + s[2] * (1 - mix)
          ];
        }
      });

      // 6. シェーダーマテリアル — アルファチャンネルでインク密度を出力
      material = new THREE.ShaderMaterial({
        vertexShader: FULLSCREEN_VERTEX,
        fragmentShader: `
          varying vec2 vUv;
          uniform sampler2D uTexture;
          void main() {
            vec4 color = texture2D(uTexture, vUv);
            float intensity = max(color.r, max(color.g, color.b));
            // インク密度に応じてアルファを設定 — 暗い部分は透明に
            gl_FragColor = vec4(color.rgb * 1.4, intensity * 0.85);
          }
        `,
        uniforms: {
          uTexture: { value: null }
        },
        depthWrite: false,
        depthTest: false,
        transparent: true
      });

      pass = new FullscreenPass(material);

      // 7. リサイズイベント登録
      window.addEventListener('resize', resize);

      // 8. ループ
      let lastTime = performance.now();
      const tick = (now: number) => {
        if (cancelled) return;
        
        const dt = Math.min((now - lastTime) / 1000, 0.1);
        lastTime = now;

        fluid.step(dt);
        material!.uniforms.uTexture.value = fluid.dyeTexture;
        renderer!.setClearColor(0x000000, 0);
        pass.render(renderer);

        animationFrameId = requestAnimationFrame(tick);
      };

      animationFrameId = requestAnimationFrame(tick);
    };

    initFluid();

    return () => {
      cancelled = true;
      if (animationFrameId !== null) {
        cancelAnimationFrame(animationFrameId);
      }
      if (teardownPointer) {
        teardownPointer();
      }
      
      // WebGL リソースとイベントリスナーの確実な廃棄
      if (pass) pass.dispose();
      if (material) material.dispose();
      if (fluid) fluid.dispose();
      if (renderer) {
        renderer.dispose();
        renderer.forceContextLoss();
      }
    };
  }, [config]);

  return (
    <>
      {/* Layer 0: 背景色ベース + CSS フォールバック (コンテンツの下) */}
      <div 
        data-testid="fluid-container"
        className="absolute inset-0 z-0 overflow-hidden bg-brand-bg"
      >
        <div 
          data-testid="fluid-fallback"
          className="absolute inset-0 opacity-80"
          style={{
            background: `
              radial-gradient(circle at 30% 30%, rgba(212, 197, 169, 0.18) 0%, transparent 55%),
              radial-gradient(circle at 70% 60%, rgba(184, 150, 90, 0.12) 0%, transparent 55%),
              radial-gradient(circle at 50% 80%, rgba(0, 242, 255, 0.06) 0%, transparent 50%)
            `
          }}
          aria-hidden="true"
        />
      </div>

      {/* Layer 2: WebGL キャンバスオーバーレイ — コンテンツの上に描画 */}
      {config.enabled && (
        <canvas
          ref={canvasRef}
          className="absolute inset-0 w-full h-full block z-20"
          style={{ 
            mixBlendMode: 'screen',
            pointerEvents: 'none'
          }}
          aria-hidden="true"
        />
      )}

      {/* Layer 3: 透明ポインターキャプチャ — ユーザーのマウス操作を検知 */}
      {config.enabled && (
        <div
          ref={pointerRef}
          className="absolute inset-0 z-30"
          style={{ pointerEvents: 'auto' }}
          aria-hidden="true"
        />
      )}
    </>
  );
};

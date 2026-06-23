// @legacy - This is the legacy WebGL2 custom renderer. It has been replaced by the React Three Fiber (R3F) BiomeCanvas.tsx. Keep this file for reference and backward compatibility of legacy tests.
import { useEffect, useRef } from 'react';
import { compileShader, createProgram, setupQuad } from './utils/webgl-helpers';
import { cssVar } from '../../utils/cssVar';
import { parseColorToRGB } from '../../utils/colorUtils';
import { canvasToGridCoords } from './utils/gridCoords';

// シェーダーソースの raw インポート (Vite の ?raw 機能)
import vertSource from './shaders/grid.vert?raw';
import fragSource from './shaders/grid.frag?raw';
import higgsSource from './shaders/higgs.frag?raw';
import tachyonSource from './shaders/tachyon.frag?raw';
import bloomSource from './shaders/bloom.frag?raw';

export interface CellInfo {
  x: number;
  y: number;
  active: boolean;
  morphology: number;
  elements: number[]; // 8元素: C, N, P, H, O, S, Fe, Si
}

export interface InjectionMark {
  x: number;
  y: number;
  age: number; // 0 ~ 1 (0=just injected, 1=fully faded)
  elementIdx: number;
}

export interface BiomeRendererProps {
  width: number;
  height: number;
  renderView: Float32Array;
  effectType?: 'none' | 'higgs' | 'tachyon';
  effectIntensity?: number;
  effectCenter?: [number, number];
  onClick?: (coord: { x: number; y: number }) => void;
  onHover?: (coord: { x: number; y: number } | null) => void;
  bloomEnabled?: boolean;
  injectionMarks?: InjectionMark[];
}

const GRID_WIDTH = 128;
const GRID_HEIGHT = 128;


// パススルー用の頂点シェーダーとコピー用のフラグメントシェーダー
const passthroughVert = `#version 300 es
in vec2 a_pos;
in vec2 a_uv;
out vec2 v_uv;
void main() {
    v_uv = a_uv;
    gl_Position = vec4(a_pos, 0.0, 1.0);
}
`;

const copyFrag = `#version 300 es
precision highp float;
in vec2 v_uv;
out vec4 fragColor;
uniform sampler2D u_tex;
void main() {
    vec3 c = texture(u_tex, v_uv).rgb;
    fragColor = vec4(c, 1.0);
}
`;



export function BiomeRenderer({
  width,
  height,
  renderView,
  effectType = 'none',
  effectIntensity = 0.0,
  effectCenter = [0.5, 0.5],
  onClick,
  onHover,
  bloomEnabled = false,
  injectionMarks = []
}: BiomeRendererProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const requestRef = useRef<number | null>(null);

  // WebGL リソース保持用の ref
  const glRef = useRef<WebGL2RenderingContext | null>(null);
  const programRef = useRef<WebGLProgram | null>(null);
  const quadVAORef = useRef<WebGLVertexArrayObject | null>(null);
  const instanceVboRef = useRef<WebGLBuffer | null>(null);
  const quadBufferRef = useRef<WebGLBuffer | null>(null);
  const gridTex0Ref = useRef<WebGLTexture | null>(null);
  const gridTex1Ref = useRef<WebGLTexture | null>(null);
  const gridTex2Ref = useRef<WebGLTexture | null>(null);
  const tex0DataRef = useRef<Float32Array | null>(null);
  const tex1DataRef = useRef<Float32Array | null>(null);
  const tex2DataRef = useRef<Float32Array | null>(null);

  // ポストプロセス用のリソース保持 ref
  const higgsProgramRef = useRef<WebGLProgram | null>(null);
  const tachyonProgramRef = useRef<WebGLProgram | null>(null);
  const copyProgramRef = useRef<WebGLProgram | null>(null);
  const bloomProgramRef = useRef<WebGLProgram | null>(null);
  const postQuadVAORef = useRef<WebGLVertexArrayObject | null>(null);

  // Framebuffer & Texture の ref
  const sceneFboRef = useRef<WebGLFramebuffer | null>(null);
  const sceneTexRef = useRef<WebGLTexture | null>(null);

  // Bloom用の FBO & Texture
  const bloomExtractFboRef = useRef<WebGLFramebuffer | null>(null);
  const bloomExtractTexRef = useRef<WebGLTexture | null>(null);
  const bloomBlurFboRef = useRef<WebGLFramebuffer | null>(null);
  const bloomBlurTexRef = useRef<WebGLTexture | null>(null);

  // タキオンピンポンバッファ
  const historyFbosRef = useRef<[WebGLFramebuffer | null, WebGLFramebuffer | null]>([null, null]);
  const historyTexsRef = useRef<[WebGLTexture | null, WebGLTexture | null]>([null, null]);
  const pingpongIdxRef = useRef<number>(0);

  // インタラクション用のホバー座標 ref
  const hoverCellRef = useRef<{ x: number; y: number } | null>(null);

  // パラメータ同期用の ref
  const renderViewRef = useRef<Float32Array>(renderView);
  const effectTypeRef = useRef<string>(effectType);
  const effectIntensityRef = useRef<number>(effectIntensity);
  const effectCenterRef = useRef<[number, number]>(effectCenter);
  const bloomEnabledRef = useRef<boolean>(bloomEnabled);
  const injectionMarksRef = useRef<InjectionMark[]>(injectionMarks);

  // テーマカラーのキャッシュ（render loop 内で getComputedStyle を呼ばないため）
  const primaryColorRef = useRef<[number, number, number]>([0.4, 0.7, 1.0]);
  const secondaryColorRef = useRef<[number, number, number]>([1.0, 0.4, 0.7]);

  // 最新パラメータを常に ref に同期
  useEffect(() => {
    renderViewRef.current = renderView;
  }, [renderView]);

  useEffect(() => {
    effectTypeRef.current = effectType;
  }, [effectType]);

  useEffect(() => {
    effectIntensityRef.current = effectIntensity;
  }, [effectIntensity]);

  useEffect(() => {
    effectCenterRef.current = effectCenter;
  }, [effectCenter]);

  useEffect(() => {
    bloomEnabledRef.current = bloomEnabled;
  }, [bloomEnabled]);

  // テーマカラーの解決（初期化時 + テーマ切替時のみ、render loop 外）
  useEffect(() => {
    const resolve = () => {
      const primaryStr = cssVar('--color-primary', '#66b2ff');
      const secondaryStr = cssVar('--color-secondary', '#ff66b2');
      primaryColorRef.current = parseColorToRGB(primaryStr);
      secondaryColorRef.current = parseColorToRGB(secondaryStr);
    };
    resolve();
    // テーマ切替を検知
    const observer = new MutationObserver(resolve);
    observer.observe(document.documentElement, { attributes: true, attributeFilter: ['class', 'data-theme'] });
    return () => observer.disconnect();
  }, []);

  useEffect(() => {
    injectionMarksRef.current = injectionMarks;
  }, [injectionMarks]);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;

    const gl = canvas.getContext('webgl2', {
      alpha: false,
      depth: false,
      stencil: false,
      antialias: true,
      premultipliedAlpha: false,
      preserveDrawingBuffer: true,
    });

    if (!gl) {
      console.error('WebGL2 not supported');
      return;
    }
    glRef.current = gl;

    // WebGL コンテキストロスト対策
    const handleContextLost = (e: Event) => {
      e.preventDefault();
      console.error('[BiomeRenderer] WebGL context lost!');
      if (requestRef.current) {
        cancelAnimationFrame(requestRef.current);
        requestRef.current = null;
      }
    };
    const handleContextRestored = () => {
      console.warn('[BiomeRenderer] WebGL context restored — page reload needed');
    };
    canvas.addEventListener('webglcontextlost', handleContextLost);
    canvas.addEventListener('webglcontextrestored', handleContextRestored);

    // FBO/テクスチャ生成ヘルパー
    const createFBOAndTexture = (w: number, h: number) => {
      const texture = gl.createTexture();
      if (!texture) {
        throw new Error('Failed to create WebGLTexture');
      }
      gl.bindTexture(gl.TEXTURE_2D, texture);
      gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, w, h, 0, gl.RGBA, gl.UNSIGNED_BYTE, null);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);

      const fbo = gl.createFramebuffer();
      if (!fbo) {
        gl.deleteTexture(texture);
        throw new Error('Failed to create WebGLFramebuffer');
      }
      gl.bindFramebuffer(gl.FRAMEBUFFER, fbo);
      gl.framebufferTexture2D(gl.FRAMEBUFFER, gl.COLOR_ATTACHMENT0, gl.TEXTURE_2D, texture, 0);

      gl.bindTexture(gl.TEXTURE_2D, null);
      gl.bindFramebuffer(gl.FRAMEBUFFER, null);

      return { fbo, texture };
    };

    // シェーダープログラムのコンパイルとリンク
    try {
      // 1. メイングリッド描画プログラム
      const vs = compileShader(gl, vertSource, gl.VERTEX_SHADER);
      const fs = compileShader(gl, fragSource, gl.FRAGMENT_SHADER);
      programRef.current = createProgram(gl, vs, fs);

      // 2. ポストプロセス用プログラム
      const postVs = compileShader(gl, passthroughVert, gl.VERTEX_SHADER);
      
      const copyFs = compileShader(gl, copyFrag, gl.FRAGMENT_SHADER);
      copyProgramRef.current = createProgram(gl, postVs, copyFs);

      const higgsFs = compileShader(gl, higgsSource, gl.FRAGMENT_SHADER);
      higgsProgramRef.current = createProgram(gl, postVs, higgsFs);

      const tachyonFs = compileShader(gl, tachyonSource, gl.FRAGMENT_SHADER);
      tachyonProgramRef.current = createProgram(gl, postVs, tachyonFs);

      const bloomFs = compileShader(gl, bloomSource, gl.FRAGMENT_SHADER);
      bloomProgramRef.current = createProgram(gl, postVs, bloomFs);

    } catch (err) {
      console.error('Shader compilation or linking failed', err);
      return;
    }

    // Quad (グリッド用ジオメトリ) バッファ設定
    const quadBuffer = setupQuad(gl);
    quadBufferRef.current = quadBuffer;

    // データテクスチャ用バッファの初期化
    if (!tex0DataRef.current) tex0DataRef.current = new Float32Array(GRID_WIDTH * GRID_HEIGHT * 4);
    if (!tex1DataRef.current) tex1DataRef.current = new Float32Array(GRID_WIDTH * GRID_HEIGHT * 4);
    if (!tex2DataRef.current) tex2DataRef.current = new Float32Array(GRID_WIDTH * GRID_HEIGHT * 4);

    // データテクスチャの作成
    const createDataTexture = () => {
      const texture = gl.createTexture();
      if (!texture) throw new Error('Failed to create WebGLTexture');
      gl.bindTexture(gl.TEXTURE_2D, texture);
      gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA32F, GRID_WIDTH, GRID_HEIGHT, 0, gl.RGBA, gl.FLOAT, null);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.NEAREST);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.NEAREST);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
      gl.bindTexture(gl.TEXTURE_2D, null);
      return texture;
    };

    try {
      gridTex0Ref.current = createDataTexture();
      gridTex1Ref.current = createDataTexture();
      gridTex2Ref.current = createDataTexture();
    } catch (err) {
      console.error('Failed to create data textures', err);
      return;
    }

    // ポストプロセス用の画面全体 Quad 設定
    const postQuadVao = gl.createVertexArray();
    if (postQuadVao) {
      gl.bindVertexArray(postQuadVao);
      gl.bindBuffer(gl.ARRAY_BUFFER, quadBuffer);
      
      gl.enableVertexAttribArray(0);
      gl.vertexAttribPointer(0, 2, gl.FLOAT, false, 16, 0);
      
      gl.enableVertexAttribArray(1);
      gl.vertexAttribPointer(1, 2, gl.FLOAT, false, 16, 8);
      
      gl.bindVertexArray(null);
      postQuadVAORef.current = postQuadVao;
    }

    // Framebuffer & テクスチャの初期化
    const scene = createFBOAndTexture(width, height);
    sceneFboRef.current = scene.fbo;
    sceneTexRef.current = scene.texture;

    const bloomExtract = createFBOAndTexture(width, height);
    bloomExtractFboRef.current = bloomExtract.fbo;
    bloomExtractTexRef.current = bloomExtract.texture;

    const bloomBlur = createFBOAndTexture(width, height);
    bloomBlurFboRef.current = bloomBlur.fbo;
    bloomBlurTexRef.current = bloomBlur.texture;

    const hist0 = createFBOAndTexture(width, height);
    const hist1 = createFBOAndTexture(width, height);
    historyFbosRef.current = [hist0.fbo, hist1.fbo];
    historyTexsRef.current = [hist0.texture, hist1.texture];

    // 初回クリア
    gl.viewport(0, 0, width, height);
    gl.clearColor(0.04, 0.04, 0.06, 1.0);

    const startTime = performance.now();

    // 60fps 描画ループ開始
    const renderLoop = () => {
      const gl = glRef.current;
      const program = programRef.current;
      const gridTex0 = gridTex0Ref.current;
      const gridTex1 = gridTex1Ref.current;
      const gridTex2 = gridTex2Ref.current;

      const higgsProgram = higgsProgramRef.current;
      const tachyonProgram = tachyonProgramRef.current;
      const copyProgram = copyProgramRef.current;
      const bloomProgram = bloomProgramRef.current;
      const postQuadVAO = postQuadVAORef.current;

      const sceneFbo = sceneFboRef.current;
      const sceneTex = sceneTexRef.current;

      if (!gl || !program || !gridTex0 || !gridTex1 || !gridTex2 || !copyProgram || !postQuadVAO || !sceneFbo || !sceneTex) {
        requestRef.current = requestAnimationFrame(renderLoop);
        return;
      }

      // コンテキストロスト時は描画しない
      if (gl.isContextLost?.()) {
        return;
      }

      const renderView = renderViewRef.current;
      const currentEffect = effectTypeRef.current;
      const intensity = effectIntensityRef.current;
      const center = effectCenterRef.current;
      const bloomEnabled = bloomEnabledRef.current;
      const time = (performance.now() - startTime) / 1000.0;

      // --- 1. データテクスチャの更新 ---
      if (renderView && renderView.length > 0) {
        const tex0Data = tex0DataRef.current;
        const tex1Data = tex1DataRef.current;
        const tex2Data = tex2DataRef.current;
        if (tex0Data && tex1Data && tex2Data) {
          const cellCount = GRID_WIDTH * GRID_HEIGHT;
          for (let i = 0; i < cellCount; i++) {
            const offset = i * 12;
            const dstOffset = i * 4;

            // tex0: R=cell_pos.x, G=cell_pos.y, B=active, A=morphology
            tex0Data[dstOffset + 0] = renderView[offset + 0];
            tex0Data[dstOffset + 1] = renderView[offset + 1];
            tex0Data[dstOffset + 2] = renderView[offset + 2];
            tex0Data[dstOffset + 3] = renderView[offset + 3];

            // tex1: C, N, P, H (元の packing 廃止)
            tex1Data[dstOffset + 0] = renderView[offset + 4]; // C
            tex1Data[dstOffset + 1] = renderView[offset + 5]; // N
            tex1Data[dstOffset + 2] = renderView[offset + 6]; // P
            tex1Data[dstOffset + 3] = renderView[offset + 7]; // H

            // tex2: O, S, Fe, Si
            tex2Data[dstOffset + 0] = renderView[offset + 8]; // O
            tex2Data[dstOffset + 1] = renderView[offset + 9]; // S
            tex2Data[dstOffset + 2] = renderView[offset + 10]; // Fe
            tex2Data[dstOffset + 3] = renderView[offset + 11]; // Si
          }

          gl.activeTexture(gl.TEXTURE2);
          gl.bindTexture(gl.TEXTURE_2D, gridTex0);
          gl.texSubImage2D(gl.TEXTURE_2D, 0, 0, 0, GRID_WIDTH, GRID_HEIGHT, gl.RGBA, gl.FLOAT, tex0Data);

          gl.activeTexture(gl.TEXTURE3);
          gl.bindTexture(gl.TEXTURE_2D, gridTex1);
          gl.texSubImage2D(gl.TEXTURE_2D, 0, 0, 0, GRID_WIDTH, GRID_HEIGHT, gl.RGBA, gl.FLOAT, tex1Data);

          gl.activeTexture(gl.TEXTURE4);
          gl.bindTexture(gl.TEXTURE_2D, gridTex2);
          gl.texSubImage2D(gl.TEXTURE_2D, 0, 0, 0, GRID_WIDTH, GRID_HEIGHT, gl.RGBA, gl.FLOAT, tex2Data);
        }
      }


      // --- 2. 描画先の選択 ---
      // bloom 無効時は FBO を介さず直接画面に描画（パフォーマンス向上 + 安定性）
      if (bloomEnabled) {
        gl.bindFramebuffer(gl.FRAMEBUFFER, sceneFbo);
      } else {
        gl.bindFramebuffer(gl.FRAMEBUFFER, null);
      }
      gl.viewport(0, 0, width, height);
      gl.clearColor(0.04, 0.04, 0.06, 1.0);
      gl.clear(gl.COLOR_BUFFER_BIT);

      gl.useProgram(program);
      gl.bindVertexArray(postQuadVAO);

      // グリッド uniform 設定
      const uGridSizeLoc = gl.getUniformLocation(program, 'u_grid_size');
      gl.uniform2f(uGridSizeLoc, GRID_WIDTH, GRID_HEIGHT);

      const uTimeLoc = gl.getUniformLocation(program, 'u_time');
      gl.uniform1f(uTimeLoc, time);

      const uHoverCellLoc = gl.getUniformLocation(program, 'u_hover_cell');
      if (hoverCellRef.current) {
        gl.uniform2f(uHoverCellLoc, hoverCellRef.current.x, hoverCellRef.current.y);
      } else {
        gl.uniform2f(uHoverCellLoc, -1.0, -1.0);
      }

      // テーマカラー（キャッシュ済み ref を使用、getComputedStyle は呼ばない）
      const primaryColor = primaryColorRef.current;
      const secondaryColor = secondaryColorRef.current;

      const uPrimaryLoc = gl.getUniformLocation(program, 'u_primary_color');
      gl.uniform3f(uPrimaryLoc, primaryColor[0], primaryColor[1], primaryColor[2]);
      const uSecondaryLoc = gl.getUniformLocation(program, 'u_secondary_color');
      gl.uniform3f(uSecondaryLoc, secondaryColor[0], secondaryColor[1], secondaryColor[2]);

      // 注入リップル uniform 設定
      const marks = injectionMarksRef.current;
      const injCount = Math.min(marks.length, 4);
      const uInjCountLoc = gl.getUniformLocation(program, 'u_injection_count');
      gl.uniform1i(uInjCountLoc, injCount);
      for (let i = 0; i < injCount; i++) {
        const m = marks[i];
        const uInjLoc = gl.getUniformLocation(program, `u_injection_centers[${i}]`);
        gl.uniform4f(uInjLoc, m.x, m.y, m.age, m.elementIdx);
      }

      // データテクスチャのバインド
      gl.activeTexture(gl.TEXTURE2);
      gl.bindTexture(gl.TEXTURE_2D, gridTex0);
      gl.uniform1i(gl.getUniformLocation(program, 'u_gridTex0'), 2);

      gl.activeTexture(gl.TEXTURE3);
      gl.bindTexture(gl.TEXTURE_2D, gridTex1);
      gl.uniform1i(gl.getUniformLocation(program, 'u_gridTex1'), 3);

      gl.activeTexture(gl.TEXTURE4);
      gl.bindTexture(gl.TEXTURE_2D, gridTex2);
      gl.uniform1i(gl.getUniformLocation(program, 'u_gridTex2'), 4);

      // フルスクリーン Quad 描画
      gl.drawArrays(gl.TRIANGLES, 0, 6);

      // bloom 無効時はここで描画完了（FBO → copy パスをスキップ）
      if (!bloomEnabled) {
        // クリーンアップ
        gl.bindVertexArray(null);
        gl.useProgram(null);
        requestRef.current = requestAnimationFrame(renderLoop);
        return;
      }

      // --- Bloom 処理 (bloom 有効時のみ) ---
      const bloomExtractFbo = bloomExtractFboRef.current;
      const bloomExtractTex = bloomExtractTexRef.current;
      const bloomBlurFbo = bloomBlurFboRef.current;
      const bloomBlurTex = bloomBlurTexRef.current;

      let bloomedTex = sceneTex;

      if (bloomEnabled && bloomProgram && bloomExtractFbo && bloomExtractTex && bloomBlurFbo && bloomBlurTex) {
        gl.bindVertexArray(postQuadVAO);

        // 1. 輝度抽出
        gl.bindFramebuffer(gl.FRAMEBUFFER, bloomExtractFbo);
        gl.viewport(0, 0, width, height);
        gl.clearColor(0.0, 0.0, 0.0, 1.0);
        gl.clear(gl.COLOR_BUFFER_BIT);
        gl.useProgram(bloomProgram);

        gl.activeTexture(gl.TEXTURE0);
        gl.bindTexture(gl.TEXTURE_2D, sceneTex);
        gl.uniform1i(gl.getUniformLocation(bloomProgram, 'u_sceneTex'), 0);
        gl.uniform1i(gl.getUniformLocation(bloomProgram, 'u_mode'), 0); // threshold
        gl.drawArrays(gl.TRIANGLES, 0, 6);

        // 2. 水平ブラー
        gl.bindFramebuffer(gl.FRAMEBUFFER, bloomBlurFbo);
        gl.clear(gl.COLOR_BUFFER_BIT);
        gl.activeTexture(gl.TEXTURE0);
        gl.bindTexture(gl.TEXTURE_2D, bloomExtractTex);
        gl.uniform1i(gl.getUniformLocation(bloomProgram, 'u_sceneTex'), 0);
        gl.uniform1i(gl.getUniformLocation(bloomProgram, 'u_mode'), 1); // horizontal
        gl.uniform2f(gl.getUniformLocation(bloomProgram, 'u_resolution'), width, height);
        gl.drawArrays(gl.TRIANGLES, 0, 6);

        // 3. 垂直ブラー (結果を bloomExtractFbo に書き戻す)
        gl.bindFramebuffer(gl.FRAMEBUFFER, bloomExtractFbo);
        gl.clear(gl.COLOR_BUFFER_BIT);
        gl.activeTexture(gl.TEXTURE0);
        gl.bindTexture(gl.TEXTURE_2D, bloomBlurTex);
        gl.uniform1i(gl.getUniformLocation(bloomProgram, 'u_sceneTex'), 0);
        gl.uniform1i(gl.getUniformLocation(bloomProgram, 'u_mode'), 2); // vertical
        gl.uniform2f(gl.getUniformLocation(bloomProgram, 'u_resolution'), width, height);
        gl.drawArrays(gl.TRIANGLES, 0, 6);

        // 4. 合成 (結果を bloomBlurFbo に書き込む)
        gl.bindFramebuffer(gl.FRAMEBUFFER, bloomBlurFbo);
        gl.clear(gl.COLOR_BUFFER_BIT);
        gl.activeTexture(gl.TEXTURE0);
        gl.bindTexture(gl.TEXTURE_2D, sceneTex);
        gl.uniform1i(gl.getUniformLocation(bloomProgram, 'u_sceneTex'), 0);

        gl.activeTexture(gl.TEXTURE1);
        gl.bindTexture(gl.TEXTURE_2D, bloomExtractTex); // 垂直ブラー済み
        gl.uniform1i(gl.getUniformLocation(bloomProgram, 'u_bloomTex'), 1);

        gl.uniform1i(gl.getUniformLocation(bloomProgram, 'u_mode'), 3); // composite
        gl.uniform1f(gl.getUniformLocation(bloomProgram, 'u_bloomIntensity'), 0.4);
        gl.drawArrays(gl.TRIANGLES, 0, 6);

        bloomedTex = bloomBlurTex;
      }

      // --- 3. ポストプロセス演出と画面へのコピー ---
      if (currentEffect === 'higgs' && higgsProgram) {
        // Higgs: 画面への描画
        gl.bindFramebuffer(gl.FRAMEBUFFER, null);
        gl.viewport(0, 0, width, height);
        gl.clearColor(0.04, 0.04, 0.06, 1.0);
        gl.clear(gl.COLOR_BUFFER_BIT);

        gl.useProgram(higgsProgram);
        gl.bindVertexArray(postQuadVAO);

        gl.activeTexture(gl.TEXTURE0);
        gl.bindTexture(gl.TEXTURE_2D, bloomedTex);
        gl.uniform1i(gl.getUniformLocation(higgsProgram, 'u_scene_tex'), 0);
        gl.uniform2f(gl.getUniformLocation(higgsProgram, 'u_impact_center'), center[0], center[1]);
        gl.uniform1f(gl.getUniformLocation(higgsProgram, 'u_time'), time);
        gl.uniform1f(gl.getUniformLocation(higgsProgram, 'u_intensity'), intensity);

        gl.drawArrays(gl.TRIANGLES, 0, 6);

      } else if (currentEffect === 'tachyon' && tachyonProgram) {
        // Tachyon: ピンポンバッファを用いたタイムトレイルエフェクト
        const prevIdx = pingpongIdxRef.current;
        const currIdx = 1 - prevIdx;

        const historyFbos = historyFbosRef.current;
        const historyTexs = historyTexsRef.current;

        const targetFbo = historyFbos[currIdx];
        const prevTex = historyTexs[prevIdx];
        const currTex = historyTexs[currIdx];

        if (targetFbo && prevTex && currTex) {
          gl.bindFramebuffer(gl.FRAMEBUFFER, targetFbo);
          gl.viewport(0, 0, width, height);
          gl.clearColor(0.0, 0.0, 0.0, 1.0);
          gl.clear(gl.COLOR_BUFFER_BIT);

          gl.useProgram(tachyonProgram);
          gl.bindVertexArray(postQuadVAO);

          gl.activeTexture(gl.TEXTURE0);
          gl.bindTexture(gl.TEXTURE_2D, bloomedTex);
          gl.uniform1i(gl.getUniformLocation(tachyonProgram, 'u_current_tex'), 0);

          gl.activeTexture(gl.TEXTURE1);
          gl.bindTexture(gl.TEXTURE_2D, prevTex);
          gl.uniform1i(gl.getUniformLocation(tachyonProgram, 'u_history_tex'), 1);

          gl.uniform1f(gl.getUniformLocation(tachyonProgram, 'u_blend_factor'), intensity);

          gl.drawArrays(gl.TRIANGLES, 0, 6);

          gl.bindFramebuffer(gl.FRAMEBUFFER, null);
          gl.viewport(0, 0, width, height);
          gl.clearColor(0.04, 0.04, 0.06, 1.0);
          gl.clear(gl.COLOR_BUFFER_BIT);

          gl.useProgram(copyProgram);
          gl.bindVertexArray(postQuadVAO);

          gl.activeTexture(gl.TEXTURE0);
          gl.bindTexture(gl.TEXTURE_2D, currTex);
          gl.uniform1i(gl.getUniformLocation(copyProgram, 'u_tex'), 0);

          gl.drawArrays(gl.TRIANGLES, 0, 6);

          pingpongIdxRef.current = currIdx;
        }
      } else {
        // デフォルト: パススルーコピーで直接画面描画
        gl.bindFramebuffer(gl.FRAMEBUFFER, null);
        gl.viewport(0, 0, width, height);
        gl.clearColor(0.04, 0.04, 0.06, 1.0);
        gl.clear(gl.COLOR_BUFFER_BIT);

        gl.useProgram(copyProgram);
        gl.bindVertexArray(postQuadVAO);

        gl.activeTexture(gl.TEXTURE0);
        gl.bindTexture(gl.TEXTURE_2D, bloomedTex);
        gl.uniform1i(gl.getUniformLocation(copyProgram, 'u_tex'), 0);

        gl.drawArrays(gl.TRIANGLES, 0, 6);
      }

      // クリーンアップ / バインド解除
      // テクスチャのバインド解除（次フレームで sceneFbo に描画する際の
      // フィードバックループを防止する）
      gl.activeTexture(gl.TEXTURE1);
      gl.bindTexture(gl.TEXTURE_2D, null);
      gl.activeTexture(gl.TEXTURE0);
      gl.bindTexture(gl.TEXTURE_2D, null);
      gl.bindVertexArray(null);
      gl.useProgram(null);

      requestRef.current = requestAnimationFrame(renderLoop);
    };

    requestRef.current = requestAnimationFrame(renderLoop);

    return () => {
      // クリーンアップ処理
      const g = glRef.current;
      if (requestRef.current) cancelAnimationFrame(requestRef.current);

      if (g) {
        // VAO & VBO の破棄
        if (quadVAORef.current) g.deleteVertexArray(quadVAORef.current);
        if (instanceVboRef.current) g.deleteBuffer(instanceVboRef.current);
        if (quadBufferRef.current) g.deleteBuffer(quadBufferRef.current);
        if (postQuadVAORef.current) g.deleteVertexArray(postQuadVAORef.current);

        // データテクスチャの破棄
        if (gridTex0Ref.current) g.deleteTexture(gridTex0Ref.current);
        if (gridTex1Ref.current) g.deleteTexture(gridTex1Ref.current);
        if (gridTex2Ref.current) g.deleteTexture(gridTex2Ref.current);
        gridTex0Ref.current = null;
        gridTex1Ref.current = null;
        gridTex2Ref.current = null;

        // プログラムの破棄
        if (programRef.current) g.deleteProgram(programRef.current);
        if (higgsProgramRef.current) g.deleteProgram(higgsProgramRef.current);
        if (tachyonProgramRef.current) g.deleteProgram(tachyonProgramRef.current);
        if (copyProgramRef.current) g.deleteProgram(copyProgramRef.current);
        if (bloomProgramRef.current) g.deleteProgram(bloomProgramRef.current);

        // FBO & テクスチャの破棄
        if (sceneFboRef.current) g.deleteFramebuffer(sceneFboRef.current);
        if (sceneTexRef.current) g.deleteTexture(sceneTexRef.current);
        if (bloomExtractFboRef.current) g.deleteFramebuffer(bloomExtractFboRef.current);
        if (bloomExtractTexRef.current) g.deleteTexture(bloomExtractTexRef.current);
        if (bloomBlurFboRef.current) g.deleteFramebuffer(bloomBlurFboRef.current);
        if (bloomBlurTexRef.current) g.deleteTexture(bloomBlurTexRef.current);

        const hFbos = historyFbosRef.current;
        const hTexs = historyTexsRef.current;
        if (hFbos[0]) g.deleteFramebuffer(hFbos[0]);
        if (hFbos[1]) g.deleteFramebuffer(hFbos[1]);
        if (hTexs[0]) g.deleteTexture(hTexs[0]);
        if (hTexs[1]) g.deleteTexture(hTexs[1]);
      }
      // コンテキストロストイベントのクリーンアップ
      if (canvas) {
        canvas.removeEventListener('webglcontextlost', handleContextLost);
        canvas.removeEventListener('webglcontextrestored', handleContextRestored);
      }
    };
  }, [width, height]);

  // マウスイベントハンドラ
  const handleCanvasClick = (e: React.MouseEvent<HTMLCanvasElement>) => {
    if (!canvasRef.current || !onClick) return;
    const coord = canvasToGridCoords(e.clientX, e.clientY, canvasRef.current);
    if (coord) {
      onClick(coord);
    }
  };

  const handleCanvasMouseMove = (e: React.MouseEvent<HTMLCanvasElement>) => {
    if (!canvasRef.current) return;
    const coord = canvasToGridCoords(e.clientX, e.clientY, canvasRef.current);
    if (coord) {
      hoverCellRef.current = coord;
      if (onHover) onHover(coord);
    } else {
      hoverCellRef.current = null;
      if (onHover) onHover(null);
    }
  };

  const handleCanvasMouseLeave = () => {
    hoverCellRef.current = null;
    if (onHover) onHover(null);
  };

  return (
    <canvas
      ref={canvasRef}
      width={width}
      height={height}
      onClick={handleCanvasClick}
      onMouseMove={handleCanvasMouseMove}
      onMouseLeave={handleCanvasMouseLeave}
      style={{
        display: 'block',
        width,
        height,
        background: 'var(--color-bg-biome, var(--bg-primary))',
        borderRadius: 'var(--radius-md, 8px)',
        boxShadow: 'var(--shadow-lg, var(--shadow-deep))',
        backdropFilter: 'blur(4px)',
        border: '1px solid var(--color-border-biome, var(--border-glass))',
        cursor: onClick ? 'crosshair' : 'default',
      }}
    />
  );
}

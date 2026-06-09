import { useEffect, useRef } from 'react';
import { compileShader, createProgram, setupQuad } from './utils/webgl-helpers';
import { cssVar } from '../../utils/cssVar';

// シェーダーソースの raw インポート (Vite の ?raw 機能)
import vertSource from './shaders/grid.vert?raw';
import fragSource from './shaders/grid.frag?raw';
import higgsSource from './shaders/higgs.frag?raw';
import tachyonSource from './shaders/tachyon.frag?raw';

export interface CellInfo {
  x: number;
  y: number;
  active: boolean;
  morphology: number;
  elements: number[]; // 8元素: C, N, P, H, O, S, Fe, Si
}

export interface BiomeRendererProps {
  width: number;
  height: number;
  cells: CellInfo[];
  effectType?: 'none' | 'higgs' | 'tachyon';
  effectIntensity?: number;
  effectCenter?: [number, number];
}

const GRID_WIDTH = 128;
const GRID_HEIGHT = 128;
const INSTANCE_COUNT = GRID_WIDTH * GRID_HEIGHT;

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
    fragColor = texture(u_tex, v_uv);
}
`;

function parseColorToRGB(colorStr: string): [number, number, number] {
  if (colorStr.startsWith('#')) {
    const hex = colorStr.slice(1);
    if (hex.length === 3) {
      const r = parseInt(hex[0] + hex[0], 16) / 255;
      const g = parseInt(hex[1] + hex[1], 16) / 255;
      const b = parseInt(hex[2] + hex[2], 16) / 255;
      return [r, g, b];
    }
    if (hex.length === 6) {
      const r = parseInt(hex.slice(0, 2), 16) / 255;
      const g = parseInt(hex.slice(2, 4), 16) / 255;
      const b = parseInt(hex.slice(4, 6), 16) / 255;
      return [r, g, b];
    }
  }
  const rgbMatch = colorStr.match(/(?:rgb|rgba)\s*\(\s*(\d+)\s*,\s*(\d+)\s*,\s*(\d+)/i);
  if (rgbMatch) {
    return [
      parseInt(rgbMatch[1], 10) / 255,
      parseInt(rgbMatch[2], 10) / 255,
      parseInt(rgbMatch[3], 10) / 255
    ];
  }
  return [1.0, 1.0, 1.0];
}

export function BiomeRenderer({
  width,
  height,
  cells,
  effectType = 'none',
  effectIntensity = 0.0,
  effectCenter = [0.5, 0.5]
}: BiomeRendererProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const requestRef = useRef<number | null>(null);

  // WebGL リソース保持用の ref
  const glRef = useRef<WebGL2RenderingContext | null>(null);
  const programRef = useRef<WebGLProgram | null>(null);
  const quadVAORef = useRef<WebGLVertexArrayObject | null>(null);
  const instanceVboRef = useRef<WebGLBuffer | null>(null);
  const quadBufferRef = useRef<WebGLBuffer | null>(null);

  // ポストプロセス用のリソース保持 ref
  const higgsProgramRef = useRef<WebGLProgram | null>(null);
  const tachyonProgramRef = useRef<WebGLProgram | null>(null);
  const copyProgramRef = useRef<WebGLProgram | null>(null);
  const postQuadVAORef = useRef<WebGLVertexArrayObject | null>(null);

  // Framebuffer & Texture の ref
  const sceneFboRef = useRef<WebGLFramebuffer | null>(null);
  const sceneTexRef = useRef<WebGLTexture | null>(null);

  // タキオンピンポンバッファ
  const historyFbosRef = useRef<[WebGLFramebuffer | null, WebGLFramebuffer | null]>([null, null]);
  const historyTexsRef = useRef<[WebGLTexture | null, WebGLTexture | null]>([null, null]);
  const pingpongIdxRef = useRef<number>(0);

  // パラメータ同期用の ref
  const cellsRef = useRef<CellInfo[]>(cells);
  const effectTypeRef = useRef<string>(effectType);
  const effectIntensityRef = useRef<number>(effectIntensity);
  const effectCenterRef = useRef<[number, number]>(effectCenter);

  // 最新パラメータを常に ref に同期
  useEffect(() => {
    cellsRef.current = cells;
  }, [cells]);

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
    const canvas = canvasRef.current;
    if (!canvas) return;

    const gl = canvas.getContext('webgl2', {
      alpha: false,
      depth: false,
      stencil: false,
      antialias: true,
      premultipliedAlpha: false,
      preserveDrawingBuffer: false,
    });

    if (!gl) {
      console.error('WebGL2 not supported');
      return;
    }
    glRef.current = gl;

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

    } catch (err) {
      console.error('Shader compilation or linking failed', err);
      return;
    }

    // Quad (グリッド用ジオメトリ) バッファ設定
    const quadBuffer = setupQuad(gl);
    quadBufferRef.current = quadBuffer;

    // VAO (Vertex Array Object) 作成 (グリッドインスタンス描画用)
    const vao = gl.createVertexArray();
    if (!vao) return;
    gl.bindVertexArray(vao);
    quadVAORef.current = vao;

    gl.bindBuffer(gl.ARRAY_BUFFER, quadBuffer);
    
    // a_pos (location = 0)
    gl.enableVertexAttribArray(0);
    gl.vertexAttribPointer(0, 2, gl.FLOAT, false, 16, 0);
    
    // a_uv (location = 1)
    gl.enableVertexAttribArray(1);
    gl.vertexAttribPointer(1, 2, gl.FLOAT, false, 16, 8);

    // インスタンスデータ用バッファ (VBO) の作成
    const instanceVbo = gl.createBuffer();
    if (!instanceVbo) return;
    instanceVboRef.current = instanceVbo;
    gl.bindBuffer(gl.ARRAY_BUFFER, instanceVbo);

    const stride = 48; // 12 float * 4 bytes

    // a_cell_pos (location = 2)
    gl.enableVertexAttribArray(2);
    gl.vertexAttribPointer(2, 2, gl.FLOAT, false, stride, 0);
    gl.vertexAttribDivisor(2, 1);

    // a_state (location = 3)
    gl.enableVertexAttribArray(3);
    gl.vertexAttribPointer(3, 2, gl.FLOAT, false, stride, 8);
    gl.vertexAttribDivisor(3, 1);

    // a_elements (location = 4)
    gl.enableVertexAttribArray(4);
    gl.vertexAttribPointer(4, 4, gl.FLOAT, false, stride, 16);
    gl.vertexAttribDivisor(4, 1);

    // a_elements_extra (location = 5)
    gl.enableVertexAttribArray(5);
    gl.vertexAttribPointer(5, 4, gl.FLOAT, false, stride, 32);
    gl.vertexAttribDivisor(5, 1);

    gl.bindVertexArray(null);

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
      const vao = quadVAORef.current;
      const instanceVbo = instanceVboRef.current;

      const higgsProgram = higgsProgramRef.current;
      const tachyonProgram = tachyonProgramRef.current;
      const copyProgram = copyProgramRef.current;
      const postQuadVAO = postQuadVAORef.current;

      const sceneFbo = sceneFboRef.current;
      const sceneTex = sceneTexRef.current;

      if (!gl || !program || !vao || !instanceVbo || !copyProgram || !postQuadVAO || !sceneFbo || !sceneTex) {
        requestRef.current = requestAnimationFrame(renderLoop);
        return;
      }

      const activeCells = cellsRef.current;
      const currentEffect = effectTypeRef.current;
      const intensity = effectIntensityRef.current;
      const center = effectCenterRef.current;
      const time = (performance.now() - startTime) / 1000.0;

      // --- 1. インスタンスデータバッファの更新 ---
      const instanceData = new Float32Array(INSTANCE_COUNT * 12);
      for (let i = 0; i < INSTANCE_COUNT; i++) {
        const offset = i * 12;
        const cell = activeCells[i];

        if (cell) {
          instanceData[offset] = cell.x;
          instanceData[offset + 1] = cell.y;
          instanceData[offset + 2] = cell.active ? 1.0 : 0.0;
          instanceData[offset + 3] = cell.morphology;
          instanceData[offset + 4] = cell.elements[0] || 0.0;
          instanceData[offset + 5] = cell.elements[1] || 0.0;
          instanceData[offset + 6] = cell.elements[2] || 0.0;
          instanceData[offset + 7] = cell.elements[3] || 0.0;
          instanceData[offset + 8] = cell.elements[4] || 0.0;
          instanceData[offset + 9] = cell.elements[5] || 0.0;
          instanceData[offset + 10] = cell.elements[6] || 0.0;
          instanceData[offset + 11] = cell.elements[7] || 0.0;
        } else {
          const x = i % GRID_WIDTH;
          const y = Math.floor(i / GRID_WIDTH);
          instanceData[offset] = x;
          instanceData[offset + 1] = y;
          instanceData[offset + 2] = 0.0;
        }
      }
      gl.bindBuffer(gl.ARRAY_BUFFER, instanceVbo);
      gl.bufferData(gl.ARRAY_BUFFER, instanceData, gl.DYNAMIC_DRAW);

      // --- 2. Scene FBO への描画 (オフスクリーン) ---
      gl.bindFramebuffer(gl.FRAMEBUFFER, sceneFbo);
      gl.viewport(0, 0, width, height);
      gl.clearColor(0.04, 0.04, 0.06, 1.0);
      gl.clear(gl.COLOR_BUFFER_BIT);

      gl.useProgram(program);
      gl.bindVertexArray(vao);

      // グリッド uniform 設定
      const uGridSizeLoc = gl.getUniformLocation(program, 'u_grid_size');
      gl.uniform2f(uGridSizeLoc, GRID_WIDTH, GRID_HEIGHT);

      // CSS テーマカラーの解決
      const primaryColorStr = cssVar('--color-primary', '#66b2ff');
      const secondaryColorStr = cssVar('--color-secondary', '#ff66b2');
      const primaryColor = parseColorToRGB(primaryColorStr);
      const secondaryColor = parseColorToRGB(secondaryColorStr);

      const uPrimaryLoc = gl.getUniformLocation(program, 'u_primary_color');
      gl.uniform3f(uPrimaryLoc, primaryColor[0], primaryColor[1], primaryColor[2]);
      const uSecondaryLoc = gl.getUniformLocation(program, 'u_secondary_color');
      gl.uniform3f(uSecondaryLoc, secondaryColor[0], secondaryColor[1], secondaryColor[2]);

      // インスタンス描画
      gl.drawArraysInstanced(gl.TRIANGLES, 0, 6, INSTANCE_COUNT);

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
        gl.bindTexture(gl.TEXTURE_2D, sceneTex);
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
          // 履歴バッファへのブレンド描画
          gl.bindFramebuffer(gl.FRAMEBUFFER, targetFbo);
          gl.viewport(0, 0, width, height);
          gl.clearColor(0.0, 0.0, 0.0, 1.0);
          gl.clear(gl.COLOR_BUFFER_BIT);

          gl.useProgram(tachyonProgram);
          gl.bindVertexArray(postQuadVAO);

          // u_current_tex (sceneTex) -> TEXTURE0
          gl.activeTexture(gl.TEXTURE0);
          gl.bindTexture(gl.TEXTURE_2D, sceneTex);
          gl.uniform1i(gl.getUniformLocation(tachyonProgram, 'u_current_tex'), 0);

          // u_history_tex (prevTex) -> TEXTURE1
          gl.activeTexture(gl.TEXTURE1);
          gl.bindTexture(gl.TEXTURE_2D, prevTex);
          gl.uniform1i(gl.getUniformLocation(tachyonProgram, 'u_history_tex'), 1);

          // u_blend_factor (残像度合い)
          gl.uniform1f(gl.getUniformLocation(tachyonProgram, 'u_blend_factor'), intensity);

          gl.drawArrays(gl.TRIANGLES, 0, 6);

          // メインスクリーン（Canvas）にブレンド結果を描画
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

          // インデックススワップ
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
        gl.bindTexture(gl.TEXTURE_2D, sceneTex);
        gl.uniform1i(gl.getUniformLocation(copyProgram, 'u_tex'), 0);

        gl.drawArrays(gl.TRIANGLES, 0, 6);
      }

      // クリーンアップ / バインド解除
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

        // プログラムの破棄
        if (programRef.current) g.deleteProgram(programRef.current);
        if (higgsProgramRef.current) g.deleteProgram(higgsProgramRef.current);
        if (tachyonProgramRef.current) g.deleteProgram(tachyonProgramRef.current);
        if (copyProgramRef.current) g.deleteProgram(copyProgramRef.current);

        // FBO & テクスチャの破棄
        if (sceneFboRef.current) g.deleteFramebuffer(sceneFboRef.current);
        if (sceneTexRef.current) g.deleteTexture(sceneTexRef.current);

        const hFbos = historyFbosRef.current;
        const hTexs = historyTexsRef.current;
        if (hFbos[0]) g.deleteFramebuffer(hFbos[0]);
        if (hFbos[1]) g.deleteFramebuffer(hFbos[1]);
        if (hTexs[0]) g.deleteTexture(hTexs[0]);
        if (hTexs[1]) g.deleteTexture(hTexs[1]);
      }
    };
  }, [width, height]);

  return (
    <canvas
      ref={canvasRef}
      width={width}
      height={height}
      style={{
        display: 'block',
        width,
        height,
        background: 'var(--color-bg-biome, #0a0a0f)',
        borderRadius: 'var(--radius-md, 8px)',
        boxShadow: 'var(--shadow-lg, 0 8px 32px 0 rgba(0, 0, 0, 0.37))',
        backdropFilter: 'blur(4px)',
        border: '1px solid var(--color-border-biome, rgba(255, 255, 255, 0.08))',
      }}
    />
  );
}

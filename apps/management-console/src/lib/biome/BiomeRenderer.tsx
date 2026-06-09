import { useEffect, useRef } from 'react';
import { compileShader, createProgram, setupQuad } from './utils/webgl-helpers';

// シェーダーソースの raw インポート (Vite の ?raw 機能)
import vertSource from './shaders/grid.vert?raw';
import fragSource from './shaders/grid.frag?raw';

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
}

const GRID_WIDTH = 128;
const GRID_HEIGHT = 128;
const INSTANCE_COUNT = GRID_WIDTH * GRID_HEIGHT;

export function BiomeRenderer({ width, height, cells }: BiomeRendererProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const requestRef = useRef<number | null>(null);

  // WebGL リソース保持用の ref
  const glRef = useRef<WebGL2RenderingContext | null>(null);
  const programRef = useRef<WebGLProgram | null>(null);
  const quadVAORef = useRef<WebGLVertexArrayObject | null>(null);
  
  // インスタンスデータ用 VBO の ref
  const instanceVboRef = useRef<WebGLBuffer | null>(null);

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

    // シェーダーのコンパイルとリンク
    let program: WebGLProgram;
    try {
      const vs = compileShader(gl, vertSource, gl.VERTEX_SHADER);
      const fs = compileShader(gl, fragSource, gl.FRAGMENT_SHADER);
      program = createProgram(gl, vs, fs);
      programRef.current = program;
    } catch (err) {
      console.error('Shader link error', err);
      return;
    }

    // Quad (ジオメトリ) バッファ設定
    const quadBuffer = setupQuad(gl);

    // VAO (Vertex Array Object) 作成
    const vao = gl.createVertexArray();
    if (!vao) return;
    gl.bindVertexArray(vao);
    quadVAORef.current = vao;

    // ジオメトリ属性のバインド
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

    // インスタンス属性の設定
    // 属性のレイアウト:
    // a_cell_pos (location = 2) -> vec2 (8 bytes)
    // a_state (location = 3)    -> vec2 (8 bytes)
    // a_elements (location = 4) -> vec4 (16 bytes)
    // a_elements_extra (location = 5) -> vec4 (16 bytes)
    // ストライド: 8 + 8 + 16 + 16 = 48 bytes
    const stride = 48;

    // a_cell_pos
    gl.enableVertexAttribArray(2);
    gl.vertexAttribPointer(2, 2, gl.FLOAT, false, stride, 0);
    gl.vertexAttribDivisor(2, 1); // インスタンス毎に更新

    // a_state
    gl.enableVertexAttribArray(3);
    gl.vertexAttribPointer(3, 2, gl.FLOAT, false, stride, 8);
    gl.vertexAttribDivisor(3, 1);

    // a_elements
    gl.enableVertexAttribArray(4);
    gl.vertexAttribPointer(4, 4, gl.FLOAT, false, stride, 16);
    gl.vertexAttribDivisor(4, 1);

    // a_elements_extra
    gl.enableVertexAttribArray(5);
    gl.vertexAttribPointer(5, 4, gl.FLOAT, false, stride, 32);
    gl.vertexAttribDivisor(5, 1);

    gl.bindVertexArray(null);

    // 初期クリア
    gl.viewport(0, 0, width, height);
    gl.clearColor(0.04, 0.04, 0.06, 1.0); // 深いダークブルー

    return () => {
      // クリーンアップ処理 (メモリリーク防止)
      const g = glRef.current;
      if (g) {
        if (quadVAORef.current) g.deleteVertexArray(quadVAORef.current);
        if (instanceVboRef.current) g.deleteBuffer(instanceVboRef.current);
        if (programRef.current) g.deleteProgram(programRef.current);
      }
      if (requestRef.current) {
        cancelAnimationFrame(requestRef.current);
      }
    };
  }, [width, height]);

  // セルデータの転送と描画
  useEffect(() => {
    const gl = glRef.current;
    const program = programRef.current;
    const vao = quadVAORef.current;
    const instanceVbo = instanceVboRef.current;

    if (!gl || !program || !vao || !instanceVbo) return;

    // インスタンスデータ配列 (Float32Array) の作成
    // 1セルあたり 12 floats (48 bytes)
    const instanceData = new Float32Array(INSTANCE_COUNT * 12);

    for (let i = 0; i < INSTANCE_COUNT; i++) {
      const offset = i * 12;
      const cell = cells[i];

      if (cell) {
        // a_cell_pos (x, y)
        instanceData[offset] = cell.x;
        instanceData[offset + 1] = cell.y;
        
        // a_state (active, morphology)
        instanceData[offset + 2] = cell.active ? 1.0 : 0.0;
        instanceData[offset + 3] = cell.morphology;

        // a_elements (C, N, P, H)
        instanceData[offset + 4] = cell.elements[0] || 0.0;
        instanceData[offset + 5] = cell.elements[1] || 0.0;
        instanceData[offset + 6] = cell.elements[2] || 0.0;
        instanceData[offset + 7] = cell.elements[3] || 0.0;

        // a_elements_extra (O, S, Fe, Si)
        instanceData[offset + 8] = cell.elements[4] || 0.0;
        instanceData[offset + 9] = cell.elements[5] || 0.0;
        instanceData[offset + 10] = cell.elements[6] || 0.0;
        instanceData[offset + 11] = cell.elements[7] || 0.0;
      } else {
        // ダミー
        const x = i % GRID_WIDTH;
        const y = Math.floor(i / GRID_WIDTH);
        instanceData[offset] = x;
        instanceData[offset + 1] = y;
        instanceData[offset + 2] = 0.0; // inactive
      }
    }

    // VBO へインスタンスデータを転送
    gl.bindBuffer(gl.ARRAY_BUFFER, instanceVbo);
    gl.bufferData(gl.ARRAY_BUFFER, instanceData, gl.DYNAMIC_DRAW);

    // 描画処理
    const renderFrame = () => {
      gl.clear(gl.COLOR_BUFFER_BIT);

      gl.useProgram(program);
      gl.bindVertexArray(vao);

      // uniform 変数の設定
      const uGridSizeLoc = gl.getUniformLocation(program, 'u_grid_size');
      gl.uniform2f(uGridSizeLoc, GRID_WIDTH, GRID_HEIGHT);

      // TODO: cssVarブリッジから取得したテーマカラーを uniform に設定する
      const uPrimaryLoc = gl.getUniformLocation(program, 'u_primary_color');
      gl.uniform3f(uPrimaryLoc, 0.4, 0.7, 1.0); // 代替値 (ライトブルー)
      const uSecondaryLoc = gl.getUniformLocation(program, 'u_secondary_color');
      gl.uniform3f(uSecondaryLoc, 1.0, 0.4, 0.7); // 代替値 (ピンク)

      // インスタンス描画
      gl.drawArraysInstanced(gl.TRIANGLES, 0, 6, INSTANCE_COUNT);

      gl.bindVertexArray(null);
      gl.useProgram(null);
    };

    // 描画フレームをスケジュール
    requestRef.current = requestAnimationFrame(renderFrame);

    return () => {
      if (requestRef.current) {
        cancelAnimationFrame(requestRef.current);
      }
    };
  }, [cells]);

  return (
    <canvas
      ref={canvasRef}
      width={width}
      height={height}
      style={{
        display: 'block',
        width,
        height,
        background: '#0a0a0f',
        borderRadius: '8px',
        boxShadow: '0 8px 32px 0 rgba(0, 0, 0, 0.37)',
        backdropFilter: 'blur(4px)',
        border: '1px solid rgba(255, 255, 255, 0.08)',
      }}
    />
  );
}

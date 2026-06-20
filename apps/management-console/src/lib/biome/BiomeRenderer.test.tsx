import React from 'react';
import { render } from '@testing-library/react';

// Vite の ?raw クエリを解決するための Jest バーチャルモック
jest.mock('./shaders/grid.vert?raw', () => 'void main() {}', { virtual: true });
jest.mock('./shaders/grid.frag?raw', () => 'void main() {}', { virtual: true });
jest.mock('./shaders/higgs.frag?raw', () => 'void main() {}', { virtual: true });
jest.mock('./shaders/tachyon.frag?raw', () => 'void main() {}', { virtual: true });
jest.mock('./shaders/bloom.frag?raw', () => 'void main() {}', { virtual: true });

import { BiomeRenderer } from './BiomeRenderer';

// HTMLCanvasElement.prototype.getContext のモック
const mockGetContext = jest.fn().mockReturnValue({
  createShader: jest.fn().mockReturnValue({}),
  shaderSource: jest.fn(),
  compileShader: jest.fn(),
  getShaderParameter: jest.fn().mockReturnValue(true),
  createProgram: jest.fn().mockReturnValue({}),
  attachShader: jest.fn(),
  linkProgram: jest.fn(),
  getProgramParameter: jest.fn().mockReturnValue(true),
  useProgram: jest.fn(),
  createBuffer: jest.fn().mockReturnValue({}),
  bindBuffer: jest.fn(),
  bufferData: jest.fn(),
  enableVertexAttribArray: jest.fn(),
  vertexAttribPointer: jest.fn(),
  vertexAttribDivisor: jest.fn(),
  createVertexArray: jest.fn().mockReturnValue({}),
  bindVertexArray: jest.fn(),
  deleteVertexArray: jest.fn(),
  deleteBuffer: jest.fn(),
  deleteProgram: jest.fn(),
  viewport: jest.fn(),
  clearColor: jest.fn(),
  clear: jest.fn(),
  drawArraysInstanced: jest.fn(),
  createTexture: jest.fn().mockReturnValue({}),
  bindTexture: jest.fn(),
  texImage2D: jest.fn(),
  texParameteri: jest.fn(),
  deleteTexture: jest.fn(),
  createFramebuffer: jest.fn().mockReturnValue({}),
  bindFramebuffer: jest.fn(),
  framebufferTexture2D: jest.fn(),
  deleteFramebuffer: jest.fn(),
  getUniformLocation: jest.fn().mockReturnValue({}),
  uniform1f: jest.fn(),
  uniform2f: jest.fn(),
  uniform3f: jest.fn(),
  uniform1i: jest.fn(),
  activeTexture: jest.fn(),
  drawArrays: jest.fn(),
  texSubImage2D: jest.fn(),
  isContextLost: jest.fn().mockReturnValue(false),
});

HTMLCanvasElement.prototype.getContext = mockGetContext as any;

describe('BiomeRenderer Component', () => {
  beforeEach(() => {
    mockGetContext.mockClear();
  });

  it('キャンバス要素が正しく描画され、WebGL2コンテキストが取得されること', () => {
    const { container } = render(<BiomeRenderer width={400} height={400} renderView={new Float32Array(0)} />);
    
    const canvas = container.querySelector('canvas');
    expect(canvas).toBeInTheDocument();
    
    // WebGL2 コンテキストが呼び出されたことを確認
    expect(mockGetContext).toHaveBeenCalledWith('webgl2', expect.any(Object));
  });

  it('higgsエフェクトが指定されたときに正しく動作すること', () => {
    const { container } = render(
      <BiomeRenderer 
        width={400} 
        height={400} 
        renderView={new Float32Array(0)} 
        effectType="higgs" 
        effectIntensity={0.5} 
        effectCenter={[0.5, 0.5]} 
      />
    );
    const canvas = container.querySelector('canvas');
    expect(canvas).toBeInTheDocument();
    expect(mockGetContext).toHaveBeenCalled();
  });

  it('tachyonエフェクトが指定されたときに正しく動作すること', () => {
    const { container } = render(
      <BiomeRenderer 
        width={400} 
        height={400} 
        renderView={new Float32Array(0)} 
        effectType="tachyon" 
        effectIntensity={0.8}
      />
    );
    const canvas = container.querySelector('canvas');
    expect(canvas).toBeInTheDocument();
    expect(mockGetContext).toHaveBeenCalled();
  });

  it('グリッド用の3つのデータテクスチャが作成され、正しくテクスチャユニットにバインドされること', async () => {
    const renderView = new Float32Array(128 * 128 * 12);
    renderView[0] = 0.5; // x
    renderView[1] = 0.5; // y
    renderView[2] = 1.0; // active
    renderView[3] = 0.0; // morphology
    
    render(<BiomeRenderer width={400} height={400} renderView={renderView} />);
    
    // 描画ループが非同期で実行されるのを待つ
    await new Promise((resolve) => setTimeout(resolve, 50));
    
    const context = mockGetContext.mock.results[0].value;
    
    // u_gridTex2 (TEXTURE4) の設定を検証
    expect(context.uniform1i).toHaveBeenCalledWith(expect.any(Object), 4);
  });

  it('正常系: 元素データ(C, N, P, HおよびO, S, Fe, Si)がパッキングされずに生値f32のままデータテクスチャに書き込まれること', async () => {
    const renderView = new Float32Array(128 * 128 * 12);
    renderView[0] = 1.2;  // x
    renderView[1] = 3.4;  // y
    renderView[2] = 1.0;  // active
    renderView[3] = 2.0;  // morphology
    renderView[4] = 10.0; // C
    renderView[5] = 20.0; // N
    renderView[6] = 30.0; // P
    renderView[7] = 40.0; // H
    renderView[8] = 50.0; // O
    renderView[9] = 60.0; // S
    renderView[10] = 70.0; // Fe
    renderView[11] = 80.0; // Si

    render(<BiomeRenderer width={400} height={400} renderView={renderView} />);
    
    await new Promise((resolve) => setTimeout(resolve, 50));
    
    const context = mockGetContext.mock.results[0].value;
    const texSubCalls = context.texSubImage2D.mock.calls;
    expect(texSubCalls.length).toBeGreaterThanOrEqual(3);
    
    const lastCalls = texSubCalls.slice(-3);
    const tex0Data = lastCalls[0][8];
    const tex1Data = lastCalls[1][8];
    const tex2Data = lastCalls[2][8];
    
    expect(tex0Data[0]).toBeCloseTo(1.2, 5);
    expect(tex0Data[1]).toBeCloseTo(3.4, 5);
    expect(tex0Data[2]).toBeCloseTo(1.0, 5);
    expect(tex0Data[3]).toBeCloseTo(2.0, 5);
    
    expect(tex1Data[0]).toBeCloseTo(10.0, 5);
    expect(tex1Data[1]).toBeCloseTo(20.0, 5);
    expect(tex1Data[2]).toBeCloseTo(30.0, 5);
    expect(tex1Data[3]).toBeCloseTo(40.0, 5);
    
    expect(tex2Data[0]).toBeCloseTo(50.0, 5);
    expect(tex2Data[1]).toBeCloseTo(60.0, 5);
    expect(tex2Data[2]).toBeCloseTo(70.0, 5);
    expect(tex2Data[3]).toBeCloseTo(80.0, 5);
  });
});



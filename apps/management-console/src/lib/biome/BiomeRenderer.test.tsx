import React from 'react';
import { render } from '@testing-library/react';

// Vite の ?raw クエリを解決するための Jest バーチャルモック
jest.mock('./shaders/grid.vert?raw', () => 'void main() {}', { virtual: true });
jest.mock('./shaders/grid.frag?raw', () => 'void main() {}', { virtual: true });

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
});

HTMLCanvasElement.prototype.getContext = mockGetContext as any;

describe('BiomeRenderer Component', () => {
  beforeEach(() => {
    mockGetContext.mockClear();
  });

  it('キャンバス要素が正しく描画され、WebGL2コンテキストが取得されること', () => {
    const { container } = render(<BiomeRenderer width={400} height={400} cells={[]} />);
    
    const canvas = container.querySelector('canvas');
    expect(canvas).toBeInTheDocument();
    
    // WebGL2 コンテキストが呼び出されたことを確認
    // 現在は BiomeRenderer 自体が未実装のため、テストが失敗する (RED)
    expect(mockGetContext).toHaveBeenCalledWith('webgl2', expect.any(Object));
  });
});

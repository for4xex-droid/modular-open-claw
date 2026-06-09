import React from 'react';
import { render, screen, waitFor } from '@testing-library/react';

// Vite ?raw バーチャルモック
jest.mock('./shaders/grid.vert?raw', () => 'void main() {}', { virtual: true });
jest.mock('./shaders/grid.frag?raw', () => 'void main() {}', { virtual: true });

// BiomeEngine WASM のモック
jest.mock('biome-engine', () => {
  return {
    BiomeEngine: jest.fn().mockImplementation(() => {
      return {
        generation: () => 0,
        tick: jest.fn(),
        apply_tachyon_rewind: jest.fn(),
      };
    }),
  };
});

// HTMLCanvasElement WebGL2 のモック
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
});
HTMLCanvasElement.prototype.getContext = mockGetContext as any;

import { BiomeGame } from './BiomeGame';

describe('BiomeGame Component', () => {
  it('ロード完了後に HUD やコントローラー、キャンバスが描画されること', async () => {
    render(<BiomeGame seed={42} />);

    // 初期状態は loading
    expect(screen.getByText(/Loading/i)).toBeInTheDocument();

    // ロード完了を待つ
    await waitFor(() => {
      expect(screen.queryByText(/Loading/i)).not.toBeInTheDocument();
    });

    // 各統合パーツが描画されていることを確認
    expect(screen.getByText(/GENERATION/i)).toBeInTheDocument();
    expect(screen.getByText(/INJECT ELEMENTS/i)).toBeInTheDocument();
    
    const canvas = document.querySelector('canvas');
    expect(canvas).toBeInTheDocument();
  });
});

import React from 'react';
import { render, screen, waitFor } from '@testing-library/react';

// Vite ?raw バーチャルモック
jest.mock('./shaders/grid.vert?raw', () => 'void main() {}', { virtual: true });
jest.mock('./shaders/grid.frag?raw', () => 'void main() {}', { virtual: true });
jest.mock('./shaders/higgs.frag?raw', () => 'void main() {}', { virtual: true });
jest.mock('./shaders/tachyon.frag?raw', () => 'void main() {}', { virtual: true });
jest.mock('./shaders/bloom.frag?raw', () => 'void main() {}', { virtual: true });

// BiomeEngine WASM のモック
jest.mock('biome-engine', () => {
  return {
    __esModule: true,
    default: jest.fn().mockResolvedValue({
      memory: { buffer: new ArrayBuffer(1024 * 1024) },
    }),
    BiomeEngine: jest.fn().mockImplementation(() => {
      return {
        generation: () => 0,
        tick: jest.fn(),
        apply_tachyon_rewind: jest.fn(),
        render_data_ptr: () => 0,
        render_data_len: () => 16384 * 12,
        get_cell_detail: jest.fn(),
        inject_element: jest.fn(),
        apply_crisis: jest.fn(),
        get_rarity: () => 0,
        get_active_cell_count: () => 100,
        get_element_balance: () => new Uint16Array([40, 30, 10, 20, 0, 0, 0, 0]),
        get_mutation_boost: () => 1.0,
        ticks_since_mutation: () => 0,
        roll_substance: () => 0,
        serialize_genome: () => '{}',
        set_mutation_boost: jest.fn(),
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
});
// config のモックを追加して import.meta.env エラーを防ぐ
jest.mock('../../config', () => ({
  API_BASE: 'http://localhost:3015',
  APP_VERSION: 'v1.0.2',
  STRIPE_PRICE_ID: 'price_gold_monthly',
}));

// Mock canvas elements
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
    expect(screen.getByTestId('biome-generation')).toBeInTheDocument();
    expect(screen.getByText(/元素注入/i)).toBeInTheDocument();
    
    const canvas = document.querySelector('canvas');
    expect(canvas).toBeInTheDocument();
  });

  it('seedプロパティが省略された場合でも正常にレンダリングされること', async () => {
    render(<BiomeGame />);

    await waitFor(() => {
      expect(screen.queryByText(/Loading/i)).not.toBeInTheDocument();
    });

    expect(screen.getByTestId('biome-generation')).toBeInTheDocument();
  });
});


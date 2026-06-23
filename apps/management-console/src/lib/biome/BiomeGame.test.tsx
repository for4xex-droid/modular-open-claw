import React from 'react';
import { render, screen, waitFor } from '@testing-library/react';

// R3F + postprocessing モック
jest.mock('@react-three/fiber', () => ({
  Canvas: ({ children, ...props }: any) => <div data-testid="r3f-canvas" {...props}>{children}</div>,
  useFrame: jest.fn(),
  extend: jest.fn(),
}));
jest.mock('@react-three/drei', () => ({
  Sparkles: (props: any) => <div data-testid="sparkles" {...props} />,
  shaderMaterial: jest.fn(() => function MockMaterial() {}),
  OrthographicCamera: (props: any) => <div data-testid="orthographic-camera" {...props} />,
}));
jest.mock('@react-three/postprocessing', () => ({
  EffectComposer: ({ children }: any) => <div>{children}</div>,
  Bloom: (props: any) => <div data-testid="bloom" {...props} />,
}));
jest.mock('postprocessing', () => ({
  Effect: class MockEffect {
    uniforms: Map<string, { value: any }>;
    constructor(name: string, shader: string, options: any = {}) {
      this.uniforms = options.uniforms || new Map();
    }
    dispose() {}
  },
  SavePass: jest.fn().mockImplementation(() => ({
    render: jest.fn(),
    dispose: jest.fn(),
  })),
  CopyPass: jest.fn().mockImplementation(() => ({
    render: jest.fn(),
    dispose: jest.fn(),
  })),
}));

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
        get_rarity_progress: () => ({
          rarity: 0,
          active_cells: 0,
          morphology_count: 0,
          has_homeostasis: false,
          diversity_index: 0.0,
          condition_active_500: false,
          condition_morph_3: false,
          condition_morph_4: false,
          condition_active_1000: false,
        }),
        get_last_tick_events: () => [],
      };
    }),
  };
});

// config のモックを追加して import.meta.env エラーを防ぐ
jest.mock('../../config', () => ({
  API_BASE: 'http://localhost:3015',
  APP_VERSION: 'v1.0.2',
  STRIPE_PRICE_ID: 'price_gold_monthly',
}));

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
    
    expect(screen.getByTestId('r3f-canvas')).toBeInTheDocument();
  });

  it('seedプロパティが省略された場合でも正常にレンダリングされること', async () => {
    render(<BiomeGame />);

    await waitFor(() => {
      expect(screen.queryByText(/Loading/i)).not.toBeInTheDocument();
    });

    expect(screen.getByTestId('biome-generation')).toBeInTheDocument();
  });
});


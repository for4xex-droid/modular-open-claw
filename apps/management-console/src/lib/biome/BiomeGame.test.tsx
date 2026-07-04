/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import React from 'react';
import { render, screen, waitFor, fireEvent } from '@testing-library/react';

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

// モック値を動的に制御するための mock プレフィックス付き変数
let mockGenerationVal = 0;
let mockActiveCellCountVal = 100;
let mockElementBalanceVal = new Uint16Array([40, 30, 10, 20, 0, 0, 0, 0]);

// useBiomeEngine フックのモック化（Worker や WASM の複雑な非同期処理をバイパス）
jest.mock('../../hooks/useBiomeEngine', () => {
  const stableRenderView = new Float32Array(16384 * 13);
  const stableRarityProgress = {
    rarity: 0,
    active_cells: 100,
    morphology_count: 0,
    has_homeostasis: false,
    diversity_index: 0.0,
    condition_active_500: false,
    condition_morph_3: false,
    condition_morph_4: false,
    condition_active_1000: false,
    symmetry_score: 0.0,
    complexity_score: 0.0,
    cluster_count: 0,
    prismatic_cells: 0,
    condition_structure: false,
    condition_prismatic: false,
    mass: 0,
    locomotion: 0,
    longevity: 0,
    species_hash: 0,
  };
  const stableFns = {
    tick: jest.fn(),
    rewind: jest.fn(),
    getRenderView: jest.fn(() => stableRenderView),
    getCellDetail: jest.fn(),
    injectElement: jest.fn(),
    injectBrush: jest.fn(),
    applyCrisis: jest.fn(),
    serializeGenome: jest.fn(() => '{}'),
    getRarity: jest.fn(() => 0),
    getActiveCellCount: jest.fn(() => mockActiveCellCountVal),
    getElementBalance: jest.fn(() => mockElementBalanceVal),
    rollSubstance: jest.fn(() => 0),
    getMutationBoost: jest.fn(() => 1.0),
    ticksSinceMutation: jest.fn(() => 0),
    getRarityProgress: jest.fn(() => ({
      ...stableRarityProgress,
      active_cells: mockActiveCellCountVal,
    })),
    getLastTickEvents: jest.fn(() => []),
    leniaMu: 0.15,
    leniaSigma: 0.017,
    setLeniaParams: jest.fn(),
    getLeniaMu: jest.fn(() => 0.15),
    getLeniaSigma: jest.fn(() => 0.017),
  };

  return {
    useBiomeEngine: jest.fn().mockImplementation(() => ({
      loading: false,
      error: null,
      generation: mockGenerationVal,
      ...stableFns,
    }))
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
  beforeEach(() => {
    mockGenerationVal = 0;
    mockActiveCellCountVal = 100;
    mockElementBalanceVal = new Uint16Array([40, 30, 10, 20, 0, 0, 0, 0]);
  });

  it('ロード完了後に HUD やコントローラー、キャンバスが描画されること', async () => {
    render(<BiomeGame seed={42} />);

    await waitFor(() => {
      expect(screen.getByTestId('biome-generation')).toBeInTheDocument();
    });
    expect(screen.getByTestId('control-seed-mode')).toBeInTheDocument();
    
    expect(screen.getByTestId('r3f-canvas')).toBeInTheDocument();
  });

  it('seedプロパティが省略された場合でも正常にレンダリングされること', async () => {
    render(<BiomeGame />);

    await waitFor(() => {
      expect(screen.getByTestId('biome-generation')).toBeInTheDocument();
    });
  });

  it('標本を保存した際に element_balance, active_cell_count が API 送信ペイロードに含まれ、世代が正しいこと', async () => {
    // 状態をセットアップ
    mockGenerationVal = 200; // リザルト画面をトリガーする世代
    mockActiveCellCountVal = 85;
    mockElementBalanceVal = new Uint16Array([10, 20, 30, 40, 0, 0, 0, 0]); // C=10, N=20, P=30, H=40

    // Fetch API モック
    const mockFetch = jest.fn().mockResolvedValue({
      ok: true,
      json: async () => ([])
    });
    global.fetch = mockFetch;

    // JWT 認証状態のモック
    sessionStorage.setItem('aiome_secret', 'mock-jwt-token');

    render(<BiomeGame seed={42} />);

    // ロード完了と結果ダイアログの表示を待つ
    await waitFor(() => {
      expect(screen.queryByText(/Loading/i)).not.toBeInTheDocument();
    });

    // 評価ランク画面が表示されていることを確認
    expect(screen.getByTestId('result-rarity')).toBeInTheDocument();

    // 💾 保存ボタンをクリック
    const saveBtn = screen.getByTestId('result-save');
    fireEvent.click(saveBtn);

    // Fetch API が specimens 取得 + Run/Specimen 保存で呼ばれることを検証
    await waitFor(() => {
      const postCalls = mockFetch.mock.calls.filter((call) => call[1]?.method === 'POST');
      expect(postCalls.length).toBeGreaterThanOrEqual(2);
    });

    // 1 回目のリクエスト (specimens 一覧取得) をスキップし、runs 保存を検証
    const runCall = mockFetch.mock.calls.find((call) => String(call[0]).includes('/runs'));
    expect(runCall).toBeDefined();
    const runPayload = JSON.parse(runCall![1]!.body as string);
    expect(runPayload.generation).toBe(200);

    // specimens 保存の検証
    const specCall = mockFetch.mock.calls.find(
      (call) => String(call[0]).includes('/specimens') && call[1]?.method === 'POST'
    );
    expect(specCall).toBeDefined();
    const specPayload = JSON.parse(specCall![1]!.body as string);
    
    // ペイロード検証
    expect(specPayload.active_cell_count).toBe(85);
    const genome = JSON.parse(specPayload.genome_data);
    expect(genome.mu).toBe(0.15);
    expect(genome.sigma).toBe(0.017);
    expect(specPayload.element_balance).toBe(JSON.stringify({
      C: 10, N: 20, P: 30, H: 40,
      O: 0, S: 0, Fe: 0, Si: 0
    }));
    
    // クリーンアップ
    sessionStorage.removeItem('aiome_secret');
  });
});


/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
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

// モック値を動的に制御するための mock プレフィックス付き変数
let mockGenerationVal = 0;
let mockActiveCellCountVal = 100;
let mockElementBalanceVal = new Uint16Array([40, 30, 10, 20, 0, 0, 0, 0]);

// useBiomeEngine フックのモック化（Worker や WASM の複雑な非同期処理をバイパス）
jest.mock('../../hooks/useBiomeEngine', () => {
  return {
    useBiomeEngine: jest.fn().mockImplementation(() => {
      return {
        loading: false,
        error: null,
        generation: mockGenerationVal,
        tick: jest.fn(),
        rewind: jest.fn(),
        getRenderView: jest.fn(() => new Float32Array(16384 * 12)),
        getCellDetail: jest.fn(),
        injectElement: jest.fn(),
        applyCrisis: jest.fn(),
        getRarity: () => 0,
        getActiveCellCount: () => mockActiveCellCountVal,
        getElementBalance: () => mockElementBalanceVal,
        rollSubstance: () => 0,
        getMutationBoost: () => 1.0,
        ticksSinceMutation: () => 0,
        serializeGenome: () => '{}',
        getRarityProgress: () => ({
          rarity: 0,
          active_cells: mockActiveCellCountVal,
          morphology_count: 0,
          has_homeostasis: false,
          diversity_index: 0.0,
          condition_active_500: false,
          condition_morph_3: false,
          condition_morph_4: false,
          condition_active_1000: false,
        }),
        getLastTickEvents: () => [],
      };
    })
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

  it('標本を保存した際に element_balance, active_cell_count が API 送信ペイロードに含まれ、世代が正しいこと', async () => {
    // 状態をセットアップ
    mockGenerationVal = 200; // リザルト画面をトリガーする世代
    mockActiveCellCountVal = 85;
    mockElementBalanceVal = new Uint16Array([10, 20, 30, 40, 0, 0, 0, 0]); // C=10, N=20, P=30, H=40

    // Fetch API モック
    const mockFetch = jest.fn().mockResolvedValue({
      ok: true,
      json: async () => ({})
    });
    global.fetch = mockFetch;

    // JWT 認証状態のモック
    localStorage.setItem('jwt_token', 'mock-jwt-token');

    render(<BiomeGame seed={42} />);

    // ロード完了と結果ダイアログの表示を待つ
    await waitFor(() => {
      expect(screen.queryByText(/Loading/i)).not.toBeInTheDocument();
    });

    // 評価ランク画面が表示されていることを確認
    expect(screen.getByTestId('result-rarity')).toBeInTheDocument();

    // 💾 保存ボタンをクリック
    const saveBtn = screen.getByRole('button', { name: /💾 標本を保存/i });
    fireEvent.click(saveBtn);

    // Fetch API が 2 回 (Run 保存と Specimen 保存) 呼ばれることを検証
    await waitFor(() => {
      expect(mockFetch).toHaveBeenCalledTimes(2);
    });

    // 1 回目のリクエスト (runs 保存) の検証
    const runCall = mockFetch.mock.calls[0];
    const runPayload = JSON.parse(runCall[1].body);
    expect(runPayload.generation).toBe(200); // ハードコード 200 ではなく実際の世代 (200)

    // 2 回目のリクエスト (specimens 保存) の検証
    const specCall = mockFetch.mock.calls[1];
    const specPayload = JSON.parse(specCall[1].body);
    
    // ペイロード検証
    expect(specPayload.active_cell_count).toBe(85);
    expect(specPayload.element_balance).toBe(JSON.stringify({
      C: 10, N: 20, P: 30, H: 40,
      O: 0, S: 0, Fe: 0, Si: 0
    }));
    
    // クリーンアップ
    localStorage.removeItem('jwt_token');
  });
});


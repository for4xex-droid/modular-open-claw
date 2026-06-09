import { renderHook, act, waitFor } from '@testing-library/react';
import { useBiomeEngine } from './useBiomeEngine';

// biome-engine WASM のモック
jest.mock('biome-engine', () => {
  return {
    BiomeEngine: jest.fn().mockImplementation(() => {
      let generation = 0;
      return {
        generation: () => generation,
        tick: jest.fn(() => {
          generation += 1;
        }),
        apply_tachyon_rewind: jest.fn((g: number) => {
          if (g <= generation) {
            generation -= g;
            return true;
          }
          return false;
        }),
      };
    }),
  };
});

describe('useBiomeEngine Hook', () => {
  beforeEach(() => {
    jest.clearAllMocks();
  });

  it('初期状態では loading が true であり、世代数が 0 であること', async () => {
    const { result } = renderHook(() => useBiomeEngine({ seed: 42 }));
    
    // 同期直後のチェック
    expect(result.current.loading).toBe(true);
    expect(result.current.generation).toBe(0);

    // 警告回避のためにロード完了を待つ
    await waitFor(() => expect(result.current.loading).toBe(false));
  });

  it('ロード完了後に loading が false になること', async () => {
    const { result } = renderHook(() => useBiomeEngine({ seed: 42 }));
    
    await waitFor(() => expect(result.current.loading).toBe(false));
  });

  it('tick を実行した際に世代数がインクリメントされること', async () => {
    const { result } = renderHook(() => useBiomeEngine({ seed: 42 }));

    await waitFor(() => expect(result.current.loading).toBe(false));

    act(() => {
      result.current.tick();
    });

    expect(result.current.generation).toBe(1);
  });

  it('因果逆行 (rewind) を実行した際に世代数が戻ること', async () => {
    const { result } = renderHook(() => useBiomeEngine({ seed: 42 }));

    await waitFor(() => expect(result.current.loading).toBe(false));

    // 5回 tick する
    act(() => {
      for (let i = 0; i < 5; i++) {
        result.current.tick();
      }
    });
    expect(result.current.generation).toBe(5);

    // 3世代戻す
    act(() => {
      const success = result.current.rewind(3);
      expect(success).toBe(true);
    });

    expect(result.current.generation).toBe(2);
  });
});

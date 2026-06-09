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

  it('ウィンドウ非表示時に 1fps tick の setInterval が開始され、表示時にクリアされること', async () => {
    jest.useFakeTimers();
    
    // visibilityState をモック
    let visibilityState = 'visible';
    Object.defineProperty(document, 'visibilityState', {
      get: () => visibilityState,
      configurable: true
    });

    const { result, unmount } = renderHook(() => useBiomeEngine({ seed: 42, paused: false }));
    await waitFor(() => expect(result.current.loading).toBe(false));

    // hidden に変更
    act(() => {
      visibilityState = 'hidden';
      document.dispatchEvent(new Event('visibilitychange'));
    });

    // 1000ms 進める
    act(() => {
      jest.advanceTimersByTime(1000);
    });

    // generation が増加していることを確認
    expect(result.current.generation).toBeGreaterThan(0);

    // visible に戻す
    act(() => {
      visibilityState = 'visible';
      document.dispatchEvent(new Event('visibilitychange'));
    });

    const currentGen = result.current.generation;
    act(() => {
      jest.advanceTimersByTime(1000);
    });
    // visible に戻った後は setInterval による tick は追加で発生しない
    expect(result.current.generation).toBe(currentGen);

    unmount();
    jest.useRealTimers();
  });
});

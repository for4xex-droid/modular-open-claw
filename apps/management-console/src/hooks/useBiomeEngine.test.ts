import { renderHook, act, waitFor } from '@testing-library/react';
import { useBiomeEngine } from './useBiomeEngine';

jest.mock('biome-engine', () => {
  return {
    __esModule: true,
    default: jest.fn().mockResolvedValue({
      memory: { buffer: new ArrayBuffer(1024 * 1024) },
    }),
    BiomeEngine: jest.fn().mockImplementation(() => {
      let generation = 0;
      const genMock = jest.fn().mockImplementation(() => generation);
      return {
        generation: genMock,
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
        free: jest.fn(),
        serialize: jest.fn().mockReturnValue('{"generation":0,"seed":42}'),
      };
    }),
  };
});

// Mock Worker is now defined globally in setupTests.ts
const getLastWorkerInstance = () => (global as any).lastWorkerInstance;




// static メソッドをモックするために追加定義
const wasm = require('biome-engine');
wasm.BiomeEngine.deserialize = jest.fn().mockImplementation((json: string) => {
  const data = JSON.parse(json);
  let generation = data.generation || 0;
  const genMock = jest.fn().mockImplementation(() => generation);
  return {
    generation: genMock,
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
    free: jest.fn(),
    serialize: jest.fn().mockReturnValue('{"generation":0,"seed":42}'),
  };
});

let mockDbStore: Record<string, any> = {};
const mockIndexedDB = {
  open: jest.fn().mockImplementation(() => {
    const request: any = {
      result: {
        objectStoreNames: {
          contains: jest.fn().mockImplementation((name) => name === 'engine_states'),
        },
        transaction: jest.fn().mockImplementation(() => ({
          objectStore: jest.fn().mockImplementation(() => ({
            put: jest.fn().mockImplementation((value, key) => {
              mockDbStore[key] = value;
              const req: any = {};
              setTimeout(() => {
                if (req.onsuccess) req.onsuccess();
              }, 0);
              return req;
            }),
            get: jest.fn().mockImplementation((key) => {
              const req: any = { result: mockDbStore[key] };
              setTimeout(() => {
                if (req.onsuccess) req.onsuccess();
              }, 0);
              return req;
            }),
          })),
        })),
      },
    };
    setTimeout(() => {
      if (request.onsuccess) request.onsuccess();
    }, 0);
    return request;
  }),
};

describe('useBiomeEngine Hook', () => {
  beforeEach(() => {
    jest.clearAllMocks();
    mockDbStore = {};
    Object.defineProperty(window, 'indexedDB', {
      value: mockIndexedDB,
      configurable: true,
      writable: true,
    });
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

    await waitFor(() => expect(result.current.generation).toBe(1));
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
    await waitFor(() => expect(result.current.generation).toBe(5));

    // 3世代戻す
    act(() => {
      const success = result.current.rewind(3);
      expect(success).toBe(true);
    });

    await waitFor(() => expect(result.current.generation).toBe(2));
  });

  it('ウィンドウ非表示時に 1fps tick の setInterval が開始され、表示時にクリアされること', async () => {
    // まず real timers でロード完了を待つ
    const { result, unmount } = renderHook(() => useBiomeEngine({ seed: 42, paused: false }));
    await waitFor(() => expect(result.current.loading).toBe(false));

    // ロード完了後に fake timers に切り替える
    jest.useFakeTimers();
    
    // visibilityState をモック
    let visibilityState = 'visible';
    Object.defineProperty(document, 'visibilityState', {
      get: () => visibilityState,
      configurable: true
    });

    // hidden に変更してイベント発火
    act(() => {
      visibilityState = 'hidden';
      document.dispatchEvent(new Event('visibilitychange'));
    });

    // 1000ms 進める
    act(() => {
      jest.advanceTimersByTime(1000);
    });

    // generation が増加していることを確認
    await waitFor(() => expect(result.current.generation).toBeGreaterThan(0));

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

  it('無効な seed 値 (NaN, Infinity など) が渡された場合に 0 にフォールバックすること', async () => {
    // NaN の場合
    const { result: resultNaN } = renderHook(() => useBiomeEngine({ seed: NaN }));
    await waitFor(() => expect(resultNaN.current.loading).toBe(false));
    expect(getLastWorkerInstance()?.postMessage).toHaveBeenCalledWith(
      expect.objectContaining({ type: 'init', seed: 0 })
    );

    // Infinity の場合
    const { result: resultInf } = renderHook(() => useBiomeEngine({ seed: Infinity }));
    await waitFor(() => expect(resultInf.current.loading).toBe(false));
    expect(getLastWorkerInstance()?.postMessage).toHaveBeenCalledWith(
      expect.objectContaining({ type: 'init', seed: 0 })
    );
  });

  it('アンマウントまたは seed 変更時に前の Worker インスタンスが terminate されること', async () => {
    // 最初フックのロード
    const { result, rerender, unmount } = renderHook(({ seed }) => useBiomeEngine({ seed }), {
      initialProps: { seed: 42 }
    });
    
    await waitFor(() => expect(result.current.loading).toBe(false));
    const firstWorker = getLastWorkerInstance();
    
    // seed を変更する
    rerender({ seed: 100 });
    await waitFor(() => expect(result.current.loading).toBe(false));
    
    // 最初の Worker が terminate されていることを期待
    expect(firstWorker?.terminate).toHaveBeenCalled();
    const secondWorker = getLastWorkerInstance();
    
    // アンマウント
    unmount();
    expect(secondWorker?.terminate).toHaveBeenCalled();
  });

  it('seed が変化しない場合、返される API 関数の参照が安定していること', async () => {
    const { result, rerender } = renderHook(({ seed }) => useBiomeEngine({ seed }), {
      initialProps: { seed: 42 }
    });

    await waitFor(() => expect(result.current.loading).toBe(false));

    const initialTick = result.current.tick;
    const initialRewind = result.current.rewind;
    const initialInject = result.current.injectElement;
    const initialApplyCrisis = result.current.applyCrisis;

    // seed は同じままで rerender をトリガー
    rerender({ seed: 42 });

    expect(result.current.tick).toBe(initialTick);
    expect(result.current.rewind).toBe(initialRewind);
    expect(result.current.injectElement).toBe(initialInject);
    expect(result.current.applyCrisis).toBe(initialApplyCrisis);
  });

  it('IndexedDB に保存された状態がある場合、初期化時に savedState が Worker に渡され復元されること', async () => {
    mockDbStore['seed_42'] = '{"generation":150,"seed":42}';
    
    const { result } = renderHook(() => useBiomeEngine({ seed: 42 }));
    await waitFor(() => expect(result.current.loading).toBe(false));
    
    expect(getLastWorkerInstance()?.postMessage).toHaveBeenCalledWith(
      expect.objectContaining({ type: 'init', seed: 42, savedState: '{"generation":150,"seed":42}' })
    );
    expect(result.current.generation).toBe(150);
  });

  it('tick や rewind の実行時に状態が IndexedDB に自動保存されること', async () => {
    const { result } = renderHook(() => useBiomeEngine({ seed: 42 }));
    await waitFor(() => expect(result.current.loading).toBe(false));
    
    // tick 実行
    act(() => {
      result.current.tick();
    });
    
    // 非同期保存を待つ
    await waitFor(() => {
      expect(mockDbStore['seed_42']).toBe('{"generation":1,"seed":42}');
    });
  });

  it('初期化時に Web Worker が作成され、init メッセージが送信されること', async () => {
    const { result } = renderHook(() => useBiomeEngine({ seed: 42 }));
    await waitFor(() => expect(result.current.loading).toBe(false));
    expect(getLastWorkerInstance()).not.toBeNull();
    expect(getLastWorkerInstance()?.postMessage).toHaveBeenCalledWith(
      expect.objectContaining({ type: 'init', seed: 42 })
    );
  });

  it('WASMの初期化エラー（異常系）が発生した場合に、error 状態がセットされ loading が false になること', async () => {
    const { result } = renderHook(() => useBiomeEngine({ seed: -999 }));
    await waitFor(() => expect(result.current.loading).toBe(false));
    expect(result.current.error).toBe('Simulated WASM initialization failure');
  });

  it('エラー発生後に seed が変更された場合、error 状態がクリアされて loading が true になること', async () => {
    const { result, rerender } = renderHook(({ seed }) => useBiomeEngine({ seed }), {
      initialProps: { seed: -999 }
    });
    await waitFor(() => expect(result.current.loading).toBe(false));
    expect(result.current.error).toBe('Simulated WASM initialization failure');

    act(() => {
      rerender({ seed: 42 });
    });
    expect(result.current.loading).toBe(true);
    expect(result.current.error).toBeNull();

    await waitFor(() => expect(result.current.loading).toBe(false));
  });
});




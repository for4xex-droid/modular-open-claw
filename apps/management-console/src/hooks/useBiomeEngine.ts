import { useState, useEffect, useRef, useCallback } from 'react';
import type { BiomeEngine } from 'biome-engine';

export interface UseBiomeEngineOptions {
  seed: number;
  paused?: boolean;
}

export function useBiomeEngine({ seed, paused = false }: UseBiomeEngineOptions) {
  const [loading, setLoading] = useState(true);
  const [generation, setGeneration] = useState(0);
  const engineRef = useRef<BiomeEngine | null>(null);
  const wasmMemoryRef = useRef<WebAssembly.Memory | null>(null);
  const pausedRef = useRef<boolean>(paused);

  // paused の最新状態を保持
  useEffect(() => {
    pausedRef.current = paused;
  }, [paused]);

  useEffect(() => {
    let active = true;
    const validatedSeed = Number.isFinite(seed) ? seed : 0;
    
    // WASMモジュールの動的インポート
    import('biome-engine').then(async (wasm) => {
      if (!active) return;
      
      // デフォルトの初期化関数を実行して InitOutput (memoryを含む) を取得
      const initOutput = await wasm.default();
      wasmMemoryRef.current = initOutput.memory;

      // 古いインスタンスがあれば解放する
      if (engineRef.current) {
        try {
          engineRef.current.free();
        } catch (e) {
          console.warn('Failed to free previous biome-engine instance', e);
        }
      }

      // WASM側の constructor は u64 (BigInt) を期待する
      const engine = new wasm.BiomeEngine(BigInt(validatedSeed));
      engineRef.current = engine;
      setGeneration(Number(engine.generation()));
      setLoading(false);
    }).catch((err) => {
      console.error('Failed to load biome-engine WASM', err);
    });

    return () => {
      active = false;
      // アンマウント時にも解放を試みる。ただし、非同期読み込み中の場合は
      // active = false によって engineRef に代入されないため、
      // ここで解放されるか、あるいは生成されない。
      if (engineRef.current) {
        try {
          engineRef.current.free();
          engineRef.current = null;
        } catch (e) {
          console.warn('Failed to free biome-engine instance on unmount', e);
        }
      }
    };
  }, [seed]);


  const tick = useCallback(() => {
    if (!engineRef.current) return;
    engineRef.current.tick();
    setGeneration(Number(engineRef.current.generation()));
  }, []);

  // バックグラウンド進化の制御
  useEffect(() => {
    if (loading) return;

    let intervalId: ReturnType<typeof setInterval> | null = null;

    const handleVisibilityChange = () => {
      if (document.visibilityState === 'hidden') {
        if (!intervalId && !pausedRef.current) {
          intervalId = setInterval(() => {
            if (!pausedRef.current) {
              tick();
            }
          }, 1000);
        }
      } else {
        if (intervalId) {
          clearInterval(intervalId);
          intervalId = null;
        }
      }
    };

    document.addEventListener('visibilitychange', handleVisibilityChange);

    // 初期状態がすでに hidden の場合に対応
    if (document.visibilityState === 'hidden') {
      handleVisibilityChange();
    }

    return () => {
      document.removeEventListener('visibilitychange', handleVisibilityChange);
      if (intervalId) {
        clearInterval(intervalId);
      }
    };
  }, [loading, tick]);


  const rewind = useCallback((generations: number): boolean => {
    if (!engineRef.current) return false;
    const success = engineRef.current.apply_tachyon_rewind(generations);
    if (success) {
      setGeneration(Number(engineRef.current.generation()));
    }
    return success;
  }, []);

  const getRenderView = useCallback((): Float32Array => {
    if (!engineRef.current || !wasmMemoryRef.current) {
      return new Float32Array(0);
    }
    const ptr = engineRef.current.render_data_ptr();
    const len = engineRef.current.render_data_len();
    // memory.grow に対処するため、毎フレーム buffer を再取得する
    return new Float32Array(wasmMemoryRef.current.buffer, ptr, len);
  }, []);

  const getCellDetail = useCallback((x: number, y: number) => {
    if (!engineRef.current) return null;
    return engineRef.current.get_cell_detail(x, y);
  }, []);

  const injectElement = useCallback((x: number, y: number, idx: number, amount: number) => {
    if (!engineRef.current) return;
    engineRef.current.inject_element(x, y, idx, amount);
  }, []);

  const applyCrisis = useCallback((crisisType: string, x: number, y: number) => {
    if (!engineRef.current) return;
    engineRef.current.apply_crisis(crisisType, x, y);
  }, []);

  const getRarity = useCallback((): number => {
    if (!engineRef.current) return 0; // Common
    return engineRef.current.get_rarity();
  }, []);

  const getActiveCellCount = useCallback((): number => {
    if (!engineRef.current) return 0;
    return engineRef.current.get_active_cell_count();
  }, []);

  const getElementBalance = useCallback((): Uint16Array => {
    if (!engineRef.current) return new Uint16Array(8);
    return engineRef.current.get_element_balance();
  }, []);

  const rollSubstance = useCallback(() => {
    if (!engineRef.current) return 0; // None
    return engineRef.current.roll_substance();
  }, []);

  const serializeGenome = useCallback((x: number, y: number): string => {
    if (!engineRef.current) return '';
    return engineRef.current.serialize_genome(x, y);
  }, []);

  const setMutationBoost = useCallback((val: number) => {
    if (!engineRef.current) return;
    engineRef.current.set_mutation_boost(val);
  }, []);

  const getMutationBoost = useCallback((): number => {
    if (!engineRef.current) return 1.0;
    return engineRef.current.get_mutation_boost();
  }, []);

  const ticksSinceMutation = useCallback((): number => {
    if (!engineRef.current) return 0;
    return engineRef.current.ticks_since_mutation();
  }, []);

  return {
    loading,
    generation,
    tick,
    rewind,
    getRenderView,
    getCellDetail,
    injectElement,
    applyCrisis,
    getRarity,
    getActiveCellCount,
    getElementBalance,
    rollSubstance,
    serializeGenome,
    setMutationBoost,
    getMutationBoost,
    ticksSinceMutation,
  };
}


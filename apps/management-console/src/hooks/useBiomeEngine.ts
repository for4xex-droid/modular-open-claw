import { useState, useEffect, useRef } from 'react';

export interface UseBiomeEngineOptions {
  seed: number;
  paused?: boolean;
}

export function useBiomeEngine({ seed, paused = false }: UseBiomeEngineOptions) {
  const [loading, setLoading] = useState(true);
  const [generation, setGeneration] = useState(0);
  const engineRef = useRef<any>(null);
  const pausedRef = useRef<boolean>(paused);

  // paused の最新状態を保持
  useEffect(() => {
    pausedRef.current = paused;
  }, [paused]);

  useEffect(() => {
    let active = true;
    
    // WASMモジュールの動的インポート
    import('biome-engine').then((wasm) => {
      if (!active) return;
      
      // WASM側の constructor は u64 (BigInt) を期待する
      const engine = new wasm.BiomeEngine(BigInt(seed));
      engineRef.current = engine;
      setGeneration(Number(engine.generation()));
      setLoading(false);
    }).catch((err) => {
      console.error('Failed to load biome-engine WASM', err);
    });

    return () => {
      active = false;
    };
  }, [seed]);

  const tick = () => {
    if (!engineRef.current) return;
    engineRef.current.tick();
    setGeneration(Number(engineRef.current.generation()));
  };

  // バックグラウンド進化の制御
  useEffect(() => {
    if (loading) return;

    let intervalId: NodeJS.Timeout | null = null;

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
  }, [loading]);

  const rewind = (generations: number): boolean => {
    if (!engineRef.current) return false;
    const success = engineRef.current.apply_tachyon_rewind(generations);
    if (success) {
      setGeneration(Number(engineRef.current.generation()));
    }
    return success;
  };

  return {
    loading,
    generation,
    tick,
    rewind,
  };
}

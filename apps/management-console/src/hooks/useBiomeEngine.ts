import { useState, useEffect, useRef } from 'react';

export interface UseBiomeEngineOptions {
  seed: number;
}

export function useBiomeEngine({ seed }: UseBiomeEngineOptions) {
  const [loading, setLoading] = useState(true);
  const [generation, setGeneration] = useState(0);
  const engineRef = useRef<any>(null);

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

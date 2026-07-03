/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import { useState, useEffect, useRef, useCallback } from 'react';
import BiomeWorker from './biome.worker?worker';

/** biome-engine `RarityProgress` (libs/biome-engine/src/rarity.rs) の TS 表現 */
export interface RarityProgress {
  rarity: number;
  active_cells: number;
  morphology_count: number;
  has_homeostasis: boolean;
  diversity_index: number;
  condition_active_500: boolean;
  condition_morph_3: boolean;
  condition_morph_4: boolean;
  condition_active_1000: boolean;
}

/** biome-engine `BiomeEvent` (libs/biome-engine/src/lib.rs) の TS 表現 */
export type BiomeEvent =
  | { type: 'MorphologyChanged'; from: number; to: number }
  | { type: 'MassExtinction'; lost_ratio: number }
  | { type: 'NewReactionDiscovered'; reaction_id: number };

// IndexedDB Helper Functions
let dbInstance: IDBDatabase | null = null;

function openDatabase(): Promise<IDBDatabase> {
  if (dbInstance) {
    return Promise.resolve(dbInstance);
  }
  return new Promise((resolve, reject) => {
    if (typeof indexedDB === 'undefined') {
      return reject(new Error('IndexedDB is not supported'));
    }
    const request = indexedDB.open('biome_db', 1);
    request.onupgradeneeded = () => {
      const db = request.result;
      if (!db.objectStoreNames.contains('engine_states')) {
        db.createObjectStore('engine_states');
      }
    };
    request.onsuccess = () => {
      dbInstance = request.result;
      dbInstance.onclose = () => {
        dbInstance = null;
      };
      dbInstance.onerror = () => {
        dbInstance = null;
      };
      resolve(dbInstance);
    };
    request.onerror = () => reject(request.error);
  });
}

function loadState(key: string): Promise<string | null> {
  return openDatabase().then((db) => {
    return new Promise<string | null>((resolve, reject) => {
      const transaction = db.transaction('engine_states', 'readonly');
      const store = transaction.objectStore('engine_states');
      const request = store.get(key);
      request.onsuccess = () => resolve(request.result || null);
      request.onerror = () => reject(request.error);
    });
  });
}

let saveQueue: Promise<void> = Promise.resolve();

function saveState(key: string, data: string): Promise<void> {
  const task = () => openDatabase().then((db) => {
    return new Promise<void>((resolve, reject) => {
      const transaction = db.transaction('engine_states', 'readwrite');
      const store = transaction.objectStore('engine_states');
      const request = store.put(data, key);
      request.onsuccess = () => resolve();
      request.onerror = () => reject(request.error);
    });
  });

  const nextPromise = saveQueue.then(task, task);
  saveQueue = nextPromise;
  return nextPromise;
}


export interface UseBiomeEngineOptions {
  seed: number;
  paused?: boolean;
}

export function useBiomeEngine({ seed, paused = false }: UseBiomeEngineOptions) {
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [generation, setGeneration] = useState(0);

  const workerRef = useRef<Worker | null>(null);
  const pausedRef = useRef<boolean>(paused);

  // 同期的なゲッターや参照をサポートするための ref 保持
  const renderViewRef = useRef<Float32Array>(new Float32Array(0));
  const frozenCellsRef = useRef<Uint8Array>(new Uint8Array(0));
  const serializedRef = useRef<string>('');

  const rarityRef = useRef<number>(0);
  const activeCellCountRef = useRef<number>(0);
  const elementBalanceRef = useRef<Uint16Array>(new Uint16Array(8));
  const mutationBoostRef = useRef<number>(1.0);
  const ticksSinceMutationRef = useRef<number>(0);
  const rarityProgressRef = useRef<RarityProgress | null>(null);
  const lastTickEventsRef = useRef<BiomeEvent[]>([]);

  // paused の最新状態を保持
  useEffect(() => {
    pausedRef.current = paused;
  }, [paused]);

  useEffect(() => {
    let active = true;
    const validatedSeed = Number.isFinite(seed) ? seed : 0;
    setLoading(true);
    setError(null);

    let worker: Worker;
    try {
      worker = new BiomeWorker();
    } catch (e) {
      // Jest フォールバック用
      worker = new Worker('./biome.worker.ts');
    }
    workerRef.current = worker;

    worker.onmessage = (e: MessageEvent) => {
      if (!active) return;

      const msg = e.data;
      if (msg.type === 'initialized') {
        setGeneration(msg.generation);
        rarityRef.current = msg.rarity;
        activeCellCountRef.current = msg.activeCells;
        elementBalanceRef.current = msg.elementBalance;
        mutationBoostRef.current = msg.mutationBoost;
        ticksSinceMutationRef.current = msg.ticksSinceMutation;
        rarityProgressRef.current = msg.rarityProgress;
        setLoading(false);
      } else if (msg.type === 'updated') {
        setGeneration(msg.generation);
        rarityRef.current = msg.rarity;
        activeCellCountRef.current = msg.activeCells;
        elementBalanceRef.current = msg.elementBalance;
        mutationBoostRef.current = msg.mutationBoost;
        ticksSinceMutationRef.current = msg.ticksSinceMutation;
        rarityProgressRef.current = msg.rarityProgress;
        lastTickEventsRef.current = msg.lastEvents || [];
        serializedRef.current = msg.serialized;

        if (msg.renderView) {
          renderViewRef.current = msg.renderView;
        }
        if (msg.frozenCells) {
          frozenCellsRef.current = msg.frozenCells;
        }

        const key = `seed_${validatedSeed}`;
        saveState(key, msg.serialized).catch((err) => {
          console.warn('Failed to auto-save state to IndexedDB', err);
        });
      } else if (msg.type === 'rewound') {
        if (msg.success) {
          setGeneration(msg.generation);
          serializedRef.current = msg.serialized;
          
          const key = `seed_${validatedSeed}`;
          saveState(key, msg.serialized).catch((err) => {
            console.warn('Failed to auto-save state after rewind', err);
          });
        }
      } else if (msg.type === 'error') {
        setError(msg.message);
        setLoading(false);
      }
    };

    // IndexedDB から過去の状態を取得して Worker に送信
    const key = `seed_${validatedSeed}`;
    loadState(key)
      .then((savedState) => {
        if (!active) return;
        worker.postMessage({
          type: 'init',
          seed: validatedSeed,
          savedState,
        });
      })
      .catch((err) => {
        console.warn('Failed to load state from IndexedDB, fallback to fresh start', err);
        if (!active) return;
        worker.postMessage({
          type: 'init',
          seed: validatedSeed,
          savedState: null,
        });
      });

    return () => {
      active = false;
      worker.terminate();
      workerRef.current = null;
    };
  }, [seed]);

  const tick = useCallback(() => {
    if (!workerRef.current) return;
    workerRef.current.postMessage({ type: 'tick' });
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
    if (!workerRef.current) return false;
    workerRef.current.postMessage({ type: 'rewind', generations });
    return true; // 非同期通信開始に成功したため true
  }, []);

  const getRenderView = useCallback((): Float32Array => {
    return renderViewRef.current;
  }, []);

  const getCellDetail = useCallback((x: number, y: number) => {
    if (!renderViewRef.current.length) return null;
    const idx = y * 128 + x;
    const offset = idx * 12;

    if (offset + 12 > renderViewRef.current.length) return null;

    const activeVal = renderViewRef.current[offset + 2] !== 0;
    const morphologyVal = renderViewRef.current[offset + 3];
    const isFrozenVal = frozenCellsRef.current ? frozenCellsRef.current[idx] !== 0 : false;

    const elements = new Uint16Array(8);
    for (let i = 0; i < 8; i++) {
      elements[i] = renderViewRef.current[offset + 4 + i];
    }

    return {
      active: activeVal,
      morphology: morphologyVal,
      elements,
      is_frozen: isFrozenVal,
      energy: elements[0], // energy として C の値をマッピング
    };
  }, []);

  const injectElement = useCallback((x: number, y: number, idx: number, amount: number) => {
    if (!workerRef.current) return;
    workerRef.current.postMessage({ type: 'inject', x, y, idx, amount });
  }, []);

  const applyCrisis = useCallback((crisisType: string, x: number, y: number) => {
    if (!workerRef.current) return;
    workerRef.current.postMessage({ type: 'crisis', crisisType, x, y });
  }, []);

  const getRarity = useCallback((): number => {
    return rarityRef.current;
  }, []);

  const getActiveCellCount = useCallback((): number => {
    return activeCellCountRef.current;
  }, []);

  const getElementBalance = useCallback((): Uint16Array => {
    return elementBalanceRef.current;
  }, []);

  const rollSubstance = useCallback(() => {
    // rollSubstance も非同期要求が必要になるが、UI 側での使われ方は？
    // スタブまたは適当な値を返すか、Worker に問い合わせが必要なら同期解決する
    return 0; // デフォルトスタブ
  }, []);

  const serializeGenome = useCallback((x: number, y: number): string => {
    if (!serializedRef.current) return '{}';
    try {
      const data = JSON.parse(serializedRef.current);
      const cell = data.cells?.[y * 128 + x];
      if (cell && cell.genome) {
        return JSON.stringify(cell.genome);
      }
    } catch (e) {
      console.warn('Failed to deserialize genome on demand', e);
    }
    return '{}';
  }, []);

  const setMutationBoost = useCallback((val: number) => {
    if (!workerRef.current) return;
    workerRef.current.postMessage({ type: 'setMutationBoost', val });
  }, []);

  const getMutationBoost = useCallback((): number => {
    return mutationBoostRef.current;
  }, []);

  const ticksSinceMutation = useCallback((): number => {
    return ticksSinceMutationRef.current;
  }, []);

  const getRarityProgress = useCallback((): RarityProgress | null => {
    return rarityProgressRef.current;
  }, []);

  const getLastTickEvents = useCallback((): BiomeEvent[] => {
    return lastTickEventsRef.current;
  }, []);

  return {
    loading,
    error,
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
    getRarityProgress,
    getLastTickEvents,
  };
}

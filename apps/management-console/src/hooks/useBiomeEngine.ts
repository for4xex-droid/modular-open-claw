/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import { useState, useEffect, useRef, useCallback } from 'react';
import BiomeWorker from './biome.worker?worker';
import { GRID_WIDTH, RENDER_STRIDE } from '../lib/biome/biomeTypes';

/** IndexedDB 自動保存のデバウンス間隔（ms） */
const SAVE_DEBOUNCE_MS = 2000;

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
  symmetry_score: number;
  complexity_score: number;
  cluster_count: number;
  prismatic_cells: number;
  condition_structure: boolean;
  condition_prismatic: boolean;
  mass: number;
  locomotion: number;
  longevity: number;
  species_hash: number;
}

/** biome-engine `BiomeEvent` (libs/biome-engine/src/lib.rs) の TS 表現 */
export type BiomeEvent =
  | { type: 'MorphologyChanged'; from: number; to: number }
  | { type: 'MassExtinction'; lost_ratio: number }
  | { type: 'NewReactionDiscovered'; reaction_id: number }
  | { type: 'PrismaticBorn'; x: number; y: number };

/** IndexedDB スキーマバージョン（v1=元素モデル, v2=Lenia 場） */
const BIOME_DB_VERSION = 2;
const BIOME_DB_NAME = 'biome_db';

// IndexedDB Helper Functions
let dbInstance: IDBDatabase | null = null;
let dbVersion: number | null = null;

function openDatabase(): Promise<IDBDatabase> {
  if (dbInstance && dbVersion === BIOME_DB_VERSION) {
    return Promise.resolve(dbInstance);
  }
  return new Promise((resolve, reject) => {
    if (typeof indexedDB === 'undefined') {
      return reject(new Error('IndexedDB is not supported'));
    }
    const request = indexedDB.open(BIOME_DB_NAME, BIOME_DB_VERSION);
    request.onupgradeneeded = () => {
      const db = request.result;
      // v1→v2: Lenia 非互換のため旧セーブを破棄
      if (db.objectStoreNames.contains('engine_states')) {
        db.deleteObjectStore('engine_states');
      }
      db.createObjectStore('engine_states');
    };
    request.onsuccess = () => {
      dbInstance = request.result;
      dbVersion = BIOME_DB_VERSION;
      dbInstance.onclose = () => {
        dbInstance = null;
        dbVersion = null;
      };
      dbInstance.onerror = () => {
        dbInstance = null;
        dbVersion = null;
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

/** @internal Jest 用: IndexedDB モジュールキャッシュをクリア */
export function resetBiomeDbCacheForTests(): void {
  dbInstance = null;
  dbVersion = null;
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
  const serializedRef = useRef<string>('');
  const saveDebounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const validatedSeedRef = useRef<number>(0);

  const rarityRef = useRef<number>(0);
  const activeCellCountRef = useRef<number>(0);
  const elementBalanceRef = useRef<Uint16Array>(new Uint16Array(8));
  const mutationBoostRef = useRef<number>(1.0);
  const ticksSinceMutationRef = useRef<number>(0);
  const rarityProgressRef = useRef<RarityProgress | null>(null);
  const lastTickEventsRef = useRef<BiomeEvent[]>([]);
  const leniaMuRef = useRef<number>(0.15);
  const leniaSigmaRef = useRef<number>(0.017);
  const [leniaMu, setLeniaMu] = useState(0.15);
  const [leniaSigma, setLeniaSigma] = useState(0.017);
  // worker が処理中の tick バッチ数（バックプレッシャー制御）
  const inflightTicksRef = useRef<number>(0);

  // paused の最新状態を保持
  useEffect(() => {
    pausedRef.current = paused;
  }, [paused]);

  useEffect(() => {
    let active = true;
    const validatedSeed = Number.isFinite(seed) ? seed : 0;
    validatedSeedRef.current = validatedSeed;
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
        leniaMuRef.current = msg.leniaMu ?? 0.15;
        leniaSigmaRef.current = msg.leniaSigma ?? 0.017;
        setLeniaMu(leniaMuRef.current);
        setLeniaSigma(leniaSigmaRef.current);
        setLoading(false);
      } else if (msg.type === 'updated') {
        inflightTicksRef.current = Math.max(0, inflightTicksRef.current - 1);
        setGeneration(msg.generation);
        rarityRef.current = msg.rarity;
        activeCellCountRef.current = msg.activeCells;
        elementBalanceRef.current = msg.elementBalance;
        mutationBoostRef.current = msg.mutationBoost;
        ticksSinceMutationRef.current = msg.ticksSinceMutation;
        rarityProgressRef.current = msg.rarityProgress;
        lastTickEventsRef.current = msg.lastEvents || [];
        if (msg.leniaMu !== undefined) {
          leniaMuRef.current = msg.leniaMu;
          setLeniaMu(msg.leniaMu);
        }
        if (msg.leniaSigma !== undefined) {
          leniaSigmaRef.current = msg.leniaSigma;
          setLeniaSigma(msg.leniaSigma);
        }

        if (msg.renderView) {
          renderViewRef.current = msg.renderView;
        }

        if (saveDebounceRef.current) {
          clearTimeout(saveDebounceRef.current);
        }
        saveDebounceRef.current = setTimeout(() => {
          saveDebounceRef.current = null;
          worker.postMessage({ type: 'requestSave' });
        }, SAVE_DEBOUNCE_MS);
      } else if (msg.type === 'saved') {
        serializedRef.current = msg.serialized;
        const key = `seed_${validatedSeedRef.current}`;
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
        inflightTicksRef.current = Math.max(0, inflightTicksRef.current - 1);
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
      if (saveDebounceRef.current) {
        clearTimeout(saveDebounceRef.current);
        saveDebounceRef.current = null;
      }
      worker.terminate();
      workerRef.current = null;
    };
  }, [seed]);

  const tick = useCallback((count: number = 1) => {
    if (!workerRef.current) return;
    // worker が遅延しているときは新規バッチを送らない（メッセージ滞留＝
    // ラグ後のバースト再生によるカクツキを防止）。最大 2 バッチまで先行可。
    if (inflightTicksRef.current >= 2) return;
    inflightTicksRef.current += 1;
    workerRef.current.postMessage({ type: 'tick', count });
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
    const idx = y * GRID_WIDTH + x;
    const offset = idx * RENDER_STRIDE;

    if (offset + RENDER_STRIDE > renderViewRef.current.length) return null;

    const activeVal = renderViewRef.current[offset + 2] !== 0;
    const morphologyVal = renderViewRef.current[offset + 3];
    const isFrozenVal = renderViewRef.current[offset + 12] !== 0;

    const elements = new Uint16Array(8);
    for (let i = 0; i < 8; i++) {
      elements[i] = renderViewRef.current[offset + 4 + i];
    }

    return {
      active: activeVal,
      morphology: morphologyVal,
      elements,
      is_frozen: isFrozenVal,
      energy: elements[0],
    };
  }, []);

  const injectElement = useCallback((x: number, y: number, idx: number, amount: number) => {
    if (!workerRef.current) return;
    workerRef.current.postMessage({ type: 'inject', x, y, idx, amount });
  }, []);

  const injectBrush = useCallback(
    (x: number, y: number, radius: number, idx: number, amount: number) => {
      if (!workerRef.current) return;
      workerRef.current.postMessage({ type: 'injectBrush', x, y, radius, idx, amount });
    },
    []
  );

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

  const setLeniaParams = useCallback((mu: number, sigma: number) => {
    if (!workerRef.current) return;
    leniaMuRef.current = mu;
    leniaSigmaRef.current = sigma;
    setLeniaMu(mu);
    setLeniaSigma(sigma);
    workerRef.current.postMessage({ type: 'setLeniaParams', mu, sigma });
  }, []);

  const getLeniaMu = useCallback((): number => leniaMuRef.current, []);
  const getLeniaSigma = useCallback((): number => leniaSigmaRef.current, []);

  const paintEnv = useCallback(
    (x: number, y: number, radius: number, kind: number) => {
      if (!workerRef.current) return;
      workerRef.current.postMessage({ type: 'paintEnv', x, y, radius, kind });
    },
    []
  );

  const clearEnv = useCallback(() => {
    if (!workerRef.current) return;
    workerRef.current.postMessage({ type: 'clearEnv' });
  }, []);

  const seedEcosystem = useCallback(
    (speciesA: number, speciesB: number, competition: number) => {
      if (!workerRef.current) return;
      workerRef.current.postMessage({
        type: 'seedEcosystem',
        speciesA,
        speciesB,
        competition,
      });
    },
    []
  );

  return {
    loading,
    error,
    generation,
    tick,
    rewind,
    getRenderView,
    getCellDetail,
    injectElement,
    injectBrush,
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
    leniaMu,
    leniaSigma,
    setLeniaParams,
    getLeniaMu,
    getLeniaSigma,
    paintEnv,
    clearEnv,
    seedEcosystem,
  };
}

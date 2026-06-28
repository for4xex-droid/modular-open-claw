/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import type { BiomeEngine } from 'biome-engine';

let engine: BiomeEngine | null = null;
let wasmMemory: WebAssembly.Memory | null = null;

self.onmessage = async (e: MessageEvent) => {
  const data = e.data;

  try {
    if (data.type === 'init') {
      const wasm = await import('biome-engine');
      const initOutput = await wasm.default();
      wasmMemory = initOutput.memory;

      if (engine) {
        engine.free();
      }

      if (data.savedState) {
        engine = wasm.BiomeEngine.deserialize(data.savedState);
      } else {
        engine = new wasm.BiomeEngine(BigInt(data.seed));
      }

      const generation = Number(engine.generation());
      const rarity = engine.get_rarity();
      const activeCells = engine.get_active_cell_count();
      const elementBalance = engine.get_element_balance();
      const mutationBoost = engine.get_mutation_boost();
      const ticksSinceMutation = engine.ticks_since_mutation();
      const rarityProgress = engine.get_rarity_progress();

      self.postMessage({
        type: 'initialized',
        generation,
        rarity,
        activeCells,
        elementBalance,
        mutationBoost,
        ticksSinceMutation,
        rarityProgress,
      });

      // 初期化直後に最初の状態（renderViewなど）をメインスレッドに同期する
      sendStateUpdate();
      return;
    }

    if (!engine) {
      throw new Error('Engine not initialized');
    }

    if (data.type === 'tick') {
      engine.tick();
      sendStateUpdate();
    } else if (data.type === 'rewind') {
      const success = engine.apply_tachyon_rewind(data.generations);
      const generation = Number(engine.generation());
      const serialized = engine.serialize();
      self.postMessage({
        type: 'rewound',
        success,
        generation,
        serialized,
      });
      // 巻き戻し後も描画データを同期
      sendStateUpdate();
    } else if (data.type === 'inject') {
      engine.inject_element(data.x, data.y, data.idx, data.amount);
      sendStateUpdate();
    } else if (data.type === 'crisis') {
      engine.apply_crisis(data.crisisType, data.x, data.y);
      sendStateUpdate();
    } else if (data.type === 'setMutationBoost') {
      engine.set_mutation_boost(data.val);
      sendStateUpdate();
    }
  } catch (err: any) {
    self.postMessage({
      type: 'error',
      message: err instanceof Error ? err.message : String(err),
    });
  }
};

function sendStateUpdate() {
  if (!engine || !wasmMemory) return;

  const generation = Number(engine.generation());
  const rarity = engine.get_rarity();
  const activeCells = engine.get_active_cell_count();
  const elementBalance = engine.get_element_balance();
  const mutationBoost = engine.get_mutation_boost();
  const ticksSinceMutation = engine.ticks_since_mutation();
  const rarityProgress = engine.get_rarity_progress();
  const lastEvents = engine.get_last_tick_events() || [];
  const serialized = engine.serialize();

  // JSONをパースして is_frozen 状態を抽出 (Worker側でパースすることでメインスレッドへの負荷をゼロにする)
  const frozenCells = new Uint8Array(128 * 128);
  try {
    const data = JSON.parse(serialized);
    if (Array.isArray(data.cells)) {
      const limit = Math.min(data.cells.length, frozenCells.length);
      for (let i = 0; i < limit; i++) {
        frozenCells[i] = data.cells[i]?.is_frozen ? 1 : 0;
      }
    }
  } catch (e) {
    console.warn('Failed to parse serialized state in worker', e);
  }

  // render_data のコピーを作成して Transferable として転送
  const ptr = engine.render_data_ptr();
  const len = engine.render_data_len();
  const wasmView = new Float32Array(wasmMemory.buffer, ptr, len);
  const renderView = new Float32Array(wasmView); // コピー

  self.postMessage(
    {
      type: 'updated',
      generation,
      rarity,
      activeCells,
      elementBalance,
      mutationBoost,
      ticksSinceMutation,
      rarityProgress,
      lastEvents,
      serialized,
      renderView,
      frozenCells,
    },
    [renderView.buffer, frozenCells.buffer]
  );
}

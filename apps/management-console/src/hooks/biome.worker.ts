/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import type { BiomeEngine } from 'biome-engine';

let engine: BiomeEngine | null = null;
let wasmMemory: WebAssembly.Memory | null = null;

/** 注入後の即時反映（follow-up tick なし — 種まきをクリック直後に表示） */
const INJECT_FOLLOWUP_TICKS = 0;

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
      const leniaMu = engine.get_lenia_mu();
      const leniaSigma = engine.get_lenia_sigma();

      self.postMessage({
        type: 'initialized',
        generation,
        rarity,
        activeCells,
        elementBalance,
        mutationBoost,
        ticksSinceMutation,
        rarityProgress,
        leniaMu,
        leniaSigma,
      });

      sendStateUpdate();
      return;
    }

    if (!engine) {
      throw new Error('Engine not initialized');
    }

    if (data.type === 'tick') {
      const count = Math.max(1, Number(data.count) || 1);
      engine.tick_n(count);
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
      sendStateUpdate();
    } else if (data.type === 'inject') {
      engine.inject_element(data.x, data.y, data.idx, data.amount);
      sendStateUpdate();
    } else if (data.type === 'injectBrush') {
      engine.inject_brush(data.x, data.y, data.radius, data.idx, data.amount);
      for (let i = 0; i < INJECT_FOLLOWUP_TICKS; i++) {
        engine.tick_n(1);
      }
      sendStateUpdate();
    } else if (data.type === 'requestSave') {
      const serialized = engine.serialize();
      self.postMessage({ type: 'saved', serialized });
    } else if (data.type === 'crisis') {
      engine.apply_crisis(data.crisisType, data.x, data.y);
      sendStateUpdate();
    } else if (data.type === 'setMutationBoost') {
      engine.set_mutation_boost(data.val);
      sendStateUpdate();
    } else if (data.type === 'setLeniaParams') {
      engine.set_lenia_params(data.mu, data.sigma);
      sendStateUpdate();
    } else if (data.type === 'paintEnv') {
      engine.paint_env(data.x, data.y, data.radius, data.kind);
      sendStateUpdate();
    } else if (data.type === 'clearEnv') {
      engine.clear_env();
      sendStateUpdate();
    } else if (data.type === 'seedEcosystem') {
      engine.seed_ecosystem(data.speciesA, data.speciesB, data.competition);
      sendStateUpdate();
    }
  } catch (err: unknown) {
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
  const leniaMu = engine.get_lenia_mu();
  const leniaSigma = engine.get_lenia_sigma();

  const ptr = engine.render_data_ptr();
  const len = engine.render_data_len();
  const wasmView = new Float32Array(wasmMemory.buffer, ptr, len);
  const renderView = new Float32Array(wasmView);

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
      leniaMu,
      leniaSigma,
      renderView,
    },
    { transfer: [renderView.buffer] }
  );
}

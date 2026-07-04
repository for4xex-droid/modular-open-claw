/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import '@testing-library/jest-dom';

// Fix for React / Framer Motion usage in JSDOM which doesn't implement window.scrollTo
window.scrollTo = jest.fn();

// Global Mock Worker implementation for tests
class MockWorker {
  onmessage: ((this: MockWorker, ev: MessageEvent) => any) | null = null;
  postMessage = jest.fn().mockImplementation((message: any) => {
    setTimeout(() => {
      this.handleIncomingMessage(message);
    }, 0);
  });
  addEventListener = jest.fn();
  removeEventListener = jest.fn();
  terminate = jest.fn();

  private generation = 0;

  constructor(public url: string, public options?: any) {
    (globalThis as any).lastWorkerInstance = this;
  }

  private handleIncomingMessage(message: any) {
    if (!this.onmessage) return;

    if (message.type === 'init') {
      if (message.seed === -999) {
        this.onmessage({
          data: {
            type: 'error',
            message: 'Simulated WASM initialization failure',
          }
        } as any);
        return;
      }
      this.generation = 0;
      if (message.savedState) {
        try {
          const state = JSON.parse(message.savedState);
          this.generation = state.generation || 0;
        } catch (e) {}
      }
      this.onmessage({
        data: {
          type: 'initialized',
          generation: this.generation,
          rarity: 0,
          activeCells: 100,
          elementBalance: new Uint16Array([40, 30, 10, 20, 0, 0, 0, 0]),
          mutationBoost: 1.0,
          ticksSinceMutation: 0,
          rarityProgress: null,
          leniaMu: 0.15,
          leniaSigma: 0.017,
        }
      } as any);
      
      this.sendUpdate(message.savedState);
    } else if (message.type === 'tick') {
      const count = Math.max(1, Number(message.count) || 1);
      this.generation += count;
      this.sendUpdate();
    } else if (message.type === 'injectBrush') {
      this.sendUpdate();
    } else if (
      message.type === 'paintEnv' ||
      message.type === 'clearEnv' ||
      message.type === 'seedEcosystem' ||
      message.type === 'setLeniaParams' ||
      message.type === 'setMutationBoost' ||
      message.type === 'inject' ||
      message.type === 'crisis'
    ) {
      this.sendUpdate();
    } else if (message.type === 'requestSave') {
      this.onmessage({
        data: {
          type: 'saved',
          serialized: JSON.stringify({ generation: this.generation, seed: 42 }),
        }
      } as any);
    } else if (message.type === 'rewind') {
      if (message.generations <= this.generation) {
        this.generation -= message.generations;
      }
      this.onmessage({
        data: {
          type: 'rewound',
          success: true,
          generation: this.generation,
          serialized: JSON.stringify({ generation: this.generation, seed: 42 }),
        }
      } as any);
      this.sendUpdate();
    }
  }

  private sendUpdate(_savedState?: string | null) {
    if (!this.onmessage) return;
    this.onmessage({
      data: {
        type: 'updated',
        generation: this.generation,
        rarity: 0,
        activeCells: 100,
        elementBalance: new Uint16Array([40, 30, 10, 20, 0, 0, 0, 0]),
        mutationBoost: 1.0,
        ticksSinceMutation: 0,
        rarityProgress: null,
        lastEvents: [],
        leniaMu: 0.15,
        leniaSigma: 0.017,
        renderView: new Float32Array(16384 * 13),
      }
    } as any);
  }
}

globalThis.Worker = MockWorker as any;

// Global mock for ?worker syntax
jest.mock('./hooks/biome.worker?worker', () => {
  return jest.fn().mockImplementation(() => {
    return new MockWorker('');
  });
});


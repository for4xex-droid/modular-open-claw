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
    (global as any).lastWorkerInstance = this;
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
        }
      } as any);
      
      this.sendUpdate(message.savedState);
    } else if (message.type === 'tick') {
      this.generation += 1;
      this.sendUpdate();
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

  private sendUpdate(savedState?: string | null) {
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
        serialized: savedState || JSON.stringify({ generation: this.generation, seed: 42 }),
        renderView: new Float32Array(16384 * 12),
        frozenCells: new Uint8Array(16384),
      }
    } as any);
  }
}

global.Worker = MockWorker as any;

// Global mock for ?worker syntax
jest.mock('./hooks/biome.worker?worker', () => {
  return jest.fn().mockImplementation(() => {
    return new MockWorker('');
  });
});


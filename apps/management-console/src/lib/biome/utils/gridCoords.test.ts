/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import { canvasToGridCoords } from './gridCoords';

describe('canvasToGridCoords', () => {
  it('should correctly convert coordinates and invert Y-axis', () => {
    // Create a mock canvas with getBoundingClientRect
    const mockCanvas = {
      getBoundingClientRect: () => ({
        left: 10,
        top: 20,
        width: 256,
        height: 256,
        right: 266,
        bottom: 276,
      }),
    } as unknown as HTMLCanvasElement;

    // Center of canvas: px = 128, py = 128
    // gridX = Math.floor((128/256) * 128) = 64
    // gridY = Math.floor((1 - 128/256) * 128) = 64
    const center = canvasToGridCoords(138, 148, mockCanvas);
    expect(center).toEqual({ x: 64, y: 64 });

    // Top-left of canvas: px = 0, py = 0 (clamped)
    // gridX = 0
    // gridY = Math.floor((1 - 0) * 128) = 128 -> clamped to 127
    const topLeft = canvasToGridCoords(10, 20, mockCanvas);
    expect(topLeft).toEqual({ x: 0, y: 127 });

    // Bottom-right of canvas: px = 256, py = 256
    // gridX = 128 -> clamped to 127
    // gridY = Math.floor((1 - 1) * 128) = 0
    const bottomRight = canvasToGridCoords(266, 276, mockCanvas);
    expect(bottomRight).toEqual({ x: 127, y: 0 });
  });

  it('should return null for coordinates outside the canvas boundary', () => {
    const mockCanvas = {
      getBoundingClientRect: () => ({
        left: 10,
        top: 20,
        width: 100,
        height: 100,
      }),
    } as unknown as HTMLCanvasElement;

    const outOfBoundsLeft = canvasToGridCoords(5, 50, mockCanvas);
    expect(outOfBoundsLeft).toBeNull();

    const outOfBoundsTop = canvasToGridCoords(50, 10, mockCanvas);
    expect(outOfBoundsTop).toBeNull();
  });
});

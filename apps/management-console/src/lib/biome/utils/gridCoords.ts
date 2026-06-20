/**
 * Converts canvas pixel coordinates to 128x128 grid coordinates
 * with Y-axis inversion (WebGL bottom-left to browser top-left).
 */
export interface GridCoord {
  x: number;
  y: number;
}

export function canvasToGridCoords(
  clientX: number,
  clientY: number,
  canvas: HTMLCanvasElement
): GridCoord | null {
  if (!canvas) return null;

  const rect = canvas.getBoundingClientRect();
  const px = clientX - rect.left;
  const py = clientY - rect.top;

  // Coordinate check inside canvas boundary
  if (px < 0 || px > rect.width || py < 0 || py > rect.height) {
    return null;
  }

  const gridX = Math.floor((px / rect.width) * 128);
  const gridY = Math.floor((1 - py / rect.height) * 128);

  // Clamp values to grid size boundaries just in case
  const clampedX = Math.max(0, Math.min(127, gridX));
  const clampedY = Math.max(0, Math.min(127, gridY));

  return { x: clampedX, y: clampedY };
}

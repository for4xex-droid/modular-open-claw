/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
export interface InjectionMark {
  x: number;
  y: number;
  age: number;
  elementIdx: number;
}

// BiomeRendererProps と同じリテラルユニオン型を使用
export type EffectType = 'none' | 'higgs' | 'tachyon';

export interface BiomeCanvasProps {
  width: number;
  height: number;
  renderView: Float32Array;
  rarity?: number;  // 0-4 (Common..Legendary)
  effectType?: EffectType;
  effectIntensity?: number;
  effectCenter?: [number, number];
  onClick?: (coord: { x: number; y: number }) => void;
  onHover?: (coord: { x: number; y: number } | null) => void;
  bloomEnabled?: boolean;
  structureBonus?: boolean;
  injectionMarks?: InjectionMark[];
  /** ドラッグ中の連続塗り（環境ペン用）。種まきでは false 推奨（postMessage 氾濫防止）。 */
  dragPaint?: boolean;
}

export const RENDER_STRIDE = 13;
export const GRID_WIDTH = 128;
export const GRID_HEIGHT = 128;
export const CELL_COUNT = GRID_WIDTH * GRID_HEIGHT;
export const MORPH_COUNT = 5;
export const MORPH_NAMES = ['Basic', 'Producer', 'Consumer', 'Predator', 'Decomposer'] as const;

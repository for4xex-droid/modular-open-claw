/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import { describe, it, expect } from 'vitest';
import { parseColorToRGB } from './colorUtils';

describe('parseColorToRGB', () => {
  it('should parse 3-digit hex colors correctly', () => {
    expect(parseColorToRGB('#fff')).toEqual([1.0, 1.0, 1.0]);
    expect(parseColorToRGB('#000')).toEqual([0.0, 0.0, 0.0]);
    expect(parseColorToRGB('#f00')).toEqual([1.0, 0.0, 0.0]);
  });

  it('should parse 6-digit hex colors correctly', () => {
    expect(parseColorToRGB('#ffffff')).toEqual([1.0, 1.0, 1.0]);
    expect(parseColorToRGB('#000000')).toEqual([0.0, 0.0, 0.0]);
    const rgb = parseColorToRGB('#d4c5a9');
    expect(rgb[0]).toBeCloseTo(212 / 255);
    expect(rgb[1]).toBeCloseTo(197 / 255);
    expect(rgb[2]).toBeCloseTo(169 / 255);
  });

  it('should parse rgb/rgba colors correctly', () => {
    expect(parseColorToRGB('rgb(255, 255, 255)')).toEqual([1.0, 1.0, 1.0]);
    expect(parseColorToRGB('rgba(0, 0, 0, 0.5)')).toEqual([0.0, 0.0, 0.0]);
    const rgb = parseColorToRGB('RGB(128, 128, 128)');
    expect(rgb[0]).toBeCloseTo(128 / 255);
    expect(rgb[1]).toBeCloseTo(128 / 255);
    expect(rgb[2]).toBeCloseTo(128 / 255);
  });

  it('should return default fallback [1, 1, 1] for invalid inputs', () => {
    expect(parseColorToRGB('invalid')).toEqual([1.0, 1.0, 1.0]);
    expect(parseColorToRGB('')).toEqual([1.0, 1.0, 1.0]);
  });
});

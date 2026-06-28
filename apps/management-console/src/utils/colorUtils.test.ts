/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import { parseColorToRGB } from './colorUtils';

describe('parseColorToRGB', () => {
  it('should parse 6-character hex colors', () => {
    expect(parseColorToRGB('#ffffff')).toEqual([1, 1, 1]);
    expect(parseColorToRGB('#000000')).toEqual([0, 0, 0]);
    expect(parseColorToRGB('#ff0000')).toEqual([1, 0, 0]);
  });

  it('should parse 3-character hex colors', () => {
    expect(parseColorToRGB('#fff')).toEqual([1, 1, 1]);
    expect(parseColorToRGB('#000')).toEqual([0, 0, 0]);
    expect(parseColorToRGB('#f00')).toEqual([1, 0, 0]);
  });

  it('should parse rgb colors', () => {
    expect(parseColorToRGB('rgb(255, 255, 255)')).toEqual([1, 1, 1]);
    expect(parseColorToRGB('rgb(0, 0, 0)')).toEqual([0, 0, 0]);
    expect(parseColorToRGB('rgb(255, 0, 128)')).toEqual([1, 0, 128 / 255]);
  });

  it('should parse rgba colors and ignore alpha', () => {
    expect(parseColorToRGB('rgba(255, 255, 255, 0.5)')).toEqual([1, 1, 1]);
    expect(parseColorToRGB('rgba(0, 0, 0, 0.2)')).toEqual([0, 0, 0]);
  });

  it('should return white [1, 1, 1] for invalid format', () => {
    expect(parseColorToRGB('invalid')).toEqual([1, 1, 1]);
    expect(parseColorToRGB('')).toEqual([1, 1, 1]);
  });
});

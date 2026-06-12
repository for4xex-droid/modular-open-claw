/**
 * CSS カラー文字列 (#hex / rgb() / rgba()) を [0-1] の RGB タプルに変換。
 * WebGL uniform への色渡しに使用。
 */
export function parseColorToRGB(colorStr: string): [number, number, number] {
  if (!colorStr) {
    return [1.0, 1.0, 1.0];
  }
  const str = colorStr.trim();
  if (str.startsWith('#')) {
    const hex = str.slice(1);
    if (hex.length === 3) {
      const r = parseInt(hex[0] + hex[0], 16) / 255;
      const g = parseInt(hex[1] + hex[1], 16) / 255;
      const b = parseInt(hex[2] + hex[2], 16) / 255;
      return [r, g, b];
    }
    if (hex.length === 6) {
      const r = parseInt(hex.slice(0, 2), 16) / 255;
      const g = parseInt(hex.slice(2, 4), 16) / 255;
      const b = parseInt(hex.slice(4, 6), 16) / 255;
      return [r, g, b];
    }
  }
  const rgbMatch = str.match(/(?:rgb|rgba)\s*\(\s*(\d+)\s*,\s*(\d+)\s*,\s*(\d+)/i);
  if (rgbMatch) {
    return [
      parseInt(rgbMatch[1], 10) / 255,
      parseInt(rgbMatch[2], 10) / 255,
      parseInt(rgbMatch[3], 10) / 255
    ];
  }
  return [1.0, 1.0, 1.0];
}

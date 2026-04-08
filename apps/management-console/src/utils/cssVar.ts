/**
 * cssVar — CSS 変数の解決値を取得するユーティリティ (Canvas / Three.js 向け)
 *
 * DOM ベースの CSS 変数 (`var(--token)`) は Canvas や WebGL コンテキストでは
 * 解決されないため、`getComputedStyle` で実値を取得してキャッシュする。
 *
 * @example
 *   import { cssVar, clearCssVarCache } from '../utils/cssVar';
 *   const color = cssVar('--accent-cyan', '#00f2ff');
 *
 * キャッシュは O(1) ルックアップ。テーマ切り替え時は clearCssVarCache() を呼ぶ。
 */

const _cache: Record<string, string> = {};

/**
 * CSS 変数の解決値を返す。初回はDOMから取得しキャッシュ。
 * SSR / テスト環境など document が存在しない場合は fallback を返す。
 */
export const cssVar = (name: string, fallback?: string): string => {
  if (_cache[name]) return _cache[name];
  
  // If we are in SSR / Node missing document
  if (typeof document === 'undefined') {
    return fallback ?? '#ff00ff'; // Magenta debug color highlights unresolved tokens
  }
  
  const val = getComputedStyle(document.documentElement).getPropertyValue(name).trim();
  if (val) {
    _cache[name] = val;
    return val;
  }
  
  console.warn(`[Aiome Design System] Missing token: '${name}'. Check tokens.css`);
  return fallback ?? '#ff00ff'; // Magenta visible debug color
};

/**
 * キャッシュをクリアする。テーマ切り替え時に呼び出す。
 */
export const clearCssVarCache = (): void => {
  for (const key in _cache) {
    delete _cache[key];
  }
};

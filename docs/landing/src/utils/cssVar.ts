/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
/**
 * cssVar — CSS 変数の解決値を取得するユーティリティ (Canvas / Three.js 向け)
 *
 * DOM ベースの CSS 変数 (`var(--token)`) は Canvas や WebGL コンテキストでは
 * 解決されないため、`getComputedStyle` で実値を取得してキャッシュする。
 */

const _cache: Record<string, string> = {};

/**
 * CSS 変数の解決値を返す。初回はDOMから取得しキャッシュ。
 * SSR / テスト環境など document が存在しない場合は fallback を返す。
 */
export const cssVar = (name: string, fallback?: string): string => {
  if (_cache[name]) return _cache[name];
  
  if (typeof document === 'undefined') {
    return fallback ?? '#ff00ff'; // Magenta debug color highlights unresolved tokens
  }
  
  const val = getComputedStyle(document.documentElement).getPropertyValue(name).trim();
  if (val) {
    _cache[name] = val;
    return val;
  }
  
  console.warn(`[Aiome Design System] Missing token: '${name}'. Check index.css`);
  return fallback ?? '#ff00ff';
};

/**
 * キャッシュをクリアする。テーマ切り替え時に呼び出す。
 */
export const clearCssVarCache = (): void => {
  for (const key in _cache) {
    delete _cache[key];
  }
};

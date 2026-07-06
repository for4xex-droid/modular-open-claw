/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import en from './en.json';
import ja from './ja.json';

function resolveKey(obj: Record<string, unknown>, path: string): string | undefined {
  const parts = path.split('.');
  let current: unknown = obj;
  for (const part of parts) {
    if (current == null || typeof current !== 'object') return undefined;
    current = (current as Record<string, unknown>)[part];
  }
  return typeof current === 'string' ? current : undefined;
}

describe('i18n Dictionary Parity', () => {
  // 再帰的にすべてのキーパスを取得するヘルパー
  const getKeys = (obj: any, prefix = ''): string[] => {
    return Object.keys(obj).reduce((res: string[], el: string) => {
      const isObject = typeof obj[el] === 'object' && obj[el] !== null && !Array.isArray(obj[el]);
      if (isObject) {
        return [...res, ...getKeys(obj[el], prefix + el + '.')];
      }
      return [...res, prefix + el];
    }, []);
  };

  it('should have the exact same keys in en.json and ja.json', () => {
    const enKeys = getKeys(en).sort();
    const jaKeys = getKeys(ja).sort();

    // en.json にあって ja.json にないキー
    const missingInJa = enKeys.filter(k => !jaKeys.includes(k));
    // ja.json にあって en.json にないキー
    const missingInEn = jaKeys.filter(k => !enKeys.includes(k));

    expect(missingInJa).toEqual([]);
    expect(missingInEn).toEqual([]);
    expect(enKeys).toEqual(jaKeys);
  });

  it('should resolve known keys in both locales', () => {
    expect(resolveKey(ja, 'common.close')).toBe('閉じる');
    expect(resolveKey(en, 'common.close')).toBe('Close');
    expect(resolveKey(ja, 'buzz.title')).toBeTruthy();
    expect(resolveKey(en, 'buzz.title')).toBeTruthy();
  });

  it('should fall back to defaultValue when key is missing', () => {
    const missingKey = 'nonexistent.key.path';
    expect(resolveKey(ja, missingKey)).toBeUndefined();
    const fallback = resolveKey(ja, missingKey) ?? 'Fallback text';
    expect(fallback).toBe('Fallback text');
  });
});

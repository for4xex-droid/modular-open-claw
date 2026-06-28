/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import en from './en.json';
import ja from './ja.json';

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
});

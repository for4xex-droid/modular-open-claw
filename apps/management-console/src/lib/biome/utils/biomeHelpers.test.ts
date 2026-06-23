import { getPercentageMap } from './biomeHelpers';

describe('biomeHelpers - getPercentageMap', () => {
  it('Record<string, number> 型のオブジェクトを正しくパーセンテージに変換すること', () => {
    const input = { C: 40, N: 10, P: 50 };
    const result = getPercentageMap(input);
    expect(result).toEqual([
      { key: 'C', pct: 40 },
      { key: 'N', pct: 10 },
      { key: 'P', pct: 50 },
    ]);
  });

  it('JSON 文字列を正しくパーセンテージに変換すること', () => {
    const input = '{"C":40,"N":10,"P":50}';
    const result = getPercentageMap(input);
    expect(result).toEqual([
      { key: 'C', pct: 40 },
      { key: 'N', pct: 10 },
      { key: 'P', pct: 50 },
    ]);
  });

  it('合計値が 0 の場合は空の配列を返すこと', () => {
    const input = { C: 0, N: 0 };
    const result = getPercentageMap(input);
    expect(result).toEqual([]);
  });

  it('空オブジェクトまたは undefined の場合は空の配列を返すこと', () => {
    expect(getPercentageMap(undefined)).toEqual([]);
    expect(getPercentageMap({})).toEqual([]);
  });

  it('不正な JSON 文字列の場合は空の配列を返すこと', () => {
    expect(getPercentageMap('invalid-json')).toEqual([]);
  });
});

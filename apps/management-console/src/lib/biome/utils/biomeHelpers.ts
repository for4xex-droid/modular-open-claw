export const ELEMENT_COLORS: Record<string, string> = {
  C: 'var(--biome-element-c)',
  N: 'var(--biome-element-n)',
  P: 'var(--biome-element-p)',
  H: 'var(--biome-element-h)',
  O: 'var(--biome-element-o)',
  S: 'var(--biome-element-s)',
  Fe: 'var(--biome-element-fe)',
  Si: 'var(--biome-element-si)',
};

export const MORPH_COLORS: Record<string, string> = {
  Basic: 'var(--biome-morph-basic)',
  Producer: 'var(--biome-morph-producer)',
  Consumer: 'var(--biome-morph-consumer)',
  Predator: 'var(--biome-morph-predator)',
  Decomposer: 'var(--biome-morph-decomposer)',
};

export interface PercentageItem {
  key: string;
  pct: number;
}

export const getPercentageMap = (data?: Record<string, number> | string): PercentageItem[] => {
  if (!data) return [];
  try {
    const parsed = typeof data === 'string' ? (JSON.parse(data) as Record<string, number>) : data;
    const total = Object.values(parsed).reduce((a, b) => a + b, 0);
    if (total === 0) return [];
    return Object.entries(parsed).map(([key, val]) => ({
      key,
      pct: (val / total) * 100,
    }));
  } catch {
    return [];
  }
};

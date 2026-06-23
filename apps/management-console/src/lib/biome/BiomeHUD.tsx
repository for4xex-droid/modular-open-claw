

export interface BiomeHUDProps {
  generation: number;
  rarity: string;
  elementBalance: Record<string, number>;
  mutationBoost?: number;
  ticksSinceMutation?: number;
  activeCellCount?: number;
  rarityProgress?: {
    active_cells: number;
    morphology_count: number;
    has_homeostasis: boolean;
    diversity_index: number;
    condition_active_500: boolean;
    condition_morph_3: boolean;
    condition_morph_4: boolean;
    condition_active_1000: boolean;
  } | null;
}

export function BiomeHUD({
  generation,
  rarity,
  elementBalance,
  mutationBoost = 1.0,
  ticksSinceMutation = 0,
  activeCellCount = 0,
  rarityProgress = null
}: BiomeHUDProps) {
  // レアリティごとの色と絵文字設定
  const getRarityDecor = (r: string) => {
    switch (r) {
      case 'Legendary': return {
        color: 'var(--biome-rarity-legendary)',
        border: 'var(--biome-rarity-legendary-border)',
        glow: 'var(--biome-rarity-legendary-glow)',
        emoji: '🔥',
        label: 'Legendary'
      };
      case 'Epic': return {
        color: 'var(--biome-rarity-epic)',
        border: 'var(--biome-rarity-epic-border)',
        glow: 'var(--biome-rarity-epic-glow)',
        emoji: '🔮',
        label: 'Epic'
      };
      case 'Rare': return {
        color: 'var(--biome-rarity-rare)',
        border: 'var(--biome-rarity-rare-border)',
        glow: 'var(--biome-rarity-rare-glow)',
        emoji: '💎',
        label: 'Rare'
      };
      case 'Uncommon': return {
        color: 'var(--biome-rarity-uncommon)',
        border: 'var(--biome-rarity-uncommon-border)',
        glow: 'var(--biome-rarity-uncommon-glow)',
        emoji: '🌟',
        label: 'Uncommon'
      };
      default: return {
        color: 'var(--biome-rarity-common)',
        border: 'var(--biome-rarity-common-border)',
        glow: 'var(--biome-rarity-common-glow)',
        emoji: '🍃',
        label: 'Common'
      };
    }
  };

  const decor = getRarityDecor(rarity);

  return (
    <div style={{
      padding: 'var(--space-sm)',
      background: 'var(--bg-glass-heavy)',
      backdropFilter: 'blur(var(--blur-md))',
      border: '1px solid var(--border-glass)',
      borderRadius: 'var(--radius-md)',
      color: 'var(--white-100)',
      display: 'flex',
      flexDirection: 'column',
      gap: 'var(--space-xs)',
      fontFamily: 'var(--font-main)',
      position: 'relative'
    }}>
      <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: 'var(--space-2xs)' }}>
        <div>
          <span style={{ fontSize: 'var(--font-size-2xs)', color: 'var(--text-muted)', fontWeight: '600' }}>現在の世代</span>
          <div 
            style={{ fontSize: '2rem', fontWeight: 'bold', animation: 'neonGlow 2s infinite ease-in-out', color: 'var(--accent-cyan, #06b6d4)' }}
            data-testid="biome-generation"
          >
            {generation}
          </div>
        </div>
        <div style={{ textAlign: 'right' }}>
          <span style={{ fontSize: 'var(--font-size-2xs)', color: 'var(--text-muted)', fontWeight: '600' }}>評価ランク</span>
          <div 
            style={{
              fontSize: '0.85rem',
              fontWeight: 'bold',
              color: decor.color,
              background: 'var(--white-03, rgba(255, 255, 255, 0.03))',
              padding: '4px 8px',
              borderRadius: 'var(--radius-sm)',
              marginTop: '4px',
              border: `1px solid ${decor.border}`,
              boxShadow: `0 0 8px ${decor.glow}`,
              display: 'flex',
              alignItems: 'center',
              gap: '4px'
            }}
            data-testid="biome-rarity"
          >
            <span>{decor.emoji}</span>
            <span>{decor.label}</span>
          </div>
        </div>
      </div>

      <div style={{ display: 'flex', justifyContent: 'space-between', borderTop: '1px solid var(--border-glass)', paddingTop: 'var(--space-2xs)' }}>
        <div>
          <span style={{ fontSize: 'var(--font-size-2xs)', color: 'var(--text-muted)', fontWeight: '600' }}>活性セル数</span>
          <div 
            style={{ fontSize: '1.25rem', fontWeight: 'bold', color: 'var(--white-100)' }}
            data-testid="biome-active-cells"
          >
            {activeCellCount}
          </div>
        </div>
      </div>

      {/* 条件型レアリティ進捗チェックリスト */}
      {rarityProgress && (
        <div style={{
          borderTop: '1px solid var(--border-glass)',
          paddingTop: 'var(--space-2xs)',
          display: 'flex',
          flexDirection: 'column',
          gap: '6px'
        }}>
          <span style={{ fontSize: 'var(--font-size-2xs)', color: 'var(--text-muted)', display: 'block', marginBottom: '4px', fontWeight: '600' }}>
            ランクアップ条件 (Legendary)
          </span>
          <div style={{ display: 'flex', flexDirection: 'column', gap: '3px', fontSize: '0.75rem' }}>
            <div style={{ display: 'flex', justifyContent: 'space-between', color: rarityProgress.condition_active_500 ? 'var(--biome-rarity-uncommon)' : 'var(--white-60)' }}>
              <span>{rarityProgress.condition_active_500 ? '✅' : '🔳'} 活性セル 500+</span>
              <span>({rarityProgress.active_cells}/500)</span>
            </div>
            <div style={{ display: 'flex', justifyContent: 'space-between', color: rarityProgress.condition_morph_3 ? 'var(--biome-rarity-uncommon)' : 'var(--white-60)' }}>
              <span>{rarityProgress.condition_morph_3 ? '✅' : '🔳'} 特殊形態 3種類+</span>
              <span>({rarityProgress.morphology_count}/3)</span>
            </div>
            <div style={{ display: 'flex', justifyContent: 'space-between', color: rarityProgress.condition_morph_4 ? 'var(--biome-rarity-uncommon)' : 'var(--white-60)' }}>
              <span>{rarityProgress.condition_morph_4 ? '✅' : '🔳'} 特殊形態 4種類+</span>
              <span>({rarityProgress.morphology_count}/4)</span>
            </div>
            <div style={{ display: 'flex', justifyContent: 'space-between', color: rarityProgress.condition_active_1000 ? 'var(--biome-rarity-uncommon)' : 'var(--white-60)' }}>
              <span>{rarityProgress.condition_active_1000 ? '✅' : '🔳'} 活性セル 1000+</span>
              <span>({rarityProgress.active_cells}/1000)</span>
            </div>
            <div style={{ display: 'flex', justifyContent: 'space-between', color: rarityProgress.has_homeostasis ? 'var(--biome-rarity-uncommon)' : 'var(--white-60)' }}>
              <span>{rarityProgress.has_homeostasis ? '✅' : '🔳'} 元素バランス (Homeostasis)</span>
              <span>{rarityProgress.has_homeostasis ? '安定' : '偏りあり'}</span>
            </div>
            <div style={{ display: 'flex', justifyContent: 'space-between', color: 'var(--white-80)' }}>
              <span>📊 多様性指数 (Shannon)</span>
              <span>{rarityProgress.diversity_index.toFixed(3)}</span>
            </div>
          </div>
        </div>
      )}

      {/* ブーストゲージ & 天井カウンター */}
      <div style={{ display: 'flex', flexDirection: 'column', gap: 'var(--space-xs)', borderTop: '1px solid var(--border-glass)', paddingTop: 'var(--space-2xs)' }}>
        <div>
          <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: 'var(--font-size-xs)', marginBottom: '4px' }}>
            <span style={{ color: 'var(--text-muted)', fontWeight: '600' }}>変異確率ブースト</span>
            <span 
              style={{ color: 'var(--accent-cyan)', fontWeight: 'bold' }}
              data-testid="biome-mutation-boost"
            >
              {mutationBoost.toFixed(2)}x
            </span>
          </div>
          <div style={{ height: '6px', background: 'var(--white-05)', borderRadius: '3px', overflow: 'hidden' }}>
            <div style={{
              height: '100%',
              width: `${Math.min(100, Math.max(0, ((mutationBoost - 1.0) / 1.0) * 100))}%`,
              background: 'linear-gradient(90deg, var(--biome-gauge-boost-start), var(--biome-gauge-boost-end))',
              borderRadius: '3px',
              transition: 'width var(--speed-fast) ease',
              animation: 'hudPulse 1.5s infinite ease-in-out'
            }} />
          </div>
        </div>

        <div>
          <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: 'var(--font-size-xs)', marginBottom: '4px' }}>
            <span style={{ color: 'var(--text-muted)', fontWeight: '600' }}>変異天井カウンター</span>
            <span 
              style={{ color: 'var(--accent-rose)', fontWeight: 'bold' }}
              data-testid="biome-mutation-pity"
            >
              {ticksSinceMutation}/1000
            </span>
          </div>
          <div style={{ height: '6px', background: 'var(--white-05)', borderRadius: '3px', overflow: 'hidden' }}>
            <div style={{
              height: '100%',
              width: `${Math.min(100, (ticksSinceMutation / 1000) * 100)}%`,
              background: 'linear-gradient(90deg, var(--biome-gauge-pity-start), var(--biome-gauge-pity-end))',
              borderRadius: '3px',
              transition: 'width var(--speed-fast) ease',
              animation: ticksSinceMutation > 800 ? 'hudPulse 0.5s infinite ease-in-out' : 'none'
            }} />
          </div>
        </div>
      </div>

      <div style={{ borderTop: '1px solid var(--border-glass)', paddingTop: 'var(--space-2xs)' }}>
        <span style={{ fontSize: 'var(--font-size-2xs)', color: 'var(--text-muted)', display: 'block', marginBottom: '6px', fontWeight: '600' }}>
          元素バランス比率
        </span>
        <div style={{ display: 'flex', flexDirection: 'column', gap: '4px' }}>
          {Object.entries(elementBalance).map(([el, val]) => {
            // 元素固有カラー
            const elColors: Record<string, string> = {
              C: 'var(--biome-element-c)', N: 'var(--biome-element-n)', P: 'var(--biome-element-p)', H: 'var(--biome-element-h)',
              O: 'var(--biome-element-o)', S: 'var(--biome-element-s)', Fe: 'var(--biome-element-fe)', Si: 'var(--biome-element-si)'
            };
            const barColor = elColors[el] || '#888';
            return (
              <div key={el} style={{ display: 'flex', alignItems: 'center', gap: '6px' }}>
                <span style={{
                  fontSize: 'var(--font-size-2xs)',
                  color: barColor,
                  fontWeight: 'bold',
                  width: '22px',
                  textAlign: 'right',
                  flexShrink: 0
                }}>{el}</span>
                <div style={{
                  flex: 1,
                  height: '8px',
                  background: 'rgba(255,255,255,0.04)',
                  borderRadius: '4px',
                  overflow: 'hidden',
                  position: 'relative'
                }}>
                  <div style={{
                    height: '100%',
                    width: `${Math.min(100, val)}%`,
                    background: `linear-gradient(90deg, ${barColor}88, ${barColor})`,
                    borderRadius: '4px',
                    transition: 'width 0.3s ease-out, box-shadow 0.3s ease-out',
                    boxShadow: val > 0 ? `0 0 6px ${barColor}66` : 'none'
                  }} />
                </div>
                <span style={{
                  fontSize: 'var(--font-size-2xs)',
                  color: val > 0 ? 'var(--white-90)' : 'var(--text-muted)',
                  fontWeight: '600',
                  width: '32px',
                  textAlign: 'right',
                  flexShrink: 0,
                  transition: 'color 0.3s ease'
                }}>{val}%</span>
              </div>
            );
          })}
        </div>
      </div>
    </div>
  );
}


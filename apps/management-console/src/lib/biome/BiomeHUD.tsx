import React from 'react';

export interface BiomeHUDProps {
  generation: number;
  rarity: string;
  elementBalance: Record<string, number>;
  mutationBoost?: number;
  ticksSinceMutation?: number;
  activeCellCount?: number;
}

export function BiomeHUD({
  generation,
  rarity,
  elementBalance,
  mutationBoost = 1.0,
  ticksSinceMutation = 0,
  activeCellCount = 0
}: BiomeHUDProps) {
  // レアリティごとの色と絵文字設定
  const getRarityDecor = (r: string) => {
    switch (r) {
      case 'Legendary': return { color: '#f59e0b', emoji: '🔥', label: 'Legendary' };
      case 'Epic': return { color: '#d946ef', emoji: '🔮', label: 'Epic' };
      case 'Rare': return { color: '#06b6d4', emoji: '💎', label: 'Rare' };
      case 'Uncommon': return { color: '#10b981', emoji: '🌟', label: 'Uncommon' };
      default: return { color: '#94a3b8', emoji: '🍃', label: 'Common' };
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
      {/* HUD専用アニメーション定義 */}
      <style dangerouslySetInnerHTML={{__html: `
        @keyframes hudPulse {
          0% { opacity: 0.9; filter: brightness(1); }
          50% { opacity: 0.7; filter: brightness(1.2); }
          100% { opacity: 0.9; filter: brightness(1); }
        }
        @keyframes neonGlow {
          0% { text-shadow: 0 0 2px rgba(6, 182, 212, 0.4); }
          50% { text-shadow: 0 0 8px rgba(6, 182, 212, 0.9); }
          100% { text-shadow: 0 0 2px rgba(6, 182, 212, 0.4); }
        }
      `}} />

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
              background: 'rgba(255, 255, 255, 0.03)',
              padding: '4px 8px',
              borderRadius: 'var(--radius-sm)',
              marginTop: '4px',
              border: `1px solid ${decor.color}33`,
              boxShadow: `0 0 8px ${decor.color}22`,
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
              background: 'linear-gradient(90deg, #06b6d4, #3b82f6)',
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
              background: 'linear-gradient(90deg, #f43f5e, #f97316)',
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
              C: '#33ff55', N: '#4488ff', P: '#ff9922', H: '#cc44ff',
              O: '#00ddff', S: '#ffdd33', Fe: '#ff5544', Si: '#aaaaee'
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


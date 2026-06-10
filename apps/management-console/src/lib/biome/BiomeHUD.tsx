

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
      fontFamily: 'var(--font-main)'
    }}>
      <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: 'var(--space-xs)' }}>
        <div>
          <span style={{ fontSize: 'var(--font-size-sm)', color: 'var(--text-muted)' }}>GENERATION</span>
          <div style={{ fontSize: 'var(--font-size-2xl)', fontWeight: 'bold' }}>{generation}</div>
        </div>
        <div style={{ textAlign: 'right' }}>
          <span style={{ fontSize: 'var(--font-size-sm)', color: 'var(--text-muted)' }}>RARITY</span>
          <div style={{
            fontSize: 'var(--font-size-base)',
            fontWeight: 'bold',
            color: rarity === 'Legendary' ? 'var(--accent-amber)' : rarity === 'Epic' ? 'var(--accent-purple)' : rarity === 'Rare' ? 'var(--accent-cyan)' : 'var(--text-muted)',
            background: 'var(--white-04)',
            padding: 'var(--space-2xs) var(--space-xs)',
            borderRadius: 'var(--radius-sm)',
            marginTop: 'var(--space-2xs)',
            border: '1px solid var(--border-glass)'
          }}>
            {rarity}
          </div>
        </div>
      </div>

      <div style={{ display: 'flex', justifyContent: 'space-between', borderTop: '1px solid var(--border-glass)', paddingTop: 'var(--space-xs)' }}>
        <div>
          <span style={{ fontSize: 'var(--font-size-sm)', color: 'var(--text-muted)' }}>ACTIVE CELLS</span>
          <div style={{ fontSize: 'var(--font-size-lg)', fontWeight: '600' }}>{activeCellCount}</div>
        </div>
      </div>

      {/* ブーストゲージ & 天井カウンター */}
      <div style={{ display: 'flex', flexDirection: 'column', gap: 'var(--space-xs)', borderTop: '1px solid var(--border-glass)', paddingTop: 'var(--space-xs)' }}>
        <div>
          <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: 'var(--font-size-xs)', marginBottom: 'var(--space-2xs)' }}>
            <span style={{ color: 'var(--text-muted)' }}>MUTATION BOOST</span>
            <span style={{ color: 'var(--accent-cyan)', fontWeight: 'bold' }}>{mutationBoost.toFixed(2)}x</span>
          </div>
          <div style={{ height: 'var(--size-bar-sm)', background: 'var(--white-05)', borderRadius: 'var(--space-2xs)', overflow: 'hidden' }}>
            <div style={{
              height: '100%',
              width: `${Math.min(100, Math.max(0, ((mutationBoost - 1.0) / 1.0) * 100))}%`,
              background: 'linear-gradient(90deg, var(--accent-cyan), var(--accent-blue))',
              borderRadius: 'var(--space-2xs)',
              transition: 'width var(--speed-fast) ease',
            }} />
          </div>
        </div>

        <div>
          <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: 'var(--font-size-xs)', marginBottom: 'var(--space-2xs)' }}>
            <span style={{ color: 'var(--text-muted)' }}>MUTATION PITY</span>
            <span style={{ color: 'var(--accent-rose)', fontWeight: 'bold' }}>{ticksSinceMutation}/1000</span>
          </div>
          <div style={{ height: 'var(--size-bar-sm)', background: 'var(--white-05)', borderRadius: 'var(--space-2xs)', overflow: 'hidden' }}>
            <div style={{
              height: '100%',
              width: `${Math.min(100, (ticksSinceMutation / 1000) * 100)}%`,
              background: 'linear-gradient(90deg, var(--accent-rose), var(--accent-orange))',
              borderRadius: 'var(--space-2xs)',
              transition: 'width var(--speed-fast) ease',
            }} />
          </div>
        </div>
      </div>

      <div style={{ borderTop: '1px solid var(--border-glass)', paddingTop: 'var(--space-xs)' }}>
        <span style={{ fontSize: 'var(--font-size-sm)', color: 'var(--text-muted)', display: 'block', marginBottom: 'var(--space-xs)' }}>ELEMENT BALANCE</span>
        <div style={{ display: 'grid', gridTemplateColumns: 'repeat(4, 1fr)', gap: 'var(--space-xs)' }}>
          {Object.entries(elementBalance).map(([el, val]) => (
            <div key={el} style={{
              background: 'var(--white-02)',
              border: '1px solid var(--white-04)',
              borderRadius: 'var(--radius-sm)',
              padding: 'var(--size-bar-sm)',
              textAlign: 'center'
            }}>
              <div style={{ fontSize: 'var(--font-size-2xs)', color: 'var(--text-muted)' }}>{el}</div>
              <div style={{ fontSize: 'var(--font-size-sm)', fontWeight: '600' }}>{val}%</div>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}




export interface BiomeHUDProps {
  generation: number;
  rarity: string;
  elementBalance: Record<string, number>;
}

export function BiomeHUD({ generation, rarity, elementBalance }: BiomeHUDProps) {
  return (
    <div style={{
      padding: '16px',
      background: 'var(--bg-glass-heavy)',
      backdropFilter: 'blur(12px)',
      border: '1px solid var(--border-glass)',
      borderRadius: '12px',
      color: 'var(--white-100)',
      fontFamily: 'system-ui, sans-serif'
    }}>
      <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: '16px' }}>
        <div>
          <span style={{ fontSize: '12px', color: 'var(--text-muted)' }}>GENERATION</span>
          <div style={{ fontSize: '24px', fontWeight: 'bold' }}>{generation}</div>
        </div>
        <div style={{ textAlign: 'right' }}>
          <span style={{ fontSize: '12px', color: 'var(--text-muted)' }}>RARITY</span>
          <div style={{
            fontSize: '14px',
            fontWeight: 'bold',
            color: rarity === 'Legendary' ? 'var(--accent-amber)' : rarity === 'Epic' ? 'var(--accent-purple)' : rarity === 'Rare' ? 'var(--accent-cyan)' : 'var(--text-muted)',
            background: 'var(--white-04)',
            padding: '4px 8px',
            borderRadius: '6px',
            marginTop: '4px',
            border: '1px solid var(--border-glass)'
          }}>
            {rarity}
          </div>
        </div>
      </div>

      <div>
        <span style={{ fontSize: '12px', color: 'var(--text-muted)', display: 'block', marginBottom: '8px' }}>ELEMENT BALANCE</span>
        <div style={{ display: 'grid', gridTemplateColumns: 'repeat(4, 1fr)', gap: '8px' }}>
          {Object.entries(elementBalance).map(([el, val]) => (
            <div key={el} style={{
              background: 'var(--white-02)',
              border: '1px solid var(--white-04)',
              borderRadius: '8px',
              padding: '8px',
              textAlign: 'center'
            }}>
              <div style={{ fontSize: '10px', color: 'var(--text-muted)' }}>{el}</div>
              <div style={{ fontSize: '14px', fontWeight: '600' }}>{val}%</div>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}

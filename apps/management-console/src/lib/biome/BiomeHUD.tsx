

export interface BiomeHUDProps {
  generation: number;
  rarity: string;
  elementBalance: Record<string, number>;
}

export function BiomeHUD({ generation, rarity, elementBalance }: BiomeHUDProps) {
  return (
    <div style={{
      padding: '16px',
      background: 'rgba(20, 20, 30, 0.6)',
      backdropFilter: 'blur(12px)',
      border: '1px solid rgba(255, 255, 255, 0.08)',
      borderRadius: '12px',
      color: '#fff',
      fontFamily: 'system-ui, sans-serif'
    }}>
      <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: '16px' }}>
        <div>
          <span style={{ fontSize: '12px', color: '#889' }}>GENERATION</span>
          <div style={{ fontSize: '24px', fontWeight: 'bold' }}>{generation}</div>
        </div>
        <div style={{ textAlign: 'right' }}>
          <span style={{ fontSize: '12px', color: '#889' }}>RARITY</span>
          <div style={{
            fontSize: '14px',
            fontWeight: 'bold',
            color: rarity === 'Legendary' ? '#ffd700' : rarity === 'Epic' ? '#c084fc' : rarity === 'Rare' ? '#60a5fa' : '#a1a1aa',
            background: 'rgba(255, 255, 255, 0.04)',
            padding: '4px 8px',
            borderRadius: '6px',
            marginTop: '4px',
            border: '1px solid rgba(255, 255, 255, 0.08)'
          }}>
            {rarity}
          </div>
        </div>
      </div>

      <div>
        <span style={{ fontSize: '12px', color: '#889', display: 'block', marginBottom: '8px' }}>ELEMENT BALANCE</span>
        <div style={{ display: 'grid', gridTemplateColumns: 'repeat(4, 1fr)', gap: '8px' }}>
          {Object.entries(elementBalance).map(([el, val]) => (
            <div key={el} style={{
              background: 'rgba(255, 255, 255, 0.02)',
              border: '1px solid rgba(255, 255, 255, 0.04)',
              borderRadius: '8px',
              padding: '8px',
              textAlign: 'center'
            }}>
              <div style={{ fontSize: '10px', color: '#889' }}>{el}</div>
              <div style={{ fontSize: '14px', fontWeight: '600' }}>{val}%</div>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}

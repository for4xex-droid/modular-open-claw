

export interface Specimen {
  id: string;
  name: string;
  generation: number;
  rarity: string;
  date: string;
}

export interface BiomeDendouProps {
  list: Specimen[];
  onLoad: (id: string) => void;
}

export function BiomeDendou({ list, onLoad }: BiomeDendouProps) {
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
      <h3 style={{ margin: '0 0 16px 0', fontSize: '16px', fontWeight: 'bold' }}>🏛️ 殿堂入り標本</h3>
      {list.length === 0 ? (
        <div style={{ color: 'var(--text-muted)', fontSize: '14px', textAlign: 'center', padding: '16px' }}>
          保存された標本はまだありません。
        </div>
      ) : (
        <div style={{ display: 'flex', flexDirection: 'column', gap: '8px' }}>
          {list.map((sp) => (
            <div 
              key={sp.id} 
              style={{
                display: 'flex',
                justifyContent: 'space-between',
                alignItems: 'center',
                background: 'var(--white-02)',
                border: '1px solid var(--white-04)',
                borderRadius: '8px',
                padding: '12px 16px'
              }}
              data-testid="dendou-specimen"
            >
              <div>
                <div style={{ fontWeight: '600', fontSize: '14px' }}>{sp.name}</div>
                <div style={{ fontSize: '11px', color: 'var(--text-muted)', marginTop: '4px' }}>
                  世代: {sp.generation} | ランク: {sp.rarity} | 日付: {sp.date}
                </div>
              </div>
              <button
                style={{
                  background: 'var(--accent-cyan-15)',
                  border: '1px solid var(--accent-cyan-30)',
                  borderRadius: '6px',
                  color: 'var(--accent-cyan)',
                  padding: '6px 12px',
                  cursor: 'pointer',
                  fontSize: '12px',
                  fontWeight: '600',
                  transition: 'all 0.2s'
                }}
                onClick={() => onLoad(sp.id)}
                aria-label="Load"
                data-testid="dendou-load"
              >
                📂 読込
              </button>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

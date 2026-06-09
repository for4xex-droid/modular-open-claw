

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
      background: 'rgba(20, 20, 30, 0.6)',
      backdropFilter: 'blur(12px)',
      border: '1px solid rgba(255, 255, 255, 0.08)',
      borderRadius: '12px',
      color: '#fff',
      fontFamily: 'system-ui, sans-serif'
    }}>
      <h3 style={{ margin: '0 0 16px 0', fontSize: '16px', fontWeight: 'bold' }}>HALL OF FAME</h3>
      {list.length === 0 ? (
        <div style={{ color: '#889', fontSize: '14px', textAlign: 'center', padding: '16px' }}>
          No legendary specimens saved yet.
        </div>
      ) : (
        <div style={{ display: 'flex', flexDirection: 'column', gap: '8px' }}>
          {list.map((sp) => (
            <div key={sp.id} style={{
              display: 'flex',
              justifyContent: 'space-between',
              alignItems: 'center',
              background: 'rgba(255, 255, 255, 0.02)',
              border: '1px solid rgba(255, 255, 255, 0.04)',
              borderRadius: '8px',
              padding: '12px 16px'
            }}>
              <div>
                <div style={{ fontWeight: '600', fontSize: '14px' }}>{sp.name}</div>
                <div style={{ fontSize: '11px', color: '#889', marginTop: '4px' }}>
                  Gen: {sp.generation} | {sp.rarity} | {sp.date}
                </div>
              </div>
              <button
                style={{
                  background: 'rgba(96, 165, 250, 0.15)',
                  border: '1px solid rgba(96, 165, 250, 0.3)',
                  borderRadius: '6px',
                  color: '#60a5fa',
                  padding: '6px 12px',
                  cursor: 'pointer',
                  fontSize: '12px',
                  fontWeight: '600',
                  transition: 'all 0.2s'
                }}
                onClick={() => onLoad(sp.id)}
                aria-label="Load"
              >
                Load
              </button>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

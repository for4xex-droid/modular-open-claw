import React from 'react';

export interface BiomeResultProps {
  generation: number;
  rarity: string;
  onSave: () => void;
  onClose: () => void;
}

export function BiomeResult({ generation, rarity, onSave, onClose }: BiomeResultProps) {
  const containerStyle: React.CSSProperties = {
    position: 'fixed',
    top: '50%',
    left: '50%',
    transform: 'translate(-50%, -50%)',
    background: 'var(--bg-deep-glass)',
    backdropFilter: 'blur(20px)',
    border: '1px solid var(--border-glass-bright)',
    borderRadius: '16px',
    padding: '32px',
    color: 'var(--white-100)',
    width: '400px',
    textAlign: 'center',
    boxShadow: 'var(--shadow-deep)',
    fontFamily: 'system-ui, sans-serif',
    zIndex: 1000
  };

  const buttonStyle: React.CSSProperties = {
    background: 'var(--white-08)',
    border: '1px solid var(--border-glass)',
    borderRadius: '8px',
    color: 'var(--white-100)',
    padding: '10px 20px',
    cursor: 'pointer',
    fontWeight: 'bold',
    margin: '8px',
    transition: 'all 0.2s'
  };

  const saveButtonStyle: React.CSSProperties = {
    ...buttonStyle,
    background: 'linear-gradient(135deg, var(--accent-amber), var(--accent-amber-30))',
    color: 'var(--text-inverse)',
    border: 'none'
  };

  return (
    <div style={containerStyle}>
      <h2 style={{ fontSize: '24px', margin: '0 0 16px 0', letterSpacing: '1px' }}>SIMULATION LEGACY</h2>
      
      <div style={{ margin: '24px 0' }}>
        <div style={{ fontSize: '14px', color: 'var(--text-muted)' }}>FINAL RARITY</div>
        <div style={{
          fontSize: '32px',
          fontWeight: '900',
          color: rarity === 'Legendary' ? 'var(--accent-amber)' : rarity === 'Epic' ? 'var(--accent-purple)' : rarity === 'Rare' ? 'var(--accent-cyan)' : 'var(--text-muted)',
          margin: '8px 0',
          textShadow: rarity === 'Legendary' ? '0 0 16px var(--accent-amber-30)' : 'none'
        }}>
          {rarity}
        </div>
      </div>

      <div style={{ margin: '16px 0', fontSize: '16px' }}>
        <span style={{ color: 'var(--text-muted)' }}>Survival Generations: </span>
        <strong style={{ fontSize: '20px' }}>{generation}</strong>
      </div>

      <div style={{ marginTop: '32px' }}>
        <button style={saveButtonStyle} onClick={onSave} aria-label="Save">
          SAVE SPECIMEN
        </button>
        <button style={buttonStyle} onClick={onClose} aria-label="Close">
          CLOSE
        </button>
      </div>
    </div>
  );
}

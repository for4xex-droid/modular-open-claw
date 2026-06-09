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
    background: 'rgba(15, 15, 25, 0.95)',
    backdropFilter: 'blur(20px)',
    border: '1px solid rgba(255, 255, 255, 0.12)',
    borderRadius: '16px',
    padding: '32px',
    color: '#fff',
    width: '400px',
    textAlign: 'center',
    boxShadow: '0 24px 64px rgba(0, 0, 0, 0.8)',
    fontFamily: 'system-ui, sans-serif',
    zIndex: 1000
  };

  const buttonStyle: React.CSSProperties = {
    background: 'rgba(255, 255, 255, 0.08)',
    border: '1px solid rgba(255, 255, 255, 0.1)',
    borderRadius: '8px',
    color: '#fff',
    padding: '10px 20px',
    cursor: 'pointer',
    fontWeight: 'bold',
    margin: '8px',
    transition: 'all 0.2s'
  };

  const saveButtonStyle: React.CSSProperties = {
    ...buttonStyle,
    background: 'linear-gradient(135deg, #ffd700, #b8860b)',
    color: '#000',
    border: 'none'
  };

  return (
    <div style={containerStyle}>
      <h2 style={{ fontSize: '24px', margin: '0 0 16px 0', letterSpacing: '1px' }}>SIMULATION LEGACY</h2>
      
      <div style={{ margin: '24px 0' }}>
        <div style={{ fontSize: '14px', color: '#889' }}>FINAL RARITY</div>
        <div style={{
          fontSize: '32px',
          fontWeight: '900',
          color: rarity === 'Legendary' ? '#ffd700' : rarity === 'Epic' ? '#c084fc' : rarity === 'Rare' ? '#60a5fa' : '#a1a1aa',
          margin: '8px 0',
          textShadow: rarity === 'Legendary' ? '0 0 16px rgba(255,215,0,0.4)' : 'none'
        }}>
          {rarity}
        </div>
      </div>

      <div style={{ margin: '16px 0', fontSize: '16px' }}>
        <span style={{ color: '#889' }}>Survival Generations: </span>
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

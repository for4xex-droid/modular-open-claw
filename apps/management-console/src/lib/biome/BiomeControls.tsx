import React from 'react';

export interface BiomeControlsProps {
  onInjectElement: (element: string) => void;
  onTriggerCrisis: (crisis: string) => void;
  onRewind: () => void;
  paused: boolean;
  onTogglePause: () => void;
}

export function BiomeControls({
  onInjectElement,
  onTriggerCrisis,
  onRewind,
  paused,
  onTogglePause
}: BiomeControlsProps) {
  const elements = ['C', 'N', 'P', 'H'];
  const crises = ['Meteor', 'IceAge'];

  const buttonStyle: React.CSSProperties = {
    background: 'var(--white-05)',
    border: '1px solid var(--border-glass)',
    borderRadius: '8px',
    color: 'var(--white-100)',
    padding: '8px 16px',
    cursor: 'pointer',
    fontSize: '14px',
    fontWeight: '500',
    transition: 'all 0.2s',
  };

  const primaryButtonStyle: React.CSSProperties = {
    ...buttonStyle,
    background: 'var(--accent-cyan-20)',
    border: '1px solid var(--accent-cyan-30)',
    color: 'var(--accent-cyan)',
  };

  const dangerButtonStyle: React.CSSProperties = {
    ...buttonStyle,
    background: 'var(--accent-rose-15)',
    border: '1px solid var(--accent-rose-30)',
    color: 'var(--accent-rose)',
  };

  return (
    <div style={{
      padding: '16px',
      background: 'var(--bg-glass-heavy)',
      backdropFilter: 'blur(12px)',
      border: '1px solid var(--border-glass)',
      borderRadius: '12px',
      color: 'var(--white-100)',
      display: 'flex',
      flexDirection: 'column',
      gap: '16px',
      fontFamily: 'system-ui, sans-serif'
    }}>
      {/* 元素注入 */}
      <div>
        <span style={{ fontSize: '12px', color: 'var(--text-muted)', display: 'block', marginBottom: '8px' }}>INJECT ELEMENTS</span>
        <div style={{ display: 'flex', gap: '8px' }}>
          {elements.map((el) => (
            <button
              key={el}
              style={primaryButtonStyle}
              onClick={() => onInjectElement(el)}
              aria-label={el}
            >
              {el}
            </button>
          ))}
        </div>
      </div>

      {/* 災害発生 */}
      <div>
        <span style={{ fontSize: '12px', color: 'var(--text-muted)', display: 'block', marginBottom: '8px' }}>TRIGGER CRISIS</span>
        <div style={{ display: 'flex', gap: '8px' }}>
          {crises.map((cr) => (
            <button
              key={cr}
              style={dangerButtonStyle}
              onClick={() => onTriggerCrisis(cr)}
              aria-label={cr}
            >
              {cr}
            </button>
          ))}
        </div>
      </div>

      {/* システム制御 */}
      <div style={{ borderTop: '1px solid var(--border-glass)', paddingTop: '12px', display: 'flex', gap: '8px' }}>
        <button style={buttonStyle} onClick={onTogglePause} aria-label={paused ? 'Resume' : 'Pause'}>
          {paused ? 'Resume' : 'Pause'}
        </button>
        <button style={buttonStyle} onClick={onRewind} aria-label="Rewind">
          Rewind
        </button>
      </div>
    </div>
  );
}

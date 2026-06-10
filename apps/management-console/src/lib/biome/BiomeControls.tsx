import React from 'react';

export interface BiomeControlsProps {
  onInjectElement: (element: string) => void;
  onTriggerCrisis: (crisis: string) => void;
  onRewind: () => void;
  paused: boolean;
  onTogglePause: () => void;
  onNewSeed?: () => void;
}

export function BiomeControls({
  onInjectElement,
  onTriggerCrisis,
  onRewind,
  paused,
  onTogglePause,
  onNewSeed
}: BiomeControlsProps) {
  const elements = ['C', 'N', 'P', 'H'];
  const crises = ['Meteor', 'IceAge'];


  const buttonStyle: React.CSSProperties = {
    background: 'var(--white-05)',
    border: '1px solid var(--border-glass)',
    borderRadius: 'var(--radius-sm)',
    color: 'var(--white-100)',
    padding: 'var(--space-xs) var(--space-sm)',
    cursor: 'pointer',
    fontSize: '0.875rem',
    fontWeight: '500',
    transition: 'all var(--speed-fast)',
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
      padding: 'var(--space-sm)',
      background: 'var(--bg-glass-heavy)',
      backdropFilter: 'blur(var(--blur-md))',
      border: '1px solid var(--border-glass)',
      borderRadius: 'var(--radius-md)',
      color: 'var(--white-100)',
      display: 'flex',
      flexDirection: 'column',
      gap: 'var(--space-sm)',
      fontFamily: 'var(--font-main)'
    }}>
      {/* 元素注入 */}
      <div>
        <span style={{ fontSize: '0.75rem', color: 'var(--text-muted)', display: 'block', marginBottom: 'var(--space-xs)' }}>INJECT ELEMENTS</span>
        <div style={{ display: 'flex', gap: 'var(--space-xs)' }}>
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
        <span style={{ fontSize: '0.75rem', color: 'var(--text-muted)', display: 'block', marginBottom: 'var(--space-xs)' }}>TRIGGER CRISIS</span>
        <div style={{ display: 'flex', gap: 'var(--space-xs)' }}>
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
      <div style={{ borderTop: '1px solid var(--border-glass)', paddingTop: 'var(--space-sm)', display: 'flex', gap: 'var(--space-xs)' }}>
        <button style={buttonStyle} onClick={onTogglePause} aria-label={paused ? 'Resume' : 'Pause'}>
          {paused ? 'Resume' : 'Pause'}
        </button>
        <button style={buttonStyle} onClick={onRewind} aria-label="Rewind">
          Rewind
        </button>
        {onNewSeed && (
          <button style={buttonStyle} onClick={onNewSeed} aria-label="New Seed">
            New Seed
          </button>
        )}
      </div>

    </div>
  );
}


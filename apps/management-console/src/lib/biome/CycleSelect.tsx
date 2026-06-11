/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */



interface CycleSelectProps {
  speed: number;
  onSpeedChange: (speed: number) => void;
  paused: boolean;
  onTogglePause: () => void;
}

export function CycleSelect({ speed, onSpeedChange, paused, onTogglePause }: CycleSelectProps) {
  return (
    <div style={{
      display: 'flex',
      alignItems: 'center',
      gap: '0.5rem',
      padding: '0.5rem',
      background: 'var(--black-20)',
      borderRadius: 'var(--radius-sm)',
      border: '1px solid var(--white-05)'
    }}>
      <button
        onClick={onTogglePause}
        style={{
          background: paused ? 'var(--accent-cyan)' : 'var(--white-10)',
          color: paused ? 'var(--bg-primary)' : 'var(--white-100)',
          border: 'none',
          padding: '0.4rem 0.8rem',
          borderRadius: '4px',
          cursor: 'pointer',
          fontWeight: 'bold',
          transition: 'all 0.2s'
        }}
      >
        {paused ? 'Resume' : 'Pause'}
      </button>
      <div style={{ display: 'flex', gap: '0.25rem' }}>
        {[100, 50, 20, 10].map(s => {
          const label = s === 100 ? '1x' : s === 50 ? '2x' : s === 20 ? '5x' : '10x';
          const isActive = speed === s;
          return (
            <button
              key={s}
              onClick={() => onSpeedChange(s)}
              style={{
                background: isActive ? 'var(--accent-purple)' : 'var(--white-05)',
                color: 'var(--white-100)',
                border: 'none',
                padding: '0.4rem 0.6rem',
                borderRadius: '4px',
                cursor: 'pointer',
                fontSize: '0.8rem',
                transition: 'all 0.2s'
              }}
            >
              {label}
            </button>
          );
        })}
      </div>
    </div>
  );
}

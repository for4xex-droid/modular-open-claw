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
  bloomEnabled?: boolean;
  onToggleBloom?: () => void;
}

export function CycleSelect({
  speed,
  onSpeedChange,
  paused,
  onTogglePause,
  bloomEnabled = true,
  onToggleBloom
}: CycleSelectProps) {
  return (
    <div style={{
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'space-between',
      gap: '0.4rem',
      padding: '0.5rem',
      background: 'var(--black-20)',
      borderRadius: 'var(--radius-sm)',
      border: '1px solid var(--white-05)',
      flexWrap: 'wrap'
    }}>
      <div style={{ display: 'flex', gap: '0.4rem', alignItems: 'center' }}>
        <button
          onClick={onTogglePause}
          aria-label={paused ? 'Resume' : 'Pause'}
          data-testid="cycle-pause"
          style={{
            background: paused ? 'var(--accent-cyan)' : 'var(--white-10)',
            color: paused ? 'var(--text-inverse)' : 'var(--white-100)',
            border: 'none',
            padding: '0.4rem 0.6rem',
            borderRadius: '4px',
            cursor: 'pointer',
            fontWeight: 'bold',
            fontSize: '0.8rem',
            transition: 'all 0.2s'
          }}
        >
          {paused ? '▶ 再開' : '⏸ 停止'}
        </button>
        <div style={{ display: 'flex', gap: '0.25rem' }}>
          {[100, 50, 20, 10].map(s => {
            const label = s === 100 ? '1x' : s === 50 ? '2x' : s === 20 ? '5x' : '10x';
            const isActive = speed === s;
            return (
              <button
                key={s}
                onClick={() => onSpeedChange(s)}
                data-testid={`speed-${label}`}
                style={{
                  background: isActive ? 'var(--accent-purple)' : 'var(--white-05)',
                  color: 'var(--white-100)',
                  border: 'none',
                  padding: '0.4rem 0.5rem',
                  borderRadius: '4px',
                  cursor: 'pointer',
                  fontSize: '0.75rem',
                  transition: 'all 0.2s'
                }}
              >
                {label}
              </button>
            );
          })}
        </div>
      </div>

      {onToggleBloom && (
        <button
          onClick={onToggleBloom}
          aria-label="Toggle Bloom"
          data-testid="toggle-bloom"
          style={{
            background: bloomEnabled ? 'var(--accent-cyan-20)' : 'var(--white-05)',
            color: bloomEnabled ? 'var(--accent-cyan)' : 'var(--white-60)',
            border: bloomEnabled ? '1px solid var(--accent-cyan-30)' : '1px solid transparent',
            padding: '0.4rem 0.6rem',
            borderRadius: '4px',
            cursor: 'pointer',
            fontSize: '0.75rem',
            fontWeight: '600',
            transition: 'all 0.2s',
            boxShadow: bloomEnabled ? '0 0 8px var(--accent-cyan-10)' : 'none'
          }}
        >
          ✨ グロウ
        </button>
      )}
    </div>
  );
}

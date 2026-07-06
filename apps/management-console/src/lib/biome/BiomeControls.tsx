/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import React from 'react';

export interface BiomeControlsProps {
  seedMode: boolean;
  onToggleSeedMode: () => void;
  leniaMu: number;
  leniaSigma: number;
  onLeniaMuChange: (v: number) => void;
  onLeniaSigmaChange: (v: number) => void;
  onShowCatalog: () => void;
  onRewind: () => void;
  paused: boolean;
  onTogglePause: () => void;
  onNewSeed?: () => void;
  onShowTutorial?: () => void;
}

export function BiomeControls({
  seedMode,
  onToggleSeedMode,
  leniaMu,
  leniaSigma,
  onLeniaMuChange,
  onLeniaSigmaChange,
  onShowCatalog,
  onRewind,
  paused,
  onTogglePause,
  onNewSeed,
  onShowTutorial,
}: BiomeControlsProps) {
  const buttonStyle: React.CSSProperties = {
    background: 'var(--white-05)',
    border: '1px solid var(--border-glass)',
    borderRadius: 'var(--radius-sm)',
    color: 'var(--white-100)',
    padding: 'var(--space-xs) var(--space-sm)',
    cursor: 'pointer',
    fontSize: '0.8125rem',
    fontWeight: '600',
    transition: 'all var(--speed-fast)',
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
    gap: '4px',
  };

  const primaryButtonStyle: React.CSSProperties = {
    ...buttonStyle,
    background: 'var(--accent-cyan-20)',
    border: '1px solid var(--accent-cyan-30)',
    color: 'var(--accent-cyan)',
  };

  const sliderLabelStyle: React.CSSProperties = {
    fontSize: '0.75rem',
    color: 'var(--text-muted)',
    display: 'block',
    marginBottom: '4px',
    fontWeight: '600',
  };

  return (
    <div
      style={{
        padding: 'var(--space-sm)',
        background: 'var(--bg-glass-heavy)',
        backdropFilter: 'blur(var(--blur-md))',
        border: '1px solid var(--border-glass)',
        borderRadius: 'var(--radius-md)',
        color: 'var(--white-100)',
        display: 'flex',
        flexDirection: 'column',
        gap: 'var(--space-sm)',
        fontFamily: 'var(--font-main)',
      }}
    >
      {/* 種まきモード */}
      <div>
        <span style={sliderLabelStyle}>操作: 種まき</span>
        <button
          style={{
            ...primaryButtonStyle,
            width: '100%',
            background: seedMode ? 'var(--accent-cyan)' : 'var(--accent-cyan-20)',
            color: seedMode ? 'var(--text-inverse)' : 'var(--accent-cyan)',
            boxShadow: seedMode ? '0 0 12px var(--accent-cyan)' : 'none',
          }}
          onClick={onToggleSeedMode}
          aria-label="Seed Mode"
          data-testid="control-seed-mode"
        >
          🌱 {seedMode ? '種まき ON — 画面をタッチ' : '種まき OFF'}
        </button>
      </div>

      {/* μ / σ スライダー */}
      <div>
        <span style={sliderLabelStyle}>成長パラメータ μ</span>
        <input
          type="range"
          min={0.05}
          max={0.35}
          step={0.001}
          value={leniaMu}
          onChange={(e) => onLeniaMuChange(parseFloat(e.target.value))}
          data-testid="control-lenia-mu"
          style={{ width: '100%' }}
        />
        <span style={{ fontSize: '0.7rem', color: 'var(--white-60)' }}>{leniaMu.toFixed(3)}</span>
      </div>

      <div>
        <span style={sliderLabelStyle}>成長パラメータ σ</span>
        <input
          type="range"
          min={0.005}
          max={0.05}
          step={0.0005}
          value={leniaSigma}
          onChange={(e) => onLeniaSigmaChange(parseFloat(e.target.value))}
          data-testid="control-lenia-sigma"
          style={{ width: '100%' }}
        />
        <span style={{ fontSize: '0.7rem', color: 'var(--white-60)' }}>{leniaSigma.toFixed(4)}</span>
      </div>

      {/* 図鑑 */}
      <div>
        <button
          style={{ ...buttonStyle, width: '100%', background: 'var(--accent-purple-20)', color: 'var(--accent-purple)', border: '1px solid var(--accent-purple-30)' }}
          onClick={onShowCatalog}
          aria-label="Catalog"
          data-testid="control-catalog"
        >
          📖 種図鑑
        </button>
      </div>

      {/* システム制御 */}
      <div style={{ borderTop: '1px solid var(--border-glass)', paddingTop: 'var(--space-sm)', display: 'flex', flexWrap: 'wrap', gap: 'var(--space-xs)' }}>
        <button
          style={{ ...buttonStyle, flex: '1 1 45%' }}
          onClick={onTogglePause}
          aria-label={paused ? 'Resume' : 'Pause'}
          data-testid="control-pause"
        >
          {paused ? '▶ 再開' : '⏸ 停止'}
        </button>
        <button
          style={{ ...buttonStyle, flex: '1 1 45%' }}
          onClick={onRewind}
          aria-label="Rewind"
          data-testid="control-rewind"
        >
          ⏪ 巻き戻し (20世代)
        </button>
        {onNewSeed && (
          <button
            style={{ ...buttonStyle, flex: '1 1 45%' }}
            onClick={onNewSeed}
            aria-label="New Seed"
            data-testid="control-newseed"
          >
            🔄 新シード
          </button>
        )}
        {onShowTutorial && (
          <button
            style={{ ...buttonStyle, flex: '1 1 45%', background: 'var(--accent-amber-15)', color: 'var(--accent-amber)', border: '1px solid var(--accent-amber-30)' }}
            onClick={onShowTutorial}
            aria-label="Tutorial"
            data-testid="control-tutorial"
          >
            ❓ 遊び方
          </button>
        )}
      </div>
    </div>
  );
}

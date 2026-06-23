import React from 'react';

export interface BiomeControlsProps {
  selectedElement: string | null;
  onSelectElement: (element: string | null) => void;
  selectedCrisis: string | null;
  onSelectCrisis: (crisis: string | null) => void;
  onInjectElement: (element: string) => void;
  onTriggerCrisis: (crisis: string) => void;
  onRollSubstance: () => void;
  onRewind: () => void;
  paused: boolean;
  onTogglePause: () => void;
  onNewSeed?: () => void;
  onShowTutorial?: () => void;
}

export function BiomeControls({
  selectedElement,
  onSelectElement,
  selectedCrisis,
  onSelectCrisis,
  onInjectElement: _onInjectElement,
  onTriggerCrisis: _onTriggerCrisis,
  onRollSubstance,
  onRewind,
  paused,
  onTogglePause,
  onNewSeed,
  onShowTutorial
}: BiomeControlsProps) {
  const elements = [
    { name: 'C',  label: '炭素' },
    { name: 'N',  label: '窒素' },
    { name: 'P',  label: 'リン' },
    { name: 'H',  label: '水素' },
    { name: 'O',  label: '酸素' },
    { name: 'S',  label: '硫黄' },
    { name: 'Fe', label: '鉄' },
    { name: 'Si', label: 'ケイ素' },
  ];
  const crises = ['Meteor', 'IceAge'];

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
    gap: '4px'
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

  const getElementStyle = (el: string): React.CSSProperties => {
    const active = selectedElement === el;
    return {
      ...primaryButtonStyle,
      background: active ? 'var(--accent-cyan, #00f0ff)' : 'var(--accent-cyan-20)',
      color: active ? '#0c0f1d' : 'var(--accent-cyan)',
      boxShadow: active ? '0 0 12px var(--accent-cyan)' : 'none',
      border: active ? '1px solid #fff' : primaryButtonStyle.border,
      transform: active ? 'scale(1.08)' : 'none',
    };
  };

  const getCrisisStyle = (cr: string): React.CSSProperties => {
    const active = selectedCrisis === cr;
    return {
      ...dangerButtonStyle,
      background: active ? 'var(--accent-rose, #ff4d6d)' : 'var(--accent-rose-15)',
      color: active ? '#0c0f1d' : 'var(--accent-rose)',
      boxShadow: active ? '0 0 12px var(--accent-rose)' : 'none',
      border: active ? '1px solid #fff' : dangerButtonStyle.border,
      transform: active ? 'scale(1.08)' : 'none',
    };
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
        <span style={{ fontSize: '0.75rem', color: 'var(--text-muted)', display: 'block', marginBottom: 'var(--space-xs)', fontWeight: '600' }}>
          元素注入 (クリックして選択し、画面をタッチ)
        </span>
        <div style={{ display: 'grid', gridTemplateColumns: 'repeat(4, 1fr)', gap: 'var(--space-xs)' }}>
          {elements.map((el) => (
            <button
              key={el.name}
              style={getElementStyle(el.name)}
              onClick={() => onSelectElement(selectedElement === el.name ? null : el.name)}
              aria-label={el.name}
              data-testid={`inject-${el.name.toLowerCase()}`}
              title={el.label}
            >
              {el.name}
            </button>
          ))}
        </div>
      </div>

      {/* 災害発生 */}
      <div>
        <span style={{ fontSize: '0.75rem', color: 'var(--text-muted)', display: 'block', marginBottom: 'var(--space-xs)', fontWeight: '600' }}>
          環境災害 (クリックして選択し、標的をタッチ)
        </span>
        <div style={{ display: 'flex', gap: 'var(--space-xs)' }}>
          {crises.map((cr) => (
            <button
              key={cr}
              style={getCrisisStyle(cr)}
              onClick={() => onSelectCrisis(selectedCrisis === cr ? null : cr)}
              aria-label={cr}
              data-testid={`crisis-${cr.toLowerCase()}`}
            >
              {cr === 'Meteor' ? '☄️ 隕石落下' : '❄️ 氷河期'}
            </button>
          ))}
        </div>
      </div>

      {/* 特殊物質合成 */}
      <div>
        <span style={{ fontSize: '0.75rem', color: 'var(--text-muted)', display: 'block', marginBottom: 'var(--space-xs)', fontWeight: '600' }}>
          研究ラボ
        </span>
        <div style={{ display: 'flex', gap: 'var(--space-xs)' }}>
          <button
            style={{ ...buttonStyle, background: 'var(--accent-purple-20)', color: 'var(--accent-purple, #d946ef)', border: '1px solid var(--accent-purple-30)', flex: 1 }}
            onClick={onRollSubstance}
            aria-label="Roll Substance"
            data-testid="control-random"
          >
            🎲 物質合成 (ランダム注入)
          </button>
        </div>
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
            style={{ ...buttonStyle, flex: '1 1 45%', background: 'var(--accent-amber-15)', color: 'var(--accent-amber, #f59e0b)', border: '1px solid var(--accent-amber-30)' }} 
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


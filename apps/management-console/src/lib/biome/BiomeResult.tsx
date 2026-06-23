import React from 'react';
import { ELEMENT_COLORS, MORPH_COLORS, getPercentageMap } from './utils/biomeHelpers';

export interface BiomeResultProps {
  generation: number;
  rarity: string;
  onSave: () => void;
  onClose: () => void;
  elementBalance?: Record<string, number>;
  morphologyDistribution?: Record<string, number>;
  discoveredReactions?: string[];
  activeCellCount?: number;
}

export function BiomeResult({ 
  generation, 
  rarity, 
  onSave, 
  onClose,
  elementBalance,
  morphologyDistribution,
  discoveredReactions,
  activeCellCount
}: BiomeResultProps) {
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
    width: '420px',
    maxHeight: '85vh',
    overflowY: 'auto',
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
    color: 'var(--text-inverse, #0c0f1d)',
    border: 'none'
  };

  const elements = getPercentageMap(elementBalance);
  const morphs = getPercentageMap(morphologyDistribution);

  return (
    <div style={containerStyle}>
      <h2 style={{ fontSize: '24px', margin: '0 0 16px 0', letterSpacing: '1px' }}>🏆 シミュレーション完了</h2>
      
      <div style={{ margin: '24px 0' }}>
        <div style={{ fontSize: '14px', color: 'var(--text-muted)' }}>最終評価ランク</div>
        <div 
          style={{
            fontSize: '32px',
            fontWeight: '900',
            color: rarity === 'Legendary' ? 'var(--accent-amber)' : rarity === 'Epic' ? 'var(--accent-purple)' : rarity === 'Rare' ? 'var(--accent-cyan)' : 'var(--text-muted)',
            margin: '8px 0',
            textShadow: rarity === 'Legendary' ? '0 0 16px var(--accent-amber-30)' : 'none'
          }}
          data-testid="result-rarity"
        >
          {rarity}
        </div>
      </div>

      <div style={{ margin: '16px 0', fontSize: '16px', display: 'flex', flexDirection: 'column', gap: '8px', alignItems: 'center' }}>
        <div>
          <span style={{ color: 'var(--text-muted)' }}>生存世代数: </span>
          <strong style={{ fontSize: '20px' }} data-testid="result-generation">{generation}</strong>
        </div>
        {activeCellCount !== undefined && (
          <div style={{ fontSize: '14px', fontWeight: '600' }}>
            活性セル数: {activeCellCount}
          </div>
        )}
      </div>

      {/* 詳細データ表示 */}
      {(elements.length > 0 || morphs.length > 0 || (discoveredReactions && discoveredReactions.length > 0)) && (
        <div style={{
          borderTop: '1px solid var(--white-08)',
          marginTop: '20px',
          paddingTop: '20px',
          display: 'flex',
          flexDirection: 'column',
          gap: '16px',
          textAlign: 'left',
          fontSize: '12px'
        }}>
          {elements.length > 0 && (
            <div>
              <div style={{ fontWeight: 'bold', marginBottom: '6px', color: 'var(--white-70)' }}>元素比率</div>
              <div style={{ display: 'flex', flexDirection: 'column', gap: '4px' }}>
                {elements.map(({ key, pct }) => (
                  <div key={key} style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
                    <span style={{ width: '20px', fontWeight: 'bold' }}>{key}</span>
                    <div style={{ flex: 1, height: '6px', background: 'var(--white-05)', borderRadius: '3px', overflow: 'hidden' }}>
                      <div style={{ height: '100%', width: `${pct}%`, background: ELEMENT_COLORS[key] || 'var(--white-40)' }} />
                    </div>
                    <span style={{ width: '45px', textAlign: 'right', color: 'var(--white-60)' }}>{pct.toFixed(1)}%</span>
                  </div>
                ))}
              </div>
            </div>
          )}

          {morphs.length > 0 && (
            <div>
              <div style={{ fontWeight: 'bold', marginBottom: '6px', color: 'var(--white-70)' }}>形態分布</div>
              <div style={{ display: 'flex', flexDirection: 'column', gap: '4px' }}>
                {morphs.map(({ key, pct }) => (
                  <div key={key} style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
                    <span style={{ width: '80px', textOverflow: 'ellipsis', overflow: 'hidden', whiteSpace: 'nowrap' }}>{key}</span>
                    <div style={{ flex: 1, height: '6px', background: 'var(--white-05)', borderRadius: '3px', overflow: 'hidden' }}>
                      <div style={{ height: '100%', width: `${pct}%`, background: MORPH_COLORS[key] || 'var(--white-40)' }} />
                    </div>
                    <span style={{ width: '45px', textAlign: 'right', color: 'var(--white-60)' }}>{pct.toFixed(1)}%</span>
                  </div>
                ))}
              </div>
            </div>
          )}

          {discoveredReactions && discoveredReactions.length > 0 && (
            <div>
              <div style={{ fontWeight: 'bold', marginBottom: '6px', color: 'var(--white-70)' }}>発見した反応</div>
              <div style={{ display: 'flex', flexWrap: 'wrap', gap: '6px' }}>
                {discoveredReactions.map((rx, idx) => (
                  <span 
                    key={idx} 
                    style={{
                      background: 'var(--white-05)',
                      border: '1px solid var(--white-10)',
                      borderRadius: '4px',
                      padding: '2px 6px',
                      fontSize: '11px',
                      color: 'var(--accent-cyan)'
                    }}
                  >
                    {rx}
                  </span>
                ))}
              </div>
            </div>
          )}
        </div>
      )}

      <div style={{ marginTop: '32px' }}>
        <button style={saveButtonStyle} onClick={onSave} aria-label="Save" data-testid="result-save">
          💾 標本を保存
        </button>
        <button style={buttonStyle} onClick={onClose} aria-label="Close" data-testid="result-close">
          ✕ 閉じる
        </button>
      </div>
    </div>
  );
}

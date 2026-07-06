/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import type { CSSProperties } from 'react';
import { useTranslation } from '../../i18n';
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
  symmetryScore?: number;
  complexityScore?: number;
  prismaticCells?: number;
}

export function BiomeResult({ 
  generation, 
  rarity, 
  onSave, 
  onClose,
  elementBalance,
  morphologyDistribution,
  discoveredReactions,
  activeCellCount,
  symmetryScore,
  complexityScore,
  prismaticCells,
}: BiomeResultProps) {
  const { t } = useTranslation();
  const containerStyle: CSSProperties = {
    position: 'fixed',
    top: '50%',
    left: '50%',
    transform: 'translate(-50%, -50%)',
    background: 'var(--bg-deep-glass)',
    backdropFilter: 'blur(20px)',
    border: '1px solid var(--border-glass-bright)',
    borderRadius: '16px',
    padding: 'var(--space-lg)',
    color: 'var(--white-100)',
    width: '420px',
    maxHeight: '85vh',
    overflowY: 'auto',
    textAlign: 'center',
    boxShadow: 'var(--shadow-deep)',
    fontFamily: 'var(--font-main)',
    zIndex: 1000
  };

  const buttonStyle: CSSProperties = {
    background: 'var(--white-08)',
    border: '1px solid var(--border-glass)',
    borderRadius: 'var(--radius-sm)',
    color: 'var(--white-100)',
    padding: '10px 20px',
    cursor: 'pointer',
    fontWeight: 'bold',
    margin: 'var(--space-xs)',
    transition: 'all 0.2s'
  };

  const saveButtonStyle: CSSProperties = {
    ...buttonStyle,
    background: 'linear-gradient(135deg, var(--accent-amber), var(--accent-amber-30))',
    color: 'var(--text-inverse)',
    border: 'none'
  };

  const elements = getPercentageMap(elementBalance);
  const morphs = getPercentageMap(morphologyDistribution);

  return (
    <div style={containerStyle}>
      <h2 style={{ fontSize: 'var(--font-size-2xl)', margin: '0 0 16px 0', letterSpacing: '1px' }}>🏆 シミュレーション完了</h2>
      
      <div style={{ margin: '24px 0' }}>
        <div style={{ fontSize: 'var(--font-size-base)', color: 'var(--text-muted)' }}>{t('biomeConsole.finalRarity')}</div>
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

      <div style={{ margin: '16px 0', fontSize: 'var(--font-size-md)', display: 'flex', flexDirection: 'column', gap: 'var(--space-xs)', alignItems: 'center' }}>
        <div>
          <span style={{ color: 'var(--text-muted)' }}>{t('biomeConsole.survivalGenerations')} </span>
          <strong style={{ fontSize: '20px' }} data-testid="result-generation">{generation}</strong>
        </div>
        {activeCellCount !== undefined && (
          <div style={{ fontSize: 'var(--font-size-base)', fontWeight: '600' }}>
            {t('biomeConsole.activeCells')} {activeCellCount}
          </div>
        )}
        {symmetryScore !== undefined && (
          <div style={{ fontSize: 'var(--font-size-base)' }}>
            {t('biome.result.symmetry')} {symmetryScore.toFixed(2)}
          </div>
        )}
        {complexityScore !== undefined && (
          <div style={{ fontSize: 'var(--font-size-base)' }}>
            {t('biome.result.complexity')} {complexityScore.toFixed(2)}
          </div>
        )}
        {prismaticCells !== undefined && (
          <div style={{ fontSize: 'var(--font-size-base)' }}>
            {t('biome.result.prismatic')} {prismaticCells}
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
          gap: 'var(--space-sm)',
          textAlign: 'left',
          fontSize: 'var(--font-size-sm)'
        }}>
          {elements.length > 0 && (
            <div>
              <div style={{ fontWeight: 'bold', marginBottom: '6px', color: 'var(--white-70)' }}>{t('biomeConsole.elementRatio')}</div>
              <div style={{ display: 'flex', flexDirection: 'column', gap: '4px' }}>
                {elements.map(({ key, pct }) => (
                  <div key={key} style={{ display: 'flex', alignItems: 'center', gap: 'var(--space-xs)' }}>
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
              <div style={{ fontWeight: 'bold', marginBottom: '6px', color: 'var(--white-70)' }}>{t('biomeConsole.morphologyDistribution')}</div>
              <div style={{ display: 'flex', flexDirection: 'column', gap: '4px' }}>
                {morphs.map(({ key, pct }) => (
                  <div key={key} style={{ display: 'flex', alignItems: 'center', gap: 'var(--space-xs)' }}>
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
              <div style={{ fontWeight: 'bold', marginBottom: '6px', color: 'var(--white-70)' }}>{t('biomeConsole.discoveredReactions')}</div>
              <div style={{ display: 'flex', flexWrap: 'wrap', gap: '6px' }}>
                {discoveredReactions.map((rx, idx) => (
                  <span 
                    key={idx} 
                    style={{
                      background: 'var(--white-05)',
                      border: '1px solid var(--white-10)',
                      borderRadius: '4px',
                      padding: '2px 6px',
                      fontSize: 'var(--font-size-xs)',
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

      <div style={{ marginTop: 'var(--space-lg)' }}>
        <button style={saveButtonStyle} onClick={onSave} aria-label="Save" data-testid="result-save">
          💾 標本を保存
        </button>
        <button style={buttonStyle} onClick={onClose} aria-label="Close" data-testid="result-close">
          ✕ {t('biomeConsole.close')}
        </button>
      </div>
    </div>
  );
}

/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */


import { useState } from 'react';
import { useTranslation } from '../../i18n';
import { ELEMENT_COLORS, MORPH_COLORS, getPercentageMap } from './utils/biomeHelpers';

export interface Specimen {
  id: string;
  name: string;
  generation: number;
  rarity: string;
  date: string;
  genome_data?: string;
  element_balance?: string;
  morphology_distribution?: string;
  discovered_reactions?: string;
  active_cell_count?: number;
}

function parseLeniaSpecies(genomeData?: string): {
  mu?: number;
  sigma?: number;
  species_hash?: number;
  mass?: number;
  locomotion?: number;
  longevity?: number;
} | null {
  if (!genomeData) return null;
  try {
    const parsed = JSON.parse(genomeData) as Record<string, unknown>;
    if (typeof parsed.mu === 'number' || typeof parsed.sigma === 'number') {
      return {
        mu: typeof parsed.mu === 'number' ? parsed.mu : undefined,
        sigma: typeof parsed.sigma === 'number' ? parsed.sigma : undefined,
        species_hash: typeof parsed.species_hash === 'number' ? parsed.species_hash : undefined,
        mass: typeof parsed.mass === 'number' ? parsed.mass : undefined,
        locomotion: typeof parsed.locomotion === 'number' ? parsed.locomotion : undefined,
        longevity: typeof parsed.longevity === 'number' ? parsed.longevity : undefined,
      };
    }
  } catch {
    return null;
  }
  return null;
}

export interface BiomeDendouProps {
  list: Specimen[];
  onLoad: (id: string) => void;
}

export function BiomeDendou({ list, onLoad }: BiomeDendouProps) {
  const { t } = useTranslation();
  const [expandedId, setExpandedId] = useState<string | null>(null);

  return (
    <div style={{
      padding: 'var(--space-sm)',
      background: 'var(--bg-glass-heavy)',
      backdropFilter: 'blur(12px)',
      border: '1px solid var(--border-glass)',
      borderRadius: 'var(--radius-md)',
      color: 'var(--white-100)',
      fontFamily: 'var(--font-main)'
    }}>
      <h3 style={{ margin: '0 0 16px 0', fontSize: 'var(--font-size-md)', fontWeight: 'bold' }}>{t('biomeConsole.dendouTitle')}</h3>
      {list.length === 0 ? (
        <div style={{ color: 'var(--text-muted)', fontSize: 'var(--font-size-base)', textAlign: 'center', padding: 'var(--space-sm)' }}>
          {t('biomeConsole.noSpecimens')}
        </div>
      ) : (
        <div style={{ display: 'flex', flexDirection: 'column', gap: 'var(--space-xs)' }}>
          {list.map((sp) => {
            const isExpanded = expandedId === sp.id;
            const elements = getPercentageMap(sp.element_balance);
            const morphs = getPercentageMap(sp.morphology_distribution);
            const leniaSpecies = parseLeniaSpecies(sp.genome_data);
            let reactions: string[] = [];
            try {
              if (sp.discovered_reactions) {
                reactions = JSON.parse(sp.discovered_reactions) as string[];
              }
            } catch (e) {
              console.warn('Failed to parse discovered_reactions:', e);
            }

            return (
              <div 
                key={sp.id} 
                style={{
                  display: 'flex',
                  flexDirection: 'column',
                  background: 'var(--white-02)',
                  border: '1px solid var(--white-04)',
                  borderRadius: 'var(--radius-sm)',
                  padding: '12px 16px',
                  gap: '12px'
                }}
                data-testid="dendou-specimen"
              >
                <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                  <div>
                    <div style={{ fontWeight: '600', fontSize: 'var(--font-size-base)' }}>{sp.name}</div>
                    <div style={{ fontSize: 'var(--font-size-xs)', color: 'var(--text-muted)', marginTop: '4px' }}>
                      {t('biomeConsole.generationLabel')} {sp.generation} | {t('biomeConsole.rarityLabel')} {sp.rarity} | {t('biomeConsole.dateLabel')} {sp.date}
                    </div>
                  </div>
                  <div style={{ display: 'flex', gap: 'var(--space-xs)' }}>
                    <button
                      style={{
                        background: 'var(--white-05)',
                        border: '1px solid var(--white-10)',
                        borderRadius: '6px',
                        color: 'var(--white-80)',
                        padding: '6px 12px',
                        cursor: 'pointer',
                        fontSize: 'var(--font-size-sm)',
                        fontWeight: '600',
                        transition: 'all 0.2s'
                      }}
                      onClick={() => setExpandedId(isExpanded ? null : sp.id)}
                    >
                      {isExpanded ? '▲ ' + t('biomeConsole.close') : '🔍 ' + t('biomeConsole.detail')}
                    </button>
                    <button
                      style={{
                        background: 'var(--accent-cyan-15)',
                        border: '1px solid var(--accent-cyan-30)',
                        borderRadius: '6px',
                        color: 'var(--accent-cyan)',
                        padding: '6px 12px',
                        cursor: 'pointer',
                        fontSize: 'var(--font-size-sm)',
                        fontWeight: '600',
                        transition: 'all 0.2s'
                      }}
                      onClick={() => onLoad(sp.id)}
                      aria-label="Load"
                      data-testid="dendou-load"
                    >
                      {t('biomeConsole.load')}
                    </button>
                  </div>
                </div>

                {isExpanded && (
                  <div style={{
                    borderTop: '1px solid var(--white-08)',
                    paddingTop: '12px',
                    fontSize: 'var(--font-size-sm)',
                    display: 'flex',
                    flexDirection: 'column',
                    gap: '12px'
                  }}>
                    {sp.active_cell_count !== undefined && (
                      <div style={{ fontWeight: 'bold', color: 'var(--white-90)' }}>
                        {t('biomeConsole.activeCells')} {sp.active_cell_count}
                      </div>
                    )}

                    {leniaSpecies && (
                      <div>
                        <div style={{ fontWeight: 'bold', marginBottom: '6px', color: 'var(--white-70)' }}>Lenia 種パラメータ</div>
                        <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '4px 12px', fontSize: '0.8rem' }}>
                          {leniaSpecies.mu !== undefined && (<><span>μ</span><span>{leniaSpecies.mu.toFixed(3)}</span></>)}
                          {leniaSpecies.sigma !== undefined && (<><span>σ</span><span>{leniaSpecies.sigma.toFixed(4)}</span></>)}
                          {leniaSpecies.mass !== undefined && (<><span>mass</span><span>{leniaSpecies.mass.toFixed(1)}</span></>)}
                          {leniaSpecies.longevity !== undefined && (<><span>longevity</span><span>{leniaSpecies.longevity} tick</span></>)}
                          {leniaSpecies.species_hash !== undefined && (
                            <><span>種ID</span><span>{String(leniaSpecies.species_hash).slice(0, 8)}…</span></>
                          )}
                        </div>
                      </div>
                    )}

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

                    {reactions.length > 0 && (
                      <div>
                        <div style={{ fontWeight: 'bold', marginBottom: '6px', color: 'var(--white-70)' }}>{t('biomeConsole.discoveredReactions')}</div>
                        <div style={{ display: 'flex', flexWrap: 'wrap', gap: '6px' }}>
                          {reactions.map((rx, idx) => (
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
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
}

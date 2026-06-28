/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */


import { useState } from 'react';
import { ELEMENT_COLORS, MORPH_COLORS, getPercentageMap } from './utils/biomeHelpers';

export interface Specimen {
  id: string;
  name: string;
  generation: number;
  rarity: string;
  date: string;
  element_balance?: string; // JSON String
  morphology_distribution?: string; // JSON String
  discovered_reactions?: string; // JSON String
  active_cell_count?: number;
}

export interface BiomeDendouProps {
  list: Specimen[];
  onLoad: (id: string) => void;
}

export function BiomeDendou({ list, onLoad }: BiomeDendouProps) {
  const [expandedId, setExpandedId] = useState<string | null>(null);

  return (
    <div style={{
      padding: '16px',
      background: 'var(--bg-glass-heavy)',
      backdropFilter: 'blur(12px)',
      border: '1px solid var(--border-glass)',
      borderRadius: '12px',
      color: 'var(--white-100)',
      fontFamily: 'system-ui, sans-serif'
    }}>
      <h3 style={{ margin: '0 0 16px 0', fontSize: '16px', fontWeight: 'bold' }}>🏛️ 殿堂入り標本</h3>
      {list.length === 0 ? (
        <div style={{ color: 'var(--text-muted)', fontSize: '14px', textAlign: 'center', padding: '16px' }}>
          保存された標本はまだありません。
        </div>
      ) : (
        <div style={{ display: 'flex', flexDirection: 'column', gap: '8px' }}>
          {list.map((sp) => {
            const isExpanded = expandedId === sp.id;
            const elements = getPercentageMap(sp.element_balance);
            const morphs = getPercentageMap(sp.morphology_distribution);
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
                  borderRadius: '8px',
                  padding: '12px 16px',
                  gap: '12px'
                }}
                data-testid="dendou-specimen"
              >
                <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                  <div>
                    <div style={{ fontWeight: '600', fontSize: '14px' }}>{sp.name}</div>
                    <div style={{ fontSize: '11px', color: 'var(--text-muted)', marginTop: '4px' }}>
                      世代: {sp.generation} | ランク: {sp.rarity} | 日付: {sp.date}
                    </div>
                  </div>
                  <div style={{ display: 'flex', gap: '8px' }}>
                    <button
                      style={{
                        background: 'var(--white-05)',
                        border: '1px solid var(--white-10)',
                        borderRadius: '6px',
                        color: 'var(--white-80)',
                        padding: '6px 12px',
                        cursor: 'pointer',
                        fontSize: '12px',
                        fontWeight: '600',
                        transition: 'all 0.2s'
                      }}
                      onClick={() => setExpandedId(isExpanded ? null : sp.id)}
                    >
                      {isExpanded ? '▲ 閉じる' : '🔍 詳細'}
                    </button>
                    <button
                      style={{
                        background: 'var(--accent-cyan-15)',
                        border: '1px solid var(--accent-cyan-30)',
                        borderRadius: '6px',
                        color: 'var(--accent-cyan)',
                        padding: '6px 12px',
                        cursor: 'pointer',
                        fontSize: '12px',
                        fontWeight: '600',
                        transition: 'all 0.2s'
                      }}
                      onClick={() => onLoad(sp.id)}
                      aria-label="Load"
                      data-testid="dendou-load"
                    >
                      📂 読込
                    </button>
                  </div>
                </div>

                {isExpanded && (
                  <div style={{
                    borderTop: '1px solid var(--white-08)',
                    paddingTop: '12px',
                    fontSize: '12px',
                    display: 'flex',
                    flexDirection: 'column',
                    gap: '12px'
                  }}>
                    {sp.active_cell_count !== undefined && (
                      <div style={{ fontWeight: 'bold', color: 'var(--white-90)' }}>
                        活性セル数: {sp.active_cell_count}
                      </div>
                    )}

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

                    {reactions.length > 0 && (
                      <div>
                        <div style={{ fontWeight: 'bold', marginBottom: '6px', color: 'var(--white-70)' }}>発見した反応</div>
                        <div style={{ display: 'flex', flexWrap: 'wrap', gap: '6px' }}>
                          {reactions.map((rx, idx) => (
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
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
}

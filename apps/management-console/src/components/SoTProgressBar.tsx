/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import React, { useMemo, useState, useEffect } from 'react';
import { useSystemVitality } from '../hooks/useSystemVitality';
import { SoTEvent } from '../types';
import { useTranslation } from '../i18n';

export const SoTProgressBar: React.FC = () => {
    const { t } = useTranslation();
    const { events } = useSystemVitality();
    
    const currentSession = useMemo(() => {
        let activeSession: {
            id: string;
            roles: string[];
            currentRound: number;
            status: 'active' | 'ended';
            scores: [string, number][];
            events: SoTEvent[];
            abstentionCount: number;
            protocol: string | null;
            outcome: string | null;
        } | null = null;
        
        const sotEvents = events
            .filter(e => e.type === 'sot_progress')
            .map(e => e.data as SoTEvent)
            .reverse();

        for (const se of sotEvents) {
            if (!se) continue;
            const { type, data } = se;
            switch(type) {
                case 'SessionStart':
                    if (!activeSession || activeSession.id !== data.session_id) {
                        activeSession = {
                            id: data.session_id,
                            roles: [],
                            currentRound: 0,
                            status: 'active',
                            scores: [],
                            events: [se],
                            abstentionCount: 0,
                            protocol: null,
                            outcome: null,
                        };
                    }
                    break;
                case 'ProtocolSelected':
                    if (activeSession && activeSession.id === data.session_id) {
                        activeSession.protocol = data.protocol;
                    }
                    break;
                case 'RoleStart':
                case 'RoleOutput':
                    if (!activeSession) {
                        activeSession = {
                            id: data.session_id,
                            roles: [],
                            currentRound: data.round,
                            status: 'active',
                            scores: [],
                            events: [],
                            abstentionCount: 0,
                            protocol: null,
                            outcome: null,
                        };
                    }
                    if (activeSession.id === data.session_id) {
                        activeSession.currentRound = data.round;
                        if (type === 'RoleStart' && !activeSession.roles.includes(data.role)) {
                            activeSession.roles.push(data.role);
                        }
                    }
                    break;
                case 'ThinkerAbstained':
                    if (activeSession && activeSession.id === data.session_id) {
                        activeSession.abstentionCount += 1;
                    }
                    break;
                case 'Score':
                    if (activeSession && activeSession.id === data.session_id) {
                        activeSession.scores = data.scores;
                    }
                    break;
                case 'SessionEnd':
                    if (activeSession && activeSession.id === data.session_id) {
                        activeSession.status = 'ended';
                        const rawOutcome = data.outcome;
                        activeSession.outcome = typeof rawOutcome === 'string' ? rawOutcome : 'Error';
                    }
                    break;
            }
        }
        
        return activeSession;
    }, [events]);

    // Auto-dismiss: hide the bar 5 seconds after session ends
    const [dismissed, setDismissed] = useState(false);
    useEffect(() => {
        if (currentSession?.status === 'ended') {
            setDismissed(false);
            const timer = setTimeout(() => setDismissed(true), 5000);
            return () => clearTimeout(timer);
        }
        setDismissed(false);
    }, [currentSession?.status, currentSession?.id]);

    if (!currentSession || dismissed) {
        return null;
    }

    return (
        <div style={{
            position: 'fixed',
            bottom: 'var(--space-xl)',
            left: '50%',
            transform: 'translateX(-50%)',
            width: '400px',
            background: 'var(--bg-glass-heavy)',
            backdropFilter: 'blur(12px)',
            border: '1px solid var(--border-glass-bright)',
            borderRadius: 'var(--radius-md)',
            padding: 'var(--space-sm)',
            boxShadow: 'var(--shadow-deep)',
            zIndex: 50,
            animation: `fadeIn var(--speed-normal) ease-out`,
            fontFamily: 'var(--font-main)',
            color: 'var(--text-primary)'
        }}>
            <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: 'var(--space-xs)' }}>
                <h3 style={{ margin: 0, fontSize: '0.85rem', fontWeight: 700, color: 'var(--accent-purple)', display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
                    <span style={{ position: 'relative', display: 'flex', height: '12px', width: '12px' }}>
                      <span className="ani-pulse" style={{ position: 'absolute', height: '100%', width: '100%', borderRadius: '50%', background: 'var(--accent-purple)', opacity: 0.7 }}></span>
                      <span style={{ position: 'relative', display: 'inline-flex', borderRadius: '50%', height: '12px', width: '12px', background: 'var(--accent-purple)' }}></span>
                    </span>
                    {t('sot.active')}
                </h3>
                <div style={{ display: 'flex', alignItems: 'center', gap: 'var(--space-xs)' }}>
                    {currentSession.protocol && (
                        <span style={{
                            fontSize: '0.65rem',
                            color: 'var(--accent-cyan)',
                            padding: '2px 6px',
                            background: 'var(--accent-cyan-10)',
                            border: '1px solid var(--accent-cyan-30)',
                            borderRadius: '4px'
                        }}>
                            {currentSession.protocol}
                        </span>
                    )}
                    {currentSession.abstentionCount > 0 && (
                        <span style={{
                            fontSize: '0.65rem',
                            color: 'var(--accent-amber)',
                            padding: '2px 6px',
                            background: 'var(--accent-amber-10)',
                            border: '1px solid var(--accent-amber-30)',
                            borderRadius: '4px'
                        }} data-tooltip={t('sot.abstentions')}>
                            🤚 {currentSession.abstentionCount}
                        </span>
                    )}
                    <span style={{
                        fontSize: '0.75rem',
                        color: 'var(--text-muted)',
                        padding: '2px 8px',
                        background: 'var(--bg-primary)',
                        borderRadius: '4px'
                    }}>
                        {t('sot.round')} {currentSession.currentRound}
                    </span>
                </div>
            </div>
            
            <div style={{ display: 'flex', flexDirection: 'column', gap: 'var(--space-xs)' }}>
                {currentSession.roles.map((role: string, idx: number) => {
                    const isFocus = idx === currentSession.roles.length - 1;
                    return (
                        <div key={role} style={{
                            fontSize: '0.75rem',
                            padding: 'var(--space-xs)',
                            borderRadius: 'var(--radius-sm)',
                            border: `1px solid ${isFocus ? 'var(--accent-purple-20)' : 'var(--white-05)'}`,
                            background: isFocus ? 'var(--accent-purple-10)' : 'var(--bg-primary)',
                            color: isFocus ? 'var(--accent-purple)' : 'var(--text-secondary)'
                        }}>
                            {isFocus ? (
                                <div style={{ display: 'flex', justifyContent: 'space-between' }}>
                                    <span>{role} {t('sot.thinking')}</span>
                                    <span className="ani-pulse">..</span>
                                </div>
                            ) : (
                                <span>{role} {t('sot.completed')}</span>
                            )}
                        </div>
                    );
                })}
            </div>
            
            {currentSession.scores.length > 0 && (
                <div style={{ marginTop: 'var(--space-sm)', paddingTop: 'var(--space-xs)', borderTop: '1px solid var(--border-glass)' }}>
                    <div style={{ fontSize: '0.65rem', color: 'var(--text-muted)', marginBottom: '4px', textTransform: 'uppercase', fontWeight: 600 }}>{t('sot.latestScores')}</div>
                    <div style={{ display: 'flex', gap: 'var(--space-xs)' }}>
                        {currentSession.scores.map(([metric, score]: [string, number]) => {
                            const color = score >= 4 ? 'var(--accent-emerald)' : score >= 3 ? 'var(--accent-amber)' : 'var(--accent-rose)';
                            return (
                                <div key={metric} style={{
                                    fontSize: '0.75rem',
                                    padding: '2px 6px',
                                    borderRadius: '4px',
                                    background: score >= 4 ? 'var(--accent-emerald-10)' : score >= 3 ? 'var(--accent-amber-10)' : 'var(--accent-rose-10)',
                                    color: color
                                }}>
                                    {metric}: {score}/5
                                </div>
                            );
                        })}
                    </div>
                </div>
            )}

            {currentSession.status === 'ended' && currentSession.outcome && (() => {
                const outcomeMap: Record<string, { icon: string; label: string; color: string; bg: string; border: string }> = {
                    'AllCriteriaPassed': { icon: '✓', label: t('sot.completed'), color: 'var(--accent-emerald)', bg: 'var(--accent-emerald-10)', border: 'var(--accent-emerald-30)' },
                    'ConvergedEarly': { icon: '✓', label: t('sot.convergedEarly'), color: 'var(--accent-emerald)', bg: 'var(--accent-emerald-10)', border: 'var(--accent-emerald-30)' },
                    'MaxRoundsReached': { icon: '⏱', label: t('sot.maxRoundsReached'), color: 'var(--accent-amber)', bg: 'var(--accent-amber-10)', border: 'var(--accent-amber-30)' },
                    'BudgetExhausted': { icon: '💰', label: t('sot.budgetExhausted'), color: 'var(--accent-amber)', bg: 'var(--accent-amber-10)', border: 'var(--accent-amber-30)' },
                    'Timeout': { icon: '⏳', label: t('sot.timeout'), color: 'var(--accent-amber)', bg: 'var(--accent-amber-10)', border: 'var(--accent-amber-30)' },
                    'SpectralDivergence': { icon: '⚠', label: t('sot.spectralDivergence'), color: 'var(--accent-rose)', bg: 'var(--accent-rose-10)', border: 'var(--accent-rose-30)' },
                };
                const cfg = outcomeMap[currentSession.outcome] ?? {
                    icon: '●', label: currentSession.outcome, color: 'var(--text-muted)', bg: 'var(--bg-primary)', border: 'var(--border-glass)',
                };
                return (
                    <div style={{
                        marginTop: 'var(--space-sm)',
                        paddingTop: 'var(--space-xs)',
                        borderTop: '1px solid var(--border-glass)',
                        display: 'flex',
                        alignItems: 'center',
                        gap: 'var(--space-xs)',
                    }}>
                        <span style={{
                            fontSize: '0.75rem',
                            padding: '3px 8px',
                            borderRadius: '4px',
                            background: cfg.bg,
                            color: cfg.color,
                            border: `1px solid ${cfg.border}`,
                            fontWeight: 600,
                        }}>{cfg.icon} {cfg.label}</span>
                    </div>
                );
            })()}
        </div>
    );
};

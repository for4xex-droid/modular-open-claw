/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import React, { useMemo } from 'react';
import { useSystemVitality } from '../hooks/useSystemVitality';
import { SoTEvent } from '../types';

export const SoTProgressBar: React.FC = () => {
    const { events } = useSystemVitality();
    
    // We only care about the latest active Session or recently ended one.
    // Instead of using effect and state which causes double-renders, we use derived state.
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
        } | null = null;
        
        // Find all SoT events and sort oldest first to reconstruct chronological state
        const sotEvents = events
            .filter(e => e.type === 'sot_progress')
            .map(e => e.data as SoTEvent)
            .reverse();

        for (const se of sotEvents) {
            const { type, data } = se.event;
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
                        // Resiliency: Fallback if we connected mid-session
                        activeSession = {
                            id: data.session_id,
                            roles: [],
                            currentRound: data.round,
                            status: 'active',
                            scores: [],
                            events: [],
                            abstentionCount: 0,
                            protocol: null,
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
                    }
                    break;
            }
        }
        
        return activeSession;
    }, [events]);

    if (!currentSession || currentSession.status === 'ended') {
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
            animation: 'fadeIn 0.3s ease-out',
            fontFamily: 'var(--font-main)',
            color: 'var(--text-primary)'
        }}>
            <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: 'var(--space-xs)' }}>
                <h3 style={{ margin: 0, fontSize: '0.85rem', fontWeight: 700, color: 'var(--accent-purple)', display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
                    <span style={{ position: 'relative', display: 'flex', height: '12px', width: '12px' }}>
                      <span className="ani-pulse" style={{ position: 'absolute', height: '100%', width: '100%', borderRadius: '50%', background: 'var(--accent-purple)', opacity: 0.7 }}></span>
                      <span style={{ position: 'relative', display: 'inline-flex', borderRadius: '50%', height: '12px', width: '12px', background: 'var(--accent-purple)' }}></span>
                    </span>
                    Society of Thought Active
                </h3>
                <div style={{ display: 'flex', alignItems: 'center', gap: 'var(--space-xs)' }}>
                    {currentSession.protocol && (
                        <span style={{
                            fontSize: '0.65rem',
                            color: 'var(--accent-cyan)',
                            padding: '2px 6px',
                            background: 'var(--accent-cyan-glass)',
                            border: '1px solid rgba(0,242,255,0.3)',
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
                            background: 'rgba(245,158,11,0.15)',
                            border: '1px solid rgba(245,158,11,0.3)',
                            borderRadius: '4px'
                        }} title="Voluntary Self-Abstentions">
                            🤚 {currentSession.abstentionCount}
                        </span>
                    )}
                    <span style={{
                        fontSize: '0.75rem',
                        color: 'var(--text-muted)',
                        padding: '2px 8px',
                        background: 'var(--bg-dark-obsidian)',
                        borderRadius: '4px'
                    }}>
                        Round {currentSession.currentRound}
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
                            border: `1px solid ${isFocus ? 'rgba(188,140,255,0.4)' : 'var(--border-glass)'}`,
                            background: isFocus ? 'var(--accent-purple-glass)' : 'var(--bg-dark-obsidian)',
                            color: isFocus ? 'var(--accent-purple)' : 'var(--text-secondary)'
                        }}>
                            {isFocus ? (
                                <div style={{ display: 'flex', justifyContent: 'space-between' }}>
                                    <span>{role} is thinking...</span>
                                    <span className="ani-pulse">..</span>
                                </div>
                            ) : (
                                <span>{role} completed</span>
                            )}
                        </div>
                    );
                })}
            </div>
            
            {currentSession.scores.length > 0 && (
                <div style={{ marginTop: 'var(--space-sm)', paddingTop: 'var(--space-xs)', borderTop: '1px solid var(--border-glass)' }}>
                    <div style={{ fontSize: '0.65rem', color: 'var(--text-muted)', marginBottom: '4px', textTransform: 'uppercase', fontWeight: 600 }}>Latest Scores</div>
                    <div style={{ display: 'flex', gap: 'var(--space-xs)' }}>
                        {currentSession.scores.map(([metric, score]: [string, number]) => {
                            const color = score >= 4 ? 'var(--accent-emerald)' : score >= 3 ? 'var(--accent-amber)' : 'var(--accent-rose)';
                            return (
                                <div key={metric} style={{
                                    fontSize: '0.75rem',
                                    padding: '2px 6px',
                                    borderRadius: '4px',
                                    background: `rgba(${score >= 4 ? '16,185,129' : score >= 3 ? '245,158,11' : '255,77,148'}, 0.15)`,
                                    color: color
                                }}>
                                    {metric}: {score}/5
                                </div>
                            );
                        })}
                    </div>
                </div>
            )}
        </div>
    );
};

/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import React from 'react';
import { AgentStats } from '../../types';
import VrmRenderer from '../../lib/vrm/VrmRenderer';
import InxRenderer from '../../lib/inx/InxRenderer';
import GlbRenderer from '../../lib/glb/GlbRenderer';
import ErrorBoundary from '../common/ErrorBoundary';
import { TokenSavingsIndicator } from '../common/TokenSavingsIndicator';
import { EkycStatusBadge } from '../character/EkycStatusBadge';
import { SoulStatusBadge } from '../character/SoulStatusBadge';

interface CharacterPanelProps {
    stats: AgentStats;
    onOpenViewer: () => void;
    isViewerOpen: boolean;
    modelUrl: string;
    avatarState: 'idle' | 'thinking' | 'speaking' | 'learning' | 'meditating' | 'awakened';
    mode: 'vrm' | 'inx' | 'glb' | 'off' | 'lite';
    sessionSavedChars?: number;
}

const CharacterPanel: React.FC<CharacterPanelProps> = ({ stats, onOpenViewer, isViewerOpen, modelUrl, avatarState, mode, sessionSavedChars }) => {
    const [ekycStatus, setEkycStatus] = React.useState<boolean | null>(null);
    const [soulState, setSoulState] = React.useState<string>('Awake');
    const [fetchedLevel, setFetchedLevel] = React.useState<number | null>(null);

    React.useEffect(() => {
        fetch('/api/v1/ekyc/status')
            .then(r => r.ok ? r.json() : Promise.reject('Status not ok'))
            .then(d => setEkycStatus(d.verified))
            .catch(e => console.error('EKYC fetch error', e));
        fetch('/api/v1/soul/status')
            .then(r => r.ok ? r.json() : Promise.reject('Status not ok'))
            .then(d => {
                setSoulState(d.state || 'Awake');
                if (d.level) setFetchedLevel(d.level);
            })
            .catch(e => console.error('Soul state fetch error', e));
    }, []);

    return (
        <div className="character-panel" style={{
            background: 'var(--panel-bg)',
            border: '1px solid var(--border-glass)',
            borderRadius: '16px',
            padding: '1.5rem',
            flex: 1,
            display: 'flex',
            flexDirection: 'column',
            gap: '1.5rem'
        }}>
            <div style={{ textAlign: 'center', marginBottom: '1rem' }}>
                <div 
                    className="avatar-billboard-container"
                    onClick={onOpenViewer}
                    style={{ 
                        height: '30vh', 
                        background: 'var(--black-30)', 
                        borderRadius: '12px', 
                        border: '1px solid var(--white-10)',
                        cursor: 'pointer',
                        position: 'relative',
                        overflow: 'hidden',
                        transition: 'border-color 0.2s',
                    }}
                    onMouseEnter={(e) => e.currentTarget.style.borderColor = 'var(--accent-cyan-50)'}
                    onMouseLeave={(e) => e.currentTarget.style.borderColor = 'var(--white-10)'}
                >
                    {!isViewerOpen && mode !== 'off' && (
                        <div style={{ position: 'absolute', inset: 0, pointerEvents: 'none' }}>
                            <ErrorBoundary fallback={null}>
                                {mode === 'vrm' && <VrmRenderer modelUrl={modelUrl} avatarState={avatarState} />}
                                {mode === 'glb' && <GlbRenderer modelUrl={modelUrl} avatarState={avatarState} />}
                                {mode === 'inx' && <InxRenderer modelUrl={modelUrl} avatarState={avatarState} />}
                            </ErrorBoundary>
                        </div>
                    )}
                    {isViewerOpen && (
                        <div style={{ position: 'absolute', inset: 0, display: 'flex', alignItems: 'center', justifyContent: 'center', color: 'var(--white-30)', fontSize: '0.8rem' }}>
                            Viewing in full screen...
                        </div>
                    )}
                </div>
            </div>

            <div>
                <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '0.5rem' }}>
                    <h3 className="font-display" style={{
                        margin: 0, 
                        fontWeight: 900, 
                        letterSpacing: '0.04em',
                        textTransform: 'uppercase' as const,
                        background: 'linear-gradient(135deg, var(--bg-secondary), var(--accent-cyan-70))',
                        WebkitBackgroundClip: 'text',
                        backgroundClip: 'text',
                        WebkitTextFillColor: 'transparent',
                    }}>Level {stats.level}</h3>
                    <span className="font-mono" style={{ fontSize: '0.8rem', color: 'var(--accent-cyan)' }}>{stats.exp} / {stats.level * 1000} EXP</span>
                </div>
                <div style={{ background: 'var(--white-10)', height: '6px', borderRadius: '3px', overflow: 'hidden' }}>
                    <div style={{ 
                        background: 'var(--accent-cyan)', 
                        height: '100%', 
                        width: `${Math.min(100, (stats.exp / (stats.level * 1000)) * 100)}%`,
                        transition: 'width 0.5s ease-out'
                    }} />
                </div>
            </div>

            <div style={{ display: 'flex', gap: '0.5rem', flexWrap: 'wrap' }}>
                <SoulStatusBadge level={fetchedLevel ?? stats.level} state={soulState} />
                <EkycStatusBadge status={ekycStatus} />
            </div>
            
            <div style={{ flex: 1 }}></div>

            <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '0.85rem' }}>
                <span style={{ color: 'var(--text-secondary)' }}>Resonance</span>
                <span className="font-mono" style={{ color: 'white', fontWeight: 'bold' }}>{stats.resonance}</span>
            </div>

            {sessionSavedChars !== undefined && (
                <TokenSavingsIndicator savedChars={sessionSavedChars} variant="compact" />
            )}
        </div>
    );
};

export default CharacterPanel;

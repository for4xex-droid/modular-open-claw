/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import React from 'react';
import { AgentStats } from '../../types';
import { Shield, Sparkles } from 'lucide-react';
import VrmRenderer from '../../lib/vrm/VrmRenderer';
import InxRenderer from '../../lib/inx/InxRenderer';
import GlbRenderer from '../../lib/glb/GlbRenderer';
import ErrorBoundary from '../common/ErrorBoundary';

interface CharacterPanelProps {
    stats: AgentStats;
    onOpenViewer: () => void;
    isViewerOpen: boolean;
    modelUrl: string;
    avatarState: 'idle' | 'thinking' | 'speaking' | 'learning' | 'meditating' | 'awakened';
    mode: 'vrm' | 'inx' | 'glb' | 'off' | 'lite';
}

const CharacterPanel: React.FC<CharacterPanelProps> = ({ stats, onOpenViewer, isViewerOpen, modelUrl, avatarState, mode }) => {
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
                        background: 'rgba(0,0,0,0.3)', 
                        borderRadius: '12px', 
                        border: '1px solid rgba(255,255,255,0.1)',
                        cursor: 'pointer',
                        position: 'relative',
                        overflow: 'hidden',
                        transition: 'border-color 0.2s',
                    }}
                    onMouseEnter={(e) => e.currentTarget.style.borderColor = 'rgba(0, 242, 255, 0.5)'}
                    onMouseLeave={(e) => e.currentTarget.style.borderColor = 'rgba(255,255,255,0.1)'}
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
                        <div style={{ position: 'absolute', inset: 0, display: 'flex', alignItems: 'center', justifyContent: 'center', color: 'rgba(255,255,255,0.3)', fontSize: '0.8rem' }}>
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
                        background: 'linear-gradient(135deg, #f0f2f5, rgba(0, 242, 255, 0.7))',
                        WebkitBackgroundClip: 'text',
                        backgroundClip: 'text',
                        WebkitTextFillColor: 'transparent',
                    }}>Level {stats.level}</h3>
                    <span className="font-mono" style={{ fontSize: '0.8rem', color: 'var(--accent-cyan)' }}>{stats.exp} / {stats.level * 1000} EXP</span>
                </div>
                <div style={{ background: 'rgba(255,255,255,0.1)', height: '6px', borderRadius: '3px', overflow: 'hidden' }}>
                    <div style={{ 
                        background: 'var(--accent-cyan)', 
                        height: '100%', 
                        width: `${Math.min(100, (stats.exp / (stats.level * 1000)) * 100)}%`,
                        transition: 'width 0.5s ease-out'
                    }} />
                </div>
            </div>

            <div style={{ display: 'flex', gap: '0.5rem', flexWrap: 'wrap' }}>
                <span style={{ background: 'rgba(255, 255, 255, 0.1)', color: 'white', padding: '0.2rem 0.6rem', borderRadius: '12px', fontSize: '0.75rem', display: 'flex', alignItems: 'center' }}>
                    Lvl {fetchedLevel ?? stats.level} | {soulState}
                </span>
                {ekycStatus === true && (
                    <span style={{ background: 'rgba(0, 255, 100, 0.1)', color: 'var(--success, #00ff64)', padding: '0.2rem 0.6rem', borderRadius: '12px', fontSize: '0.75rem', display: 'flex', alignItems: 'center' }}>
                        ✓ Verified
                    </span>
                )}
                {ekycStatus === false && (
                    <span style={{ background: 'rgba(255, 100, 100, 0.1)', color: 'var(--danger, #ff6464)', padding: '0.2rem 0.6rem', borderRadius: '12px', fontSize: '0.75rem', display: 'flex', alignItems: 'center' }}>
                        ⚠ Unverified
                    </span>
                )}
                <span style={{ background: 'rgba(0, 242, 255, 0.1)', color: 'var(--accent-cyan)', padding: '0.2rem 0.6rem', borderRadius: '12px', fontSize: '0.75rem', display: 'flex', alignItems: 'center', gap: '0.25rem' }}>
                    <Shield size={12} /> Secure
                </span>
                <span style={{ background: 'rgba(188, 140, 255, 0.1)', color: 'var(--accent-purple)', padding: '0.2rem 0.6rem', borderRadius: '12px', fontSize: '0.75rem', display: 'flex', alignItems: 'center', gap: '0.25rem' }}>
                    <Sparkles size={12} /> Curious
                </span>
            </div>
            
            <div style={{ flex: 1 }}></div>

            <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '0.85rem' }}>
                <span style={{ color: 'var(--text-secondary)' }}>Resonance</span>
                <span className="font-mono" style={{ color: 'white', fontWeight: 'bold' }}>{stats.resonance}</span>
            </div>
        </div>
    );
};

export default CharacterPanel;

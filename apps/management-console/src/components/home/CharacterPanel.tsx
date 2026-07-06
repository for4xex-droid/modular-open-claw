/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import React from 'react';
import { useTranslation } from '../../i18n';
import { AgentStats } from '../../types';
import VrmRenderer from '../../lib/vrm/VrmRenderer';
import InxRenderer from '../../lib/inx/InxRenderer';
import GlbRenderer from '../../lib/glb/GlbRenderer';
import ErrorBoundary from '../common/ErrorBoundary';
import { TokenSavingsIndicator } from '../common/TokenSavingsIndicator';
import { ProofPowerIndicator } from '../common/ProofPowerIndicator';
import { EkycStatusBadge } from '../character/EkycStatusBadge';
import { SoulStatusBadge } from '../character/SoulStatusBadge';
import { authenticatedFetch } from '../../lib/auth';
import { API_BASE } from '../../config';
import { useToast } from '../common/Toast';

interface CharacterPanelProps {
    stats: AgentStats;
    onOpenViewer: () => void;
    isViewerOpen: boolean;
    modelUrl: string;
    avatarState: 'idle' | 'thinking' | 'speaking' | 'learning' | 'meditating' | 'awakened';
    mode: 'vrm' | 'inx' | 'glb' | 'off' | 'lite';
    sessionSavedChars?: number;
    proofPower?: number;
}

const CharacterPanel: React.FC<CharacterPanelProps> = ({ stats, onOpenViewer, isViewerOpen, modelUrl, avatarState, mode, sessionSavedChars }) => {
    const { t } = useTranslation();
    const { showToast } = useToast();
    const [ekycStatus, setEkycStatus] = React.useState<boolean | null>(null);
    const [soulState, setSoulState] = React.useState<string>('Awake');
    const [fetchedLevel, setFetchedLevel] = React.useState<number | null>(null);
    // U0-B2: 失敗はトースト連発ではなくパネル内表示+再試行で伝える
    const [statusLoadFailed, setStatusLoadFailed] = React.useState<boolean>(false);
    const [retryCounter, setRetryCounter] = React.useState<number>(0);

    React.useEffect(() => {
        let cancelled = false;
        setStatusLoadFailed(false);
        const ekycFetch = authenticatedFetch(`${API_BASE}/api/v1/ekyc/status`)
            .then(r => r.ok ? r.json() : Promise.reject('Status not ok'))
            .then(d => { if (!cancelled) setEkycStatus(d.verified); });
        const soulFetch = authenticatedFetch(`${API_BASE}/api/v1/soul/status`)
            .then(r => r.ok ? r.json() : Promise.reject('Status not ok'))
            .then(d => {
                if (cancelled) return;
                setSoulState(d.state || 'Awake');
                if (d.level) setFetchedLevel(d.level);
            });
        Promise.allSettled([ekycFetch, soulFetch]).then(results => {
            if (cancelled) return;
            const failures = results.filter(r => r.status === 'rejected');
            if (failures.length > 0) {
                failures.forEach(f => console.error('Status fetch error', (f as PromiseRejectedResult).reason));
                setStatusLoadFailed(true);
            }
        });
        return () => { cancelled = true; };
    }, [retryCounter]);

    const handleVerifyEkyc = async () => {
        try {
            const res = await authenticatedFetch(`${API_BASE}/api/v1/ekyc/session`, { method: 'POST' });
            if (res.ok) {
                const data = await res.json();
                if (data.session_url) {
                    window.open(data.session_url, '_blank', 'noopener,noreferrer');
                }
            } else {
                showToast('error', t('ekyc.sessionFailed', { defaultValue: 'Failed to create eKYC session.' }));
            }
        } catch (e) {
            console.error('Error creating eKYC session', e);
            showToast('error', t('common.networkError', { defaultValue: 'A network error occurred.' }));
        }
    };


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
                                {localStorage.getItem('aiome_test_mode') === 'true' ? (
                                    <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', height: '100%', color: 'var(--accent-cyan)', fontWeight: 'bold' }}>[Mock 3D View]</div>
                                ) : (
                                    <>
                                        {mode === 'vrm' && <VrmRenderer modelUrl={modelUrl} avatarState={avatarState} />}
                                        {mode === 'glb' && <GlbRenderer modelUrl={modelUrl} avatarState={avatarState} />}
                                        {mode === 'inx' && <InxRenderer modelUrl={modelUrl} avatarState={avatarState} />}
                                    </>
                                )}
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
                        background: 'linear-gradient(135deg, var(--accent-cyan), var(--accent-purple))',
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

            {statusLoadFailed && (
                <div style={{
                    display: 'flex',
                    alignItems: 'center',
                    justifyContent: 'space-between',
                    gap: '0.5rem',
                    padding: '0.5rem 0.75rem',
                    background: 'var(--accent-rose-10, var(--black-20))',
                    border: '1px solid var(--accent-rose-30)',
                    borderRadius: 'var(--radius-md)',
                    fontSize: '0.8rem',
                    color: 'var(--accent-rose)',
                }}>
                    <span>{t('character.loadFailed', { defaultValue: 'Failed to load identity status.' })}</span>
                    <button
                        onClick={() => setRetryCounter(c => c + 1)}
                        style={{
                            background: 'transparent',
                            border: '1px solid var(--accent-rose-50)',
                            borderRadius: 'var(--radius-sm)',
                            color: 'var(--accent-rose)',
                            padding: '0.15rem 0.6rem',
                            fontSize: '0.75rem',
                            cursor: 'pointer',
                            whiteSpace: 'nowrap',
                        }}
                    >
                        {t('error.retry', { defaultValue: 'Retry' })}
                    </button>
                </div>
            )}

            <div style={{ display: 'flex', gap: '0.5rem', flexWrap: 'wrap', alignItems: 'center' }}>
                <SoulStatusBadge level={fetchedLevel ?? stats.level} state={soulState} />
                <EkycStatusBadge status={ekycStatus} />
                {ekycStatus === false && (
                    <button 
                        onClick={handleVerifyEkyc}
                        style={{
                            background: 'var(--accent-cyan)',
                            color: 'black',
                            border: 'none',
                            borderRadius: '4px',
                            padding: '0.25rem 0.75rem',
                            fontSize: '0.8rem',
                            fontWeight: 'bold',
                            cursor: 'pointer'
                        }}
                    >
                        {t('ekyc.startVerification')}
                    </button>
                )}
            </div>
            
            <div style={{ flex: 1 }}></div>

            <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '0.85rem' }}>
                <span style={{ color: 'var(--text-secondary)', cursor: 'help' }} data-tooltip={t('character.resonanceTooltip')}>Resonance</span>
                <span className="font-mono" style={{ color: 'white', fontWeight: 'bold' }}>{stats.resonance}</span>
            </div>

            {sessionSavedChars !== undefined && (
                <div style={{ display: 'flex', gap: '0.5rem', flexWrap: 'wrap', marginTop: '0.5rem' }}>
                    <ProofPowerIndicator variant="compact" />
                    <TokenSavingsIndicator savedChars={sessionSavedChars} variant="compact" />
                </div>
            )}
        </div>
    );
};

export default CharacterPanel;

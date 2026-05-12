/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import React, { useEffect } from 'react';
import { Loader2, DownloadCloud, CheckCircle } from 'lucide-react';
import { useModelStatus } from '../hooks/useModelStatus';
import { useTranslation } from '../i18n';

interface ModelSetupStepProps {
    onNext: () => void;
    onSkip: () => void;
}

export const ModelSetupStep: React.FC<ModelSetupStepProps> = ({ onNext, onSkip }) => {
    const { t } = useTranslation();
    const { status, loading, error, pullProgress, isPulling, checkStatus, pullModel } = useModelStatus();

    useEffect(() => {
        checkStatus();
    }, [checkStatus]);

    if (loading) {
        return (
            <div style={{ padding: '2rem', textAlign: 'center' }}>
                <Loader2 size={32} className="ani-spin" style={{ color: 'var(--accent-cyan)', margin: '0 auto 1rem' }} />
                <div style={{ color: 'var(--text-secondary)' }}>{t('onboarding.llmSetup.checking')}</div>
            </div>
        );
    }

    if (!status?.ollama_connected) {
        return (
            <div style={{ display: 'flex', flexDirection: 'column', gap: '1rem', width: '100%', marginTop: '0.5rem', textAlign: 'left' }}>
                <div style={{ padding: '1rem', background: 'var(--accent-cyan-10)', border: '1px solid var(--accent-cyan-30)', borderRadius: '12px' }}>
                    <div style={{ fontWeight: 800, color: 'var(--accent-cyan)', marginBottom: '0.5rem' }}>{t('onboarding.llmSetup.notConnected')}</div>
                    <div style={{ fontSize: '0.9rem', color: 'var(--text-secondary)' }}>
                        {t('onboarding.llmSetup.installGuide')}
                    </div>
                    <div style={{ marginTop: '0.8rem', padding: '0.8rem', background: 'var(--white-05)', borderRadius: '8px', fontSize: '0.8rem', color: 'var(--text-secondary)' }}>
                        <strong style={{ color: 'var(--text-primary)' }}>💡 {t('onboarding.llmSetup.tipsTitle', { defaultValue: 'Troubleshooting Tips:' })}</strong>
                        <ul style={{ margin: '0.5rem 0 0 0', paddingLeft: '1.2rem', display: 'flex', flexDirection: 'column', gap: '0.3rem' }}>
                            <li>{t('onboarding.llmSetup.tip1', { defaultValue: 'Ensure Ollama is running (check your system tray or menu bar).' })}</li>
                            <li>{t('onboarding.llmSetup.tip2', { defaultValue: 'The default port is 11434. If changed, please update the API base URL.' })}</li>
                            <li>{t('onboarding.llmSetup.tip3', { defaultValue: 'For LM Studio, start the local server (default port 1234).' })}</li>
                        </ul>
                    </div>
                </div>
                
                <button 
                    onClick={() => {}}
                    style={{
                        padding: '1rem', borderRadius: 'var(--radius-md)', textAlign: 'left',
                        border: '1px solid var(--accent-cyan-30)',
                        background: 'var(--accent-cyan-05)',
                        cursor: 'pointer', transition: 'all var(--speed-normal) ease',
                        display: 'flex', alignItems: 'center', gap: '1rem'
                    }}
                >
                    <div style={{ fontSize: '1.5rem' }}>🖥️</div>
                    <div>
                        <div style={{ fontWeight: 800, color: 'var(--text-primary)' }}>{t('onboarding.llmSetup.optionLocal.title', { defaultValue: 'Local PC' })}</div>
                        <div style={{ fontSize: '0.85rem', color: 'var(--text-secondary)' }}>{t('onboarding.llmSetup.optionLocal.desc', { defaultValue: 'Install Ollama to run AI privately' })}</div>
                    </div>
                </button>

                <button 
                    onClick={onSkip}
                    style={{
                        padding: '1rem', borderRadius: 'var(--radius-md)', textAlign: 'left',
                        border: '1px solid var(--border-glass-bright)',
                        background: 'var(--white-03)',
                        cursor: 'pointer', transition: 'all var(--speed-normal) ease',
                        display: 'flex', alignItems: 'center', gap: '1rem'
                    }}
                >
                    <div style={{ fontSize: '1.5rem' }}>☁️</div>
                    <div>
                        <div style={{ fontWeight: 800, color: 'var(--text-primary)' }}>{t('onboarding.llmSetup.optionCloud.title', { defaultValue: 'Cloud API' })}</div>
                        <div style={{ fontSize: '0.85rem', color: 'var(--text-secondary)' }}>{t('onboarding.llmSetup.optionCloud.desc', { defaultValue: 'Use Gemini or OpenAI (Requires API Key)' })}</div>
                    </div>
                </button>

                <button 
                    onClick={onSkip}
                    style={{
                        padding: '1rem', borderRadius: 'var(--radius-md)', textAlign: 'left',
                        border: '1px solid var(--border-glass-bright)',
                        background: 'var(--white-03)',
                        cursor: 'pointer', transition: 'all var(--speed-normal) ease',
                        display: 'flex', alignItems: 'center', gap: '1rem'
                    }}
                >
                    <div style={{ fontSize: '1.5rem' }}>👀</div>
                    <div>
                        <div style={{ fontWeight: 800, color: 'var(--text-primary)' }}>{t('onboarding.llmSetup.optionDemo.title', { defaultValue: 'Demo' })}</div>
                        <div style={{ fontSize: '0.85rem', color: 'var(--text-secondary)' }}>{t('onboarding.llmSetup.optionDemo.desc', { defaultValue: 'Just looking around for now' })}</div>
                    </div>
                </button>
            </div>
        );
    }

    if (!status.setup_required && status.configured_model_available) {
        // Automatically skip if setup not required ? Actually, returning early or showing "ready" is better.
        return (
            <div style={{ display: 'flex', flexDirection: 'column', gap: '1.5rem', width: '100%', marginTop: '0.5rem', textAlign: 'center' }}>
                <div style={{ display: 'flex', justifyContent: 'center', marginBottom: '1rem' }}>
                    <CheckCircle size={48} color="var(--accent-cyan)" />
                </div>
                <div style={{ fontWeight: 800, fontSize: '1.2rem', color: 'var(--text-primary)' }}>{t('onboarding.llmSetup.modelReady')}</div>
                <div style={{ fontSize: '0.9rem', color: 'var(--text-secondary)' }}>
                    {t('onboarding.llmSetup.modelReadyDesc').replace('{{model}}', status.configured_model)}
                </div>
                <button 
                    onClick={onNext}
                    style={{
                        marginTop: '1rem', padding: '1rem', borderRadius: 'var(--radius-md)', textAlign: 'center',
                        border: 'none', background: 'var(--accent-cyan)', color: 'var(--bg-primary)',
                        cursor: 'pointer', fontWeight: 700
                    }}
                >
                    {t('onboarding.next')}
                </button>
            </div>
        );
    }

    if (isPulling && pullProgress) {
        const percent = pullProgress.total ? (pullProgress.completed || 0) / pullProgress.total * 100 : 0;
        
        return (
             <div style={{ display: 'flex', flexDirection: 'column', gap: '1.5rem', width: '100%', marginTop: '0.5rem', textAlign: 'left' }}>
                <div style={{ fontWeight: 800, color: 'var(--accent-cyan)' }}>{t('onboarding.llmSetup.downloading')}</div>
                <div style={{ fontSize: '0.85rem', color: 'var(--text-secondary)' }}>{pullProgress.status}</div>
                <div style={{ width: '100%', height: '8px', background: 'var(--white-10)', borderRadius: 'var(--radius-sm)', overflow: 'hidden' }}>
                    <div style={{ width: `${percent}%`, height: '100%', background: 'var(--accent-cyan)', transition: 'width var(--speed-normal) ease' }} />
                </div>
                <div style={{ fontSize: '0.8rem', color: 'var(--text-secondary)', textAlign: 'right' }}>
                    {percent.toFixed(1)}%
                </div>
                {error && <div style={{ color: 'var(--accent-rose)', fontSize: '0.85rem' }}>{error}</div>}
             </div>
        );
    }

    const options = [
        { id: 'gemma4:26b', title: t('onboarding.llmSetup.options.gemma4_26b.title'), desc: t('onboarding.llmSetup.options.gemma4_26b.desc') },
        { id: 'gemma4:12b', title: t('onboarding.llmSetup.options.gemma4_12b.title'), desc: t('onboarding.llmSetup.options.gemma4_12b.desc') },
        { id: 'gemma4:4b', title: t('onboarding.llmSetup.options.gemma4_4b.title'), desc: t('onboarding.llmSetup.options.gemma4_4b.desc') }
    ];

    return (
        <div style={{ display: 'flex', flexDirection: 'column', gap: '1rem', width: '100%', marginTop: '0.5rem' }}>
            {error && <div style={{ padding: '0.8rem', background: 'var(--accent-rose-10)', color: 'var(--accent-rose)', borderRadius: '8px', fontSize: '0.8rem' }}>{error}</div>}
            
            {options.map(opt => (
                <button
                    key={opt.id}
                    onClick={() => pullModel(opt.id)}
                    style={{
                        display: 'flex', alignItems: 'center', justifyContent: 'space-between',
                        padding: '1rem', borderRadius: 'var(--radius-md)', textAlign: 'left',
                        border: '1px solid var(--border-glass-bright)',
                        background: 'var(--white-03)',
                        cursor: 'pointer', transition: 'all var(--speed-normal) ease'
                    }}
                >
                    <div>
                        <div style={{ fontWeight: 800, color: 'var(--text-primary)' }}>{opt.title}</div>
                        <div style={{ fontSize: '0.8rem', color: 'var(--text-secondary)', marginTop: '0.3rem' }}>{opt.desc}</div>
                    </div>
                    <DownloadCloud size={20} color="var(--accent-cyan)" />
                </button>
            ))}
            
            <button 
                onClick={onSkip}
                style={{
                    padding: '0.8rem', marginTop: '0.5rem', background: 'transparent',
                    border: 'none', color: 'var(--text-secondary)',
                    cursor: 'pointer', textDecoration: 'underline'
                }}
            >
                {t('onboarding.llmSetup.skipCloud')}
            </button>
        </div>
    );
};

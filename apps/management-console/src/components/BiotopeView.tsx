/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import React, { useEffect, useState } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { Activity, Zap } from 'lucide-react';
import { AgentStats, VitalityUIEvent } from '../types';
import { useTranslation } from '../i18n';
import { TreasureBox } from './TreasureBox';
import { TokenSavingsIndicator } from './common/TokenSavingsIndicator';

interface BiotopeViewProps {
    stats: AgentStats;
    isConnected: boolean;
    recentEvents: VitalityUIEvent[];
    sessionSavedChars?: number;
}

const BiotopeView: React.FC<BiotopeViewProps> = ({ stats, isConnected, recentEvents, sessionSavedChars = 0 }) => {
    const { t } = useTranslation();
    const [pulseLevel, setPulseLevel] = useState(0);

    // Local visual pulse effect still responds to stats changes for flair
    useEffect(() => {
        setPulseLevel(prev => Math.min(100, prev + 20));
    }, [stats.level]);

    useEffect(() => {
        if (pulseLevel <= 0) return;
        const timer = setTimeout(() => setPulseLevel(prev => Math.max(0, prev - 5)), 2000);
        return () => clearTimeout(timer);
    }, [pulseLevel]);

    return (
        <div className="biotope-view" style={{ display: 'grid', gridTemplateColumns: '1fr var(--layout-right-panel-width)', gap: 'var(--layout-panel-gap)' }}>
            {/* Left: Avatar & Main Visualization */}
            <div className="main-panel ani-fade" style={{
                display: 'flex', flexDirection: 'column', alignItems: 'center', justifyContent: 'center',
                padding: '3rem', position: 'relative', minHeight: '500px',
                background: 'var(--white-01)', // Almost fully transparent
                backdropFilter: 'none', // Remove blur to see VRM clearly
                border: '1px solid var(--white-05)',
                overflow: 'hidden'
            }}>
                <div style={{ position: 'absolute', top: '1.5rem', left: '1.5rem', display: 'flex', gap: '1rem', alignItems: 'center', zIndex: 10 }}>
                    <Activity size={20} color="var(--accent-cyan)" className="ani-breath" />
                    <h3 style={{ fontSize: '1rem', fontWeight: 600, color: 'var(--text-muted)' }}>{t('biotope.liveVitality')}</h3>
                </div>

                {/* MoodRing & Pulsing Aura Background */}
                <div style={{ position: 'absolute', zIndex: 0, top: '40%', left: '50%', transform: 'translate(-50%, -50%)', pointerEvents: 'none' }}>
                    {/* Outer Cyan/Purple Ring */}
                    <motion.div
                        animate={{ rotate: 360 }}
                        transition={{ duration: 25, repeat: Infinity, ease: 'linear' }}
                        style={{
                            position: 'absolute', top: -200, left: -200, width: 400, height: 400,
                            borderRadius: '50%',
                            background: 'conic-gradient(from 0deg, transparent 0%, var(--accent-cyan-50) 25%, transparent 50%, var(--accent-purple-50) 75%, transparent 100%)',
                            WebkitMaskImage: 'radial-gradient(circle, transparent 60%, var(--black-90) 61%)'
                        }}
                    />
                    {/* Inner Rosa/Cyan Ring */}
                    <motion.div
                        animate={{ rotate: -360 }}
                        transition={{ duration: 18, repeat: Infinity, ease: 'linear' }}
                        style={{
                            position: 'absolute', top: -160, left: -160, width: 320, height: 320,
                            borderRadius: '50%', border: '1px solid var(--white-05)',
                            background: 'conic-gradient(from 90deg, transparent 0%, var(--accent-rose-40) 30%, transparent 60%, var(--accent-cyan-40) 90%, transparent 100%)',
                            WebkitMaskImage: 'radial-gradient(circle, transparent 65%, var(--black-90) 66%)'
                        }}
                    />
                    {/* Dynamic Vitality Pulse */}
                    <motion.div
                        animate={{ scale: [1, 1.1 + (pulseLevel / 50), 1], opacity: [0.3, 0.8, 0.3] }}
                        transition={{ duration: 4, repeat: Infinity, ease: 'easeInOut' }}
                        style={{
                            position: 'absolute', top: -140, left: -140, width: 280, height: 280,
                            borderRadius: '50%',
                            background: 'radial-gradient(circle, var(--accent-cyan-15) 0%, transparent 70%)',
                            filter: 'blur(15px)'
                        }}
                    />
                </div>

                <div style={{ zIndex: 1, display: 'flex', flexDirection: 'column', alignItems: 'center', gap: '2rem', marginTop: 'auto', pointerEvents: 'none' }}>
                    {/* Space for the Avatar to sit */}
                    <div style={{ height: '320px' }} />

                    <div style={{ textAlign: 'center', background: 'var(--bg-glass-heavy)', padding: '1.5rem 2rem', borderRadius: 'var(--radius-lg)', backdropFilter: 'blur(12px)', border: '1px solid var(--border-glass-bright)', width: '320px' }}>
                        <h2 style={{ fontSize: '1.6rem', fontWeight: 800, marginBottom: '1.25rem', textShadow: '0 0 15px var(--white-30)', letterSpacing: '-0.02em' }}>
                            {t('sidebar.level')} {stats.level} <span style={{ color: 'var(--accent-purple)', fontSize: '0.9rem', fontWeight: 600, textShadow: 'var(--glow-purple)', verticalAlign: 'middle', marginLeft: '0.5rem' }}>{t('biotope.ascension', { n: Math.floor(stats.level / 10) })}</span>
                        </h2>

                        <div style={{ display: 'flex', flexDirection: 'column', gap: '1.2rem' }}>
                            {/* Resonance Meter */}
                            <div style={{ width: '100%' }}>
                                <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '0.75rem', marginBottom: '0.4rem', color: 'var(--accent-cyan)', fontWeight: 700 }}>
                                    <span>{t('biotope.resonance')}</span>
                                    <span>{stats.resonance}%</span>
                                </div>
                                <div style={{ height: '6px', background: 'var(--white-05)', borderRadius: '3px', overflow: 'hidden' }}>
                                    <motion.div
                                        initial={{ width: 0 }}
                                        animate={{ width: `${stats.resonance}%` }}
                                        style={{ height: '100%', background: 'var(--accent-cyan)', boxShadow: 'var(--glow-cyan)' }}
                                    />
                                </div>
                            </div>

                            {/* Creativity Meter */}
                            <div style={{ width: '100%' }}>
                                <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '0.75rem', marginBottom: '0.4rem', color: 'var(--accent-amber)', fontWeight: 700 }}>
                                    <span>{t('biotope.creativity')}</span>
                                    <span>{stats.creativity}%</span>
                                </div>
                                <div style={{ height: '6px', background: 'var(--white-05)', borderRadius: '3px', overflow: 'hidden' }}>
                                    <motion.div
                                        initial={{ width: 0 }}
                                        animate={{ width: `${stats.creativity}%` }}
                                        style={{ height: '100%', background: 'var(--accent-amber)', boxShadow: 'var(--glow-amber)' }}
                                    />
                                </div>
                            </div>

                            {/* Fatigue Meter (Inverse color logic: red is high) */}
                            <div style={{ width: '100%' }}>
                                <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '0.75rem', marginBottom: '0.4rem', color: stats.fatigue > 70 ? 'var(--accent-rose)' : 'var(--text-muted)', fontWeight: 700 }}>
                                    <span>{t('biotope.neuralFatigue')}</span>
                                    <span>{stats.fatigue}%</span>
                                </div>
                                <div style={{ height: '6px', background: 'var(--white-05)', borderRadius: '3px', overflow: 'hidden' }}>
                                    <motion.div
                                        initial={{ width: 0 }}
                                        animate={{ width: `${stats.fatigue}%` }}
                                        style={{ height: '100%', background: stats.fatigue > 70 ? 'var(--accent-rose)' : 'var(--text-secondary)', boxShadow: stats.fatigue > 70 ? 'var(--glow-rose)' : 'none' }}
                                    />
                                </div>
                            </div>

                            <div style={{ display: 'flex', gap: '0.5rem', alignItems: 'center', color: 'var(--text-secondary)', fontSize: '0.8rem', justifyContent: 'center', marginTop: '0.5rem' }}>
                                <Zap size={12} color="var(--accent-amber)" /> {Math.floor(stats.exp / 10)} {t('biotope.techExperience')}
                            </div>
                        </div>
                    </div>
                </div>
            </div>

            {/* Right: Recent Events Feed */}
            <div style={{ display: 'flex', flexDirection: 'column', gap: 'var(--space-md)' }}>
                <div className="main-panel ani-slide-right" style={{ padding: '0', flex: 1 }}>
                    <div className="panel-header" style={{ padding: '1rem 1.5rem' }}>
                        <h4 style={{ fontSize: '0.85rem', letterSpacing: '0.1em', fontWeight: 700 }}>{t('biotope.chroniclePulse')}</h4>
                    </div>
                    <div style={{ overflowY: 'auto', padding: '1rem' }}>
                        <AnimatePresence mode="popLayout">
                            {recentEvents.length === 0 ? (
                                <div key="empty" style={{ padding: '2rem', textAlign: 'center', color: 'var(--text-muted)', fontSize: '0.85rem' }}>
                                    {t('biotope.monitoringActivity')}
                                </div>
                            ) : (
                                recentEvents.map(event => (
                                    <motion.div
                                        key={event.id}
                                        initial={{ x: 20, opacity: 0 }}
                                        animate={{ x: 0, opacity: 1 }}
                                        exit={{ x: -20, opacity: 0 }}
                                        style={{
                                            padding: '1rem',
                                            borderRadius: 'var(--radius-md)',
                                            background: 'var(--white-02)',
                                            borderLeft: `3px solid ${event.color}`,
                                            marginBottom: '0.75rem',
                                            boxShadow: '0 4px 12px var(--black-10)',
                                            fontSize: '0.85rem'
                                        }}
                                    >
                                        <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem', marginBottom: '0.4rem', color: event.color, fontWeight: 700 }}>
                                            {event.icon} {event.title}
                                        </div>
                                        <div style={{ color: 'var(--text-secondary)', lineHeight: 1.4, overflow: 'hidden', textOverflow: 'ellipsis', display: '-webkit-box', WebkitLineClamp: 2, WebkitBoxOrient: 'vertical' }}>
                                            {event.desc}
                                        </div>
                                    </motion.div>
                                ))
                            )}
                        </AnimatePresence>
                    </div>
                </div>

                {/* AgentSense: TreasureBox */}
                <div className="ani-slide-right">
                    <TreasureBox />
                </div>
                
                <div className="ani-slide-right">
                    <TokenSavingsIndicator savedChars={sessionSavedChars} variant="full" />
                </div>

                <div className="stat-card ani-slide-right" style={{ padding: '1.25rem' }}>
                    <div style={{ fontSize: '0.75rem', color: 'var(--text-muted)', marginBottom: '0.5rem' }}>{t('biotope.synergyHeartbeat')}</div>
                    <div className="font-display" style={{ fontSize: '1.2rem', fontWeight: 800, color: 'var(--accent-emerald)', display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
                        {isConnected ? t('biotope.stable') : t('biotope.weak')}
                        <motion.div
                            animate={{ scale: [1, 1.2, 1], opacity: [0.5, 1, 0.5] }}
                            transition={{ duration: 1, repeat: Infinity }}
                            style={{ width: '8px', height: '8px', borderRadius: '50%', background: 'currentColor' }}
                        />
                    </div>
                </div>
            </div>
        </div>
    );
};

export default BiotopeView;

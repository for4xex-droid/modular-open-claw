/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import React, { useEffect, useState } from 'react';
import { motion } from 'framer-motion';
import { Zap, Terminal, BrainCircuit, Clock, Sparkles } from 'lucide-react';
import { API_BASE } from "../config";
import { authenticatedFetch } from '../lib/auth';
import { useTranslation } from '../i18n';

interface TimelineEvent {
    id?: string;
    _type: 'karma' | 'evolution';
    created_at: string;
    node_id?: string;
    karma_type?: string;
    job_id?: string;
    lesson?: string;
    inspiration?: string;
    event_type?: string;
    description?: string;
}

const Timeline: React.FC = () => {
    const { t } = useTranslation();
    const [events, setEvents] = useState<TimelineEvent[]>([]);
    const [selfNodeId, setSelfNodeId] = useState<string>("");
    const [loading, setLoading] = useState(true);
    const [error, setError] = useState<string | null>(null);

    useEffect(() => {
        const fetchData = async () => {
            setLoading(true);
            setError(null);
            try {
                // Fetch all data in parallel for reduced latency
                const [healthRes, karmaRes, evoRes] = await Promise.all([
                    authenticatedFetch(`${API_BASE}/api/health`),
                    authenticatedFetch(`${API_BASE}/api/synergy/karma`),
                    authenticatedFetch(`${API_BASE}/api/system/evolution`),
                ]);

                // Process health response
                if (healthRes.ok) {
                    const health = await healthRes.json();
                    setSelfNodeId(typeof health?.node_id === 'string' ? health.node_id : '');
                }

                // Process karma response
                let karmas: Record<string, unknown>[] = [];
                if (karmaRes.ok) {
                    const karmasRaw = await karmaRes.json();
                    karmas = Array.isArray(karmasRaw) ? karmasRaw : [];
                }

                // Process evolution response
                let evos: Record<string, unknown>[] = [];
                if (evoRes.ok) {
                    const evosRaw = await evoRes.json();
                    evos = Array.isArray(evosRaw) ? evosRaw : [];
                }

                // Merge and sort (NaN-safe: treat missing dates as epoch 0)
                const merged: TimelineEvent[] = [
                    ...karmas.map((k) => ({
                        id: String(k.id ?? ''),
                        _type: 'karma' as const,
                        created_at: String(k.created_at ?? ''),
                        node_id: typeof k.node_id === 'string' ? k.node_id : undefined,
                        karma_type: typeof k.karma_type === 'string' ? k.karma_type : undefined,
                        job_id: typeof k.job_id === 'string' ? k.job_id : undefined,
                        lesson: typeof k.lesson === 'string' ? k.lesson : undefined,
                        inspiration: typeof k.inspiration === 'string' ? k.inspiration : undefined,
                    })),
                    ...evos.map((e) => ({
                        id: String(e.id ?? ''),
                        _type: 'evolution' as const,
                        created_at: String(e.created_at ?? ''),
                        event_type: typeof e.event_type === 'string' ? e.event_type : undefined,
                        description: typeof e.description === 'string' ? e.description : undefined,
                    }))
                ].sort((a, b) => (new Date(b.created_at).getTime() || 0) - (new Date(a.created_at).getTime() || 0));

                setEvents(merged);
            } catch (e) {
                const message = e instanceof Error ? e.message : 'Unknown error';
                console.error("Failed to fetch timeline data", e);
                setError(message);
            } finally {
                setLoading(false);
            }
        };

        fetchData();
    }, []);

    return (
        <div className="main-panel ani-fade" style={{ flex: 1, display: 'flex', flexDirection: 'column' }}>
            <div className="panel-header">
                <div style={{ display: 'flex', alignItems: 'center', gap: '1rem' }}>
                    <Clock size={20} color="var(--accent-primary)" />
                    <h3>{t('timeline.title')}</h3>
                </div>
                <div style={{ fontSize: '0.8rem', color: 'var(--text-muted)' }}>
                    {events.length} {t('timeline.chronicles') || 'CHRONICLES'}
                </div>
            </div>

            <div style={{ padding: '1.5rem', maxHeight: '75vh', overflowY: 'auto' }}>
                {loading ? (
                    <div style={{ padding: '4rem', textAlign: 'center', color: 'var(--text-muted)' }}>
                        <div className="ani-pulse">{t('timeline.syncing')}</div>
                    </div>
                ) : error ? (
                    <div style={{ padding: '4rem', textAlign: 'center', color: 'var(--accent-rose)' }}>
                        <Zap size={48} style={{ opacity: 0.3, marginBottom: '1rem' }} />
                        <p>{t('timeline.error') || 'Failed to load timeline data'}</p>
                        <p style={{ fontSize: '0.75rem', color: 'var(--text-muted)', marginTop: '0.5rem' }}>{error}</p>
                    </div>
                ) : events.length === 0 ? (
                    <div style={{ padding: '4rem', textAlign: 'center', color: 'var(--text-muted)' }}>
                        <Zap size={48} style={{ opacity: 0.1, marginBottom: '1rem' }} />
                        <p>{t('timeline.noRecords')}</p>
                    </div>
                ) : (
                    <div style={{ position: 'relative' }}>
                        <div style={{
                            position: 'absolute',
                            left: '16px',
                            top: '0',
                            bottom: '0',
                            width: '2px',
                            background: 'linear-gradient(to bottom, var(--accent-primary), var(--fluid-warm-ivory), transparent)',
                            opacity: 0.2
                        }} />

                        <div style={{ display: 'flex', flexDirection: 'column', gap: '1.5rem' }}>
                            {events.map((e, i) => {
                                const isKarma = e._type === 'karma';
                                const isLocal = isKarma ? (e.node_id === selfNodeId || !e.node_id) : true;

                                return (
                                    <motion.div
                                        initial={{ opacity: 0, x: -20 }}
                                        animate={{ opacity: 1, x: 0 }}
                                        transition={{ delay: Math.min(i * 0.05, 2) }}
                                        key={e.id || `timeline-${e._type}-${i}`}
                                        style={{ display: 'flex', gap: '1.5rem', paddingLeft: '0.5rem' }}
                                    >
                                        <div style={{
                                            width: '24px',
                                            height: '24px',
                                            borderRadius: '50%',
                                            background: !isKarma ? 'var(--accent-amber)' : (isLocal ? 'var(--accent-primary)' : 'var(--accent-purple)'),
                                            border: '4px solid var(--bg-primary)',
                                            zIndex: 2,
                                            marginTop: '4px',
                                            boxShadow: !isKarma ? 'var(--glow-amber)' : (isLocal ? 'var(--glow-primary)' : 'var(--glow-purple)')
                                        }} />

                                        <div style={{
                                            flex: 1,
                                            padding: '1.25rem',
                                            borderRadius: 'var(--radius-lg)',
                                            background: !isKarma ? 'var(--accent-amber-05)' : (isLocal ? 'var(--white-03)' : 'var(--accent-purple-05)'),
                                            border: '1px solid var(--border-glass)',
                                            position: 'relative',
                                            overflow: 'hidden'
                                        }}>
                                            <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: '0.75rem', alignItems: 'center' }}>
                                                <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
                                                    <span style={{
                                                        fontSize: '0.7rem',
                                                        fontWeight: 800,
                                                        padding: '0.2rem 0.5rem',
                                                        borderRadius: '4px',
                                                        background: !isKarma ? 'var(--accent-amber-10)' : (isLocal ? 'var(--accent-primary-10)' : 'var(--accent-purple-10)'),
                                                        color: !isKarma ? 'var(--accent-amber)' : (isLocal ? 'var(--accent-primary)' : 'var(--accent-purple)'),
                                                        letterSpacing: '0.1em'
                                                    }}>
                                                        {isKarma ? (isLocal ? t('timeline.localMemory') : t('timeline.federatedMemory')) : t('timeline.evolutionStep')}
                                                    </span>
                                                    <span style={{ fontSize: '0.7rem', color: 'var(--text-muted)' }}>
                                                        {isKarma ? `${(e.karma_type || 'UNKNOWN').toUpperCase()} | JOB #${e.job_id || '?'}` : (e.event_type || 'SYSTEM').toUpperCase()}
                                                    </span>
                                                </div>
                                                <span style={{ fontSize: '0.7rem', color: 'var(--text-muted)' }}>
                                                    {new Date(e.created_at).toLocaleTimeString()}
                                                </span>
                                            </div>

                                            <div style={{ fontSize: '1.05rem', lineHeight: 1.6, color: 'var(--text-primary)' }}>
                                                {isKarma ? e.lesson : e.description}
                                            </div>

                                            {e.inspiration && (
                                                <div style={{ marginTop: '0.5rem', padding: '0.5rem', background: 'var(--white-05)', borderRadius: '4px', fontSize: '0.85rem', color: 'var(--accent-primary)', borderLeft: '2px solid var(--accent-primary)' }}>
                                                    <Sparkles size={12} style={{ marginRight: '0.5rem', verticalAlign: 'middle' }} />
                                                    {e.inspiration}
                                                </div>
                                            )}

                                            <div style={{ marginTop: '0.75rem', display: 'flex', alignItems: 'center', gap: '0.5rem', fontSize: '0.75rem', color: 'var(--text-muted)' }}>
                                                {isKarma ? (e.karma_type === 'Technical' ? <Terminal size={14} /> : <BrainCircuit size={14} />) : <Zap size={14} />}
                                                <span>{new Date(e.created_at).toLocaleDateString()}</span>
                                            </div>
                                        </div>
                                    </motion.div>
                                );
                            })}
                        </div>
                    </div>
                )}
            </div>
        </div>
    );
};

export default Timeline;

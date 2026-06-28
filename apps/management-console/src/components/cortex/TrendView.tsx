/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import { useState, useEffect } from 'react';
import { Activity, AlertTriangle, TrendingUp } from 'lucide-react';
import { authenticatedFetch } from '../../lib/auth';
import { API_BASE } from '../../config';
import { useTranslation } from '../../i18n';

import type { components } from '../../types/generated';

type TrendsResponse = components['schemas']['TrendsResponse'];

export default function TrendView() {
    const { t } = useTranslation();
    const [data, setData] = useState<TrendsResponse>({ trends: [] });
    const [loading, setLoading] = useState(true);
    const [error, setError] = useState<string | null>(null);

    const fetchTrends = async () => {
        try {
            setLoading(true);
            setError(null);
            const res = await authenticatedFetch(`${API_BASE}/api/v1/trends`);
            if (!res.ok) {
                throw new Error(`API Error: ${res.status}`);
            }
            const json: TrendsResponse = await res.json();
            setData(json);
        } catch (e) {
            console.error('Failed to fetch trends', e);
            setError(t('cortexView.loadTrendsFailed'));
        } finally {
            setLoading(false);
        }
    };

    useEffect(() => {
        fetchTrends();
    }, []);

    return (
        <div style={{ display: 'flex', flexDirection: 'column', gap: 'var(--space-md)', height: '100%' }}>
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                <h3 style={{ fontSize: '1.2rem', fontWeight: 600, color: 'var(--text-primary)', display: 'flex', alignItems: 'center', gap: 'var(--space-xs)' }}>
                    <TrendingUp size={20} color="var(--accent-cyan)" />
                    {t('cortexView.activeTrends')}
                </h3>
                <button 
                    className="icon-button" 
                    onClick={fetchTrends} 
                    disabled={loading}
                    title={t('cortexView.refreshTrends')}
                >
                    <Activity size={18} className={loading ? "ani-spin" : ""} />
                </button>
            </div>

            {error && (
                <div style={{ padding: '1rem', background: 'var(--accent-rose-10)', color: 'var(--accent-rose)', border: '1px solid var(--accent-rose-30)', borderRadius: 'var(--radius-md)' }}>
                    {error}
                </div>
            )}

            {data.warnings && data.warnings.length > 0 && (
                <div style={{ padding: '0.75rem', background: 'var(--accent-amber-10)', color: 'var(--accent-amber)', border: '1px solid var(--accent-amber-30)', borderRadius: 'var(--radius-md)', display: 'flex', flexDirection: 'column', gap: '4px' }}>
                    <div style={{ display: 'flex', alignItems: 'center', gap: '6px', fontWeight: 600 }}>
                        <AlertTriangle size={16} /> {t('cortexView.configurationWarnings')}
                    </div>
                    {data.warnings.map(w => (
                        <div key={w} style={{ fontSize: '0.85rem' }}>• {w}</div>
                    ))}
                </div>
            )}

            <div style={{ display: 'flex', flexDirection: 'column', gap: 'var(--space-sm)', overflowY: 'auto', flex: 1 }}>
                {loading && data.trends.length === 0 && (
                    <div className="ani-pulse" style={{ color: 'var(--accent-cyan)' }}>{t('cortexView.scanningTrends')}</div>
                )}
                
                {!loading && data.trends.length === 0 && !error && (
                    <div style={{ color: 'var(--text-muted)', fontSize: '0.85rem' }}>{t('cortexView.noActiveTrends')}</div>
                )}

                {data.trends.map((item, idx) => (
                    <div key={idx} className="stat-card" style={{ padding: 'var(--space-sm)', background: 'var(--bg-glass-light)', border: '1px solid var(--border-glass)', borderRadius: 'var(--radius-sm)' }}>
                        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                            <span style={{ fontWeight: 600, color: 'var(--text-primary)' }}>{item.keyword}</span>
                            <span style={{ fontSize: '0.75rem', padding: '0.1rem 0.4rem', background: 'var(--accent-cyan-glass)', color: 'var(--accent-cyan)', borderRadius: 'var(--radius-xl)' }}>
                                {item.source}
                            </span>
                        </div>
                        <div style={{ marginTop: '0.5rem' }}>
                            <div style={{ width: '100%', height: '4px', background: 'var(--bg-glass-heavy)', borderRadius: '2px', overflow: 'hidden' }}>
                                <div style={{ height: '100%', width: `${Math.min(item.score * 100, 100)}%`, background: 'var(--accent-cyan)' }} />
                            </div>
                        </div>
                    </div>
                ))}
            </div>
        </div>
    );
}

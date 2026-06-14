import { useEffect, useState } from 'react';
import { useTranslation } from '../i18n';
import { useSystemVitality } from '../hooks/useSystemVitality';
import { authenticatedFetch } from '../lib/auth';
import { SidecarHealth } from '../types';
import { API_BASE } from '../config';

/** Quality Gate 履歴エントリの型定義（バックエンド QualityGateEntry と同期） */
interface QualityGateEvent {
    id?: string;
    job_id?: string;
    score?: number;
    passed?: boolean;
    conductor?: string;
    created_at?: string;
}

/** 安全な日付文字列→表示変換。不正な値は空文字を返す */
const safeTimeString = (raw: string | undefined): string => {
    if (!raw) return '';
    const d = new Date(raw);
    return isNaN(d.getTime()) ? '' : d.toLocaleTimeString();
};

export default function SeoPulseView() {
    const { t } = useTranslation();
    const { events } = useSystemVitality();
    const [geoOptimizerStatus, setGeoOptimizerStatus] = useState<SidecarHealth | null>(null);
    const [history, setHistory] = useState<QualityGateEvent[]>([]);
    const [currentViseme, setCurrentViseme] = useState<string | null>(null);

    useEffect(() => {
        let timeoutId: ReturnType<typeof setTimeout>;
        const handleViseme = (e: Event) => {
            const customEvent = e as CustomEvent;
            if (customEvent.detail?.viseme) {
                setCurrentViseme(customEvent.detail.viseme);
                clearTimeout(timeoutId);
                timeoutId = setTimeout(() => setCurrentViseme(null), 150);
            }
        };
        window.addEventListener('aiome_viseme_played', handleViseme);
        return () => {
            window.removeEventListener('aiome_viseme_played', handleViseme);
            clearTimeout(timeoutId);
        };
    }, []);

    useEffect(() => {
        const fetchStatus = async () => {
            try {
                // auth-exempt, standard fetch
                const res = await fetch(`${API_BASE}/api/v1/bootstrap/status`);
                if (res.ok) {
                    const data = await res.json();
                    if (data && Array.isArray(data.sidecar_status)) {
                        const geo = data.sidecar_status.find((s: SidecarHealth) => s.name === 'geo-optimizer');
                        if (geo) {
                            setGeoOptimizerStatus(geo);
                        }
                    }
                }
            } catch (err) {
                console.error("Failed to fetch bootstrap status for GEO optimizer", err);
            }
        };
        const fetchHistory = async () => {
            try {
                const res = await authenticatedFetch(`${API_BASE}/api/v1/quality-gate/history?limit=10`);
                if (res.ok) {
                    const data = await res.json();
                    setHistory(Array.isArray(data) ? data : []);
                }
            } catch (err) {
                console.error("Failed to fetch quality gate history", err);
            }
        };
        fetchStatus();
        fetchHistory();
    }, []);

    // Combine unique events (by job_id or just append latest live ones to history)
    const liveEvents: QualityGateEvent[] = events
        .filter(e => e.type === 'quality_gate')
        .map(e => e.data as QualityGateEvent);
    
    // Create a deduplicated list prioritizing live events over history
    // Live events overwrite history entries with the same key
    const allEventsMap = new Map<string, QualityGateEvent>();
    for (const ev of [...history, ...liveEvents]) {
        const key = ev.job_id || ev.id;
        if (key) {
            allEventsMap.set(String(key), ev);
        }
        // Events without job_id AND id are silently dropped (data integrity guard)
    }
    
    const combinedEvents = Array.from(allEventsMap.values())
        .sort((a, b) => {
            const timeA = a.created_at ? new Date(a.created_at).getTime() : 0;
            const timeB = b.created_at ? new Date(b.created_at).getTime() : 0;
            return timeB - timeA;
        })
        .slice(0, 10);

    return (
        <div className="main-panel ani-fade" style={{ flex: 1, display: 'flex', flexDirection: 'column', marginTop: 'var(--space-md)' }}>
            <div className="panel-header">
                <div style={{ display: 'flex', alignItems: 'center', gap: 'var(--space-xs)' }}>
                    <span style={{ fontSize: 'var(--font-size-lg)' }}>🌍</span>
                    <h3 className="artemis-heading" style={{ margin: 0, fontSize: 'var(--font-size-lg)' }}>GEO Pulse</h3>
                </div>
                <div style={{ fontSize: 'var(--font-size-base)', color: 'var(--text-secondary)' }}>
                    Status: {geoOptimizerStatus ? (
                        <span style={{ fontWeight: 600, color: geoOptimizerStatus.status === 'ok' ? 'var(--accent-emerald)' : 'var(--accent-rose)' }}>
                            {geoOptimizerStatus.status}
                        </span>
                    ) : (
                        '...'
                    )}
                </div>
            </div>

            <div style={{ padding: 'var(--space-md)', display: 'flex', flexDirection: 'column', gap: 'var(--space-md)', overflowY: 'auto' }}>
                {/* Viseme Visualizer */}
                <div className="glass-panel" style={{ padding: 'var(--space-sm)', borderRadius: 'var(--radius-md)', border: '1px solid var(--border-glass)' }}>
                    <div style={{ fontSize: 'var(--font-size-2xs)', fontWeight: 600, color: 'var(--text-muted)', marginBottom: 'var(--space-xs)', textTransform: 'uppercase', letterSpacing: '0.1em' }}>
                        Procedural Lip-Sync
                    </div>
                    <div style={{ display: 'flex', alignItems: 'center', gap: 'var(--space-sm)' }}>
                        <div style={{ fontSize: 'var(--font-size-sm)', color: 'var(--text-secondary)' }}>Active Viseme:</div>
                        <div className="chip active" style={{ 
                            padding: 'var(--space-2xs) var(--space-xs)', 
                            fontSize: 'var(--font-size-xs)',
                            borderRadius: 'var(--radius-sm)',
                            transform: currentViseme ? 'scale(1.05)' : 'none',
                            transition: 'all 0.075s'
                        }}>
                            {currentViseme || 'SIL'}
                        </div>
                        {/* Visual mouth shape indicator */}
                        <div style={{ flex: 1, display: 'flex', justifyContent: 'center', alignItems: 'center', height: '2rem' }}>
                            <div style={{ 
                                transition: 'all 0.1s', 
                                borderRadius: '50%', 
                                border: '2px solid var(--accent-primary)',
                                width: currentViseme === 'AA' ? '1.5rem' :
                                       currentViseme === 'IH' ? '2rem' :
                                       currentViseme === 'OU' ? '0.75rem' :
                                       currentViseme === 'EE' ? '1.75rem' :
                                       currentViseme === 'OH' ? '1rem' : '1rem',
                                height: currentViseme === 'AA' ? '1.5rem' :
                                        currentViseme === 'IH' ? '0.5rem' :
                                        currentViseme === 'OU' ? '0.75rem' :
                                        currentViseme === 'EE' ? '0.5rem' :
                                        currentViseme === 'OH' ? '1.25rem' : '0.25rem',
                                borderColor: currentViseme ? 'var(--accent-primary)' : 'var(--border-glass)',
                                boxShadow: currentViseme ? 'var(--glow-primary)' : 'none'
                            }} />
                        </div>
                    </div>
                </div>
                
                <div style={{ display: 'flex', flexDirection: 'column', gap: 'var(--space-xs)' }}>
                    {combinedEvents.length === 0 ? (
                        <div style={{ color: 'var(--text-muted)', fontStyle: 'italic', fontSize: 'var(--font-size-sm)' }}>
                            {t('seoPulse.noEvents', { defaultValue: 'No recent audits...' }) as string}
                        </div>
                    ) : (
                        combinedEvents.map((ev) => (
                            <div key={ev.id || ev.job_id} className="glass-panel" style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', padding: 'var(--space-xs) var(--space-sm)', borderRadius: 'var(--radius-sm)', border: '1px solid var(--border-glass)' }}>
                                <span style={{ fontSize: 'var(--font-size-sm)', fontWeight: 500, color: 'var(--text-primary)' }}>{ev.conductor || 'Audit'}</span>
                                <div style={{ display: 'flex', alignItems: 'center', gap: 'var(--space-md)' }}>
                                    <span style={{ fontSize: 'var(--font-size-sm)', fontWeight: 600, color: ev.passed ? 'var(--accent-emerald)' : 'var(--accent-amber)' }}>
                                        Score: {ev.score != null ? ev.score : '—'}
                                    </span>
                                    {ev.created_at && safeTimeString(ev.created_at) && (
                                        <span style={{ fontSize: 'var(--font-size-2xs)', color: 'var(--text-muted)' }}>
                                            {safeTimeString(ev.created_at)}
                                        </span>
                                    )}
                                </div>
                            </div>
                        ))
                    )}
                </div>
            </div>
        </div>
    );
};

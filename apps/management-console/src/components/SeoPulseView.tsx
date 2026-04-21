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
        <div className="bg-gradient-to-br from-[var(--surface-color)] to-[var(--surface-color-secondary)] p-4 rounded-xl border border-[var(--border-color)] mt-4 shadow-xl">
            <h2 className="text-xl font-bold text-[var(--accent-color)] mb-2 flex items-center">
                <span className="mr-2 text-2xl">🌍</span>
                GEO Pulse
            </h2>
            <div className="mb-4 text-sm text-[var(--text-color-secondary)]">
                Status: {geoOptimizerStatus ? (
                    <span className={`font-semibold ${geoOptimizerStatus.status === 'ok' ? 'text-[var(--success-color)]' : 'text-[var(--danger-color)]'}`}>
                        {geoOptimizerStatus.status}
                    </span>
                ) : (
                    '...'
                )}
            </div>
            
            <div className="space-y-2">
                {combinedEvents.length === 0 ? (
                    <div className="text-[var(--text-color-secondary)] italic text-sm">
                        {t('seoPulse.noEvents', { defaultValue: 'No recent audits...' }) as string}
                    </div>
                ) : (
                    combinedEvents.map((ev) => (
                        <div key={ev.id || ev.job_id} className="flex justify-between items-center p-2 rounded bg-[var(--background-color)]">
                            <span className="text-sm font-medium">{ev.conductor || 'Audit'}</span>
                            <div className="flex items-center space-x-3">
                                <span className={`text-sm ${ev.passed ? 'text-[var(--success-color)]' : 'text-[var(--warning-color)]'}`}>
                                    Score: {ev.score != null ? ev.score : '—'}
                                </span>
                                {ev.created_at && safeTimeString(ev.created_at) && (
                                    <span className="text-xs text-[var(--text-color-secondary)]">
                                        {safeTimeString(ev.created_at)}
                                    </span>
                                )}
                            </div>
                        </div>
                    ))
                )}
            </div>
        </div>
    );
};

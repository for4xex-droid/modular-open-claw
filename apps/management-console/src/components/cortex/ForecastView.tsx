import { useState, useEffect } from 'react';
import { LineChart, Sparkles, AlertCircle } from 'lucide-react';
import { useTranslation } from '../../i18n';
import { authenticatedFetch } from '../../lib/auth';
import { API_BASE } from '../../config';
import type { components } from '../../types/generated';

type ForecastResponse = components['schemas']['ForecastResponse'];

export default function ForecastView() {
    const { t } = useTranslation();
    const [data, setData] = useState<ForecastResponse | null>(null);
    const [loading, setLoading] = useState(true);
    const [error, setError] = useState<string | null>(null);

    const fetchForecast = async () => {
        try {
            setLoading(true);
            setError(null);
            const res = await authenticatedFetch(`${API_BASE}/api/v1/forecast/predict?series_id=karma_trend`);
            if (!res.ok) {
                throw new Error(`API Error: ${res.status}`);
            }
            const json: ForecastResponse = await res.json();
            setData(json);
        } catch (e) {
            console.error('Failed to fetch forecast', e);
            setError(t('forecast.error'));
        } finally {
            setLoading(false);
        }
    };

    useEffect(() => {
        fetchForecast();
    }, []);

    // Helper to calculate max for simple bar chart rendering
    const maxValue = data ? Math.max(...data.values, 1) : 1;

    return (
        <div style={{ display: 'flex', flexDirection: 'column', gap: 'var(--space-md)', height: '100%' }}>
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                <h3 style={{ fontSize: '1.2rem', fontWeight: 600, color: 'var(--text-primary)', display: 'flex', alignItems: 'center', gap: 'var(--space-xs)' }}>
                    <Sparkles size={20} color="var(--accent-purple)" />
                    {t('forecast.title')}
                </h3>
                <button 
                    className="icon-button" 
                    onClick={fetchForecast} 
                    disabled={loading}
                    data-tooltip={t('forecast.refresh')}
                >
                    <LineChart size={18} className={loading ? "ani-pulse" : ""} />
                </button>
            </div>

            {error && (
                <div style={{ padding: '1rem', background: 'var(--accent-rose-10)', color: 'var(--accent-rose)', border: '1px solid var(--accent-rose-30)', borderRadius: 'var(--radius-md)', display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
                    <AlertCircle size={18} />
                    {error}
                </div>
            )}

            <div style={{ flex: 1, background: 'var(--bg-glass-light)', border: '1px solid var(--border-glass)', borderRadius: 'var(--radius-md)', padding: '1rem', display: 'flex', flexDirection: 'column', gap: '1rem' }}>
                {loading && !data && (
                    <div style={{ color: 'var(--accent-purple)', display: 'flex', justifyContent: 'center', alignItems: 'center', height: '100%', width: '100%' }}>
                        <span className="ani-pulse">{t('forecast.loading')}</span>
                    </div>
                )}
                
                {data && (
                    <div style={{ display: 'flex', alignItems: 'flex-end', height: '100%', gap: '4px', paddingTop: '1rem' }}>
                        {data.values.map((val, idx) => {
                            const heightPct = (val / maxValue) * 100;
                            return (
                                <div key={idx} style={{ flex: 1, display: 'flex', flexDirection: 'column', alignItems: 'center', gap: '4px', height: '100%', justifyContent: 'flex-end' }}>
                                    <div style={{ fontSize: '0.65rem', color: 'var(--text-muted)' }}>{Math.round(val)}</div>
                                    <div 
                                        style={{ 
                                            width: '100%', 
                                            height: `${heightPct}%`, 
                                            background: 'linear-gradient(to top, var(--accent-purple-30), var(--accent-purple))',
                                            borderRadius: '2px 2px 0 0',
                                            minHeight: '4px'
                                        }} 
                                        data-tooltip={new Date(data.timestamps[idx]).toLocaleString()}
                                    />
                                </div>
                            );
                        })}
                    </div>
                )}
            </div>
            
            <div style={{ fontSize: '0.75rem', color: 'var(--text-muted)', textAlign: 'right' }}>
                Powered by OxiLean TimesFM Sidecar
            </div>
        </div>
    );
}

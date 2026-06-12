import React, { useState, useEffect } from 'react';
import { Loader2 } from 'lucide-react';
import { useTranslation } from '../i18n';
import { authenticatedFetch } from '../lib/auth';
import { API_BASE } from '../config';

const labelStyle = {
    display: 'block',
    fontSize: '0.85rem',
    fontWeight: 600,
    marginBottom: '0.5rem',
    color: 'var(--text-secondary)'
};

const inputStyle = {
    width: '100%',
    padding: '0.8rem',
    background: 'var(--bg-tertiary)',
    border: '1px solid var(--border-glass)',
    borderRadius: '8px',
    color: 'var(--text-primary)',
    fontSize: '0.9rem',
    transition: 'border-color 0.2s',
};

const testBtnStyle = {
    padding: '0.8rem 1.5rem',
    background: 'var(--white-05)',
    border: '1px solid var(--white-10)',
    borderRadius: '8px',
    color: 'var(--text-primary)',
    fontSize: '0.85rem',
    fontWeight: 600,
    cursor: 'pointer',
    display: 'flex',
    alignItems: 'center',
    gap: '0.5rem',
    transition: 'all 0.2s ease'
};

export const OllamaModelSelector: React.FC<{ value: string, onSelect: (v: string) => void, saving?: boolean }> = ({ value, onSelect, saving }) => {
    const { t } = useTranslation();
    const [models, setModels] = useState<string[]>([]);
    const [loading, setLoading] = useState(false);
    const [error, setError] = useState('');

    useEffect(() => {
        fetchModels();
    }, []);

    const fetchModels = async () => {
        setLoading(true);
        setError('');
        try {
            const res = await authenticatedFetch(`${API_BASE}/api/v1/ollama/models`);
            if (res.ok) {
                const data = await res.json();
                if (data.models && Array.isArray(data.models)) {
                    setModels(data.models.map((m: any) => m.name));
                }
            } else {
                setError(`Failed to fetch models: ${res.status}`);
            }
        } catch (err: unknown) {
            console.error("Fetch models error:", err);
            setError(`Connection error: ${err instanceof Error ? err.message : 'Unknown error'}`);
        } finally {
            setLoading(false);
        }
    };

    return (
        <div>
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '0.6rem' }}>
                <label style={{ ...labelStyle, marginBottom: 0 }}>{t('settings.ollamaModel')}</label>
                {saving && <Loader2 size={12} className="ani-spin" color="var(--accent-cyan)" />}
            </div>
            <div style={{ display: 'flex', gap: 'var(--space-xs)' }}>
                <select
                    value={value}
                    onChange={(e) => onSelect(e.target.value)}
                    style={{ ...inputStyle, flex: 1, padding: '0.67rem', outline: 'none' }}
                >
                    <option value="" style={{ background: 'var(--bg-primary)' }}>{t('settings.ollamaPlaceholder')}</option>
                    {models.map(m => (
                        <option key={m} value={m} style={{ background: 'var(--bg-primary)' }}>{m}</option>
                    ))}
                    {!models.includes(value) && value && (
                        <option value={value} style={{ background: 'var(--bg-primary)' }}>{value} {t('settings.current')}</option>
                    )}
                </select>
                <button onClick={fetchModels} disabled={loading} data-tooltip="Refresh Models" style={{ ...testBtnStyle, padding: '0.5rem 0.8rem' }}>
                    {loading ? <Loader2 size={14} className="ani-spin" /> : t('settings.refresh')}
                </button>
            </div>
            {error && <div style={{ fontSize: '0.7rem', color: 'var(--accent-rose)', marginTop: '0.4rem' }}>{error}</div>}
        </div>
    );
};

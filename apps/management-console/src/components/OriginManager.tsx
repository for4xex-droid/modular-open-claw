import React, { useState } from 'react';
import { Loader2, Plus, X } from 'lucide-react';
import { useTranslation } from '../i18n';

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

export const OriginManager: React.FC<{ value: string, onUpdate: (v: string) => void, saving?: boolean }> = ({ value, onUpdate, saving }) => {
    const { t } = useTranslation();
    const [draft, setDraft] = useState('');
    const [error, setError] = useState('');
    const items = value ? value.split(',').map(s => s.trim()).filter(Boolean) : [];

    const addOrigin = () => {
        if (!draft.trim()) return;
        if (items.includes(draft.trim())) {
            setError(t('settings.originExists'));
            return;
        }
        const updated = [...items, draft.trim()].join(',');
        onUpdate(updated);
        setDraft('');
        setError('');
    };

    const removeOrigin = (idx: number) => {
        const updated = items.filter((_, i) => i !== idx).join(',');
        onUpdate(updated);
    };

    return (
        <div>
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '0.6rem' }}>
                <label style={{ ...labelStyle, marginBottom: 0 }}>{t('settings.allowedOrigins')}</label>
                {saving && <Loader2 size={12} className="ani-spin" color="var(--accent-cyan)" />}
            </div>
            <div style={{ display: 'flex', flexWrap: 'wrap', gap: 'var(--space-xs)', marginBottom: '0.6rem' }}>
                {items.map((item, i) => (
                    <div key={i} style={{
                        display: 'flex', alignItems: 'center', gap: '0.4rem',
                        background: 'var(--accent-cyan-glass)', border: '1px solid var(--accent-cyan-20)',
                        borderRadius: '6px', padding: '0.3rem 0.6rem', fontSize: '0.75rem',
                        color: 'var(--accent-cyan)'
                    }}>
                        <span>{item}</span>
                        <X size={12} style={{ cursor: 'pointer', opacity: 0.6 }} onClick={() => removeOrigin(i)} />
                    </div>
                ))}
            </div>
            <div style={{ display: 'flex', gap: '0.5rem' }}>
                <input
                    type="text" value={draft} placeholder="https://example.com"
                    onChange={(e) => { setDraft(e.target.value); setError(''); }}
                    onKeyDown={(e) => { if (e.nativeEvent.isComposing) return; if (e.key === 'Enter') addOrigin(); }}
                    style={{ ...inputStyle, flex: 1 }}
                />
                <button onClick={addOrigin} style={{ ...testBtnStyle, padding: '0.5rem 0.8rem' }}>
                    <Plus size={14} /> {t('settings.add')}
                </button>
            </div>
            {error && <div style={{ fontSize: '0.7rem', color: 'var(--accent-rose)', marginTop: '0.4rem' }}>{error}</div>}
            <div style={{ fontSize: '0.6rem', color: 'var(--text-muted)', marginTop: '0.4rem', fontStyle: 'italic' }}>
                {t('settings.serverRestartRequired')}
            </div>
        </div>
    );
};

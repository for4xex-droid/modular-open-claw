/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import React, { useState, useEffect } from 'react';
import { Loader2, Database, RefreshCw } from 'lucide-react';
import { useTranslation } from '../i18n';
import { authenticatedFetch } from '../lib/auth';
import { API_BASE } from '../config';

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

export const McpConfigManager: React.FC = () => {
    const { t } = useTranslation();
    const [configJson, setConfigJson] = useState('{\n  "mcp_servers": {}\n}');
    const [loading, setLoading] = useState(true);
    const [saving, setSaving] = useState(false);
    const [message, setMessage] = useState('');

    useEffect(() => {
        fetchConfig();
    }, []);

    const fetchConfig = async () => {
        try {
            const res = await authenticatedFetch(`${API_BASE}/api/skills/mcp/config`);
            if (res.ok) {
                const data = await res.json();
                setConfigJson(JSON.stringify(data, null, 2));
            }
        } catch (e) {
            console.error(e);
        } finally {
            setLoading(false);
        }
    };

    const saveConfig = async () => {
        try {
            setSaving(true);
            setMessage('');
            JSON.parse(configJson); 
            const res = await authenticatedFetch(`${API_BASE}/api/skills/mcp/config`, {
                method: 'PUT',
                headers: { 'Content-Type': 'application/json' },
                body: configJson
            });
            if (res.ok) {
                setMessage(`✅ ${t('settings.reloadedSuccessfully', { defaultValue: 'Reloaded successfully' })}`);
                setTimeout(() => setMessage(''), 3000);
            } else {
                setMessage(`❌ ${t('settings.errorSaving', { defaultValue: 'Error saving' })}`);
            }
        } catch (e) {
            setMessage(`❌ ${t('settings.invalidJson', { defaultValue: 'Invalid JSON or network error' })}`);
        } finally {
            setSaving(false);
        }
    };

    return (
        <section className="glass-panel" style={{ padding: 'var(--space-lg)', borderRadius: 'var(--radius-lg)' }}>
            <div style={{ display: 'flex', alignItems: 'center', gap: '1rem', marginBottom: '1rem' }}>
                <Database size={24} color="var(--accent-amber)" />
                <h3 style={{ margin: 0, fontSize: '1.2rem' }}>{t('settings.mcpArchitecture', { defaultValue: 'MCP Architecture (Analytics & Tools)' })}</h3>
            </div>
            {loading ? <div style={{ padding: '2rem', textAlign: 'center' }}><Loader2 className="ani-spin" size={24} color="var(--accent-amber)" /></div> : (
                <div style={{ display: 'flex', flexDirection: 'column', gap: '1rem' }}>
                    <div style={{ fontSize: '0.85rem', color: 'var(--text-muted)' }}>
                        {t('settings.mcpDesc', { defaultValue: 'Define external MCP servers (GA4, Stripe, etc). Safe to use environment variables like $STRIPE_SECRET_KEY. Saving will restart MCP processes dynamically.' })}
                    </div>
                    <textarea 
                        className="font-mono"
                        value={configJson}
                        onChange={e => setConfigJson(e.target.value)}
                        style={{
                            width: '100%', height: '200px', background: 'var(--black-50)', color: 'var(--accent-cyan)',
                            padding: '1rem', borderRadius: 'var(--radius-md)', border: '1px solid var(--border-glass)',
                            resize: 'vertical', outline: 'none', fontSize: '0.85rem'
                        }}
                    />
                    <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                        <span style={{ fontSize: '0.85rem', color: message.includes('❌') ? 'var(--accent-rose)' : 'var(--accent-emerald)', fontWeight: 600 }}>
                            {message}
                        </span>
                        <button onClick={saveConfig} disabled={saving} style={{ ...testBtnStyle, background: 'var(--accent-amber-20)', borderColor: 'var(--accent-amber-30)', color: 'var(--accent-amber)' }}>
                            {saving ? <Loader2 size={16} className="ani-spin" /> : <RefreshCw size={16} />}
                            {t('settings.saveSyncTools', { defaultValue: 'Save & Sync Tools' })}
                        </button>
                    </div>
                </div>
            )}
        </section>
    );
};

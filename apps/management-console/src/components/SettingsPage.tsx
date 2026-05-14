/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import React, { useState, useEffect } from 'react';
import { useAvatarCharacter } from '../hooks/AvatarContext';
import { useTranslation } from '../i18n';
import { useDisplayMode } from '../hooks/useDisplayMode';
import { useViewMode, type ViewMode } from '../hooks/useViewMode';
import {
    Monitor, Lock, Database,
    Shield, Check, X, Loader2, Plus, Share2, AlertTriangle
} from 'lucide-react';
import { API_BASE } from '../config';
import { setAuthToken, authenticatedFetch, clearAuthToken } from '../lib/auth';
import EscrowManagementView from './EscrowManagementView';

interface SettingEntry {
    key: string;
    value: string;
    category: string;
    is_secret: boolean;
    updated_at: string;
}

const SettingsPage: React.FC = () => {
    const { character, setCharacter, proportion, setProportion } = useAvatarCharacter();
    const { mode, setMode } = useDisplayMode();
    const { viewMode, setViewMode } = useViewMode();
    const { t } = useTranslation();
    const [settings, setSettings] = useState<SettingEntry[]>([]);
    const [loading, setLoading] = useState(true);
    const [saving, setSaving] = useState<string | null>(null);
    const [testResults, setTestResults] = useState<Record<string, { success: boolean, message: string, loading: boolean }>>({});
    const [globalError, setGlobalError] = useState<string | null>(null);

    useEffect(() => {
        fetchSettings();
    }, []);

    const fetchSettings = async () => {
        try {
            const res = await authenticatedFetch(`${API_BASE}/api/v1/settings`);
            if (res.ok) {
                const data = await res.json();
                if (Array.isArray(data)) {
                    setSettings(data);
                } else {
                    console.error("Unexpected settings response format:", typeof data);
                    setSettings([]);
                }
            }
        } catch (error) {
            console.error("Failed to fetch settings", error);
        } finally {
            setLoading(false);
        }
    };

    const updateSetting = async (key: string, value: string, category: string) => {
        setSaving(key);
        try {
            setGlobalError(null);
            const res = await authenticatedFetch(`${API_BASE}/api/v1/settings`, {
                method: 'PUT',
                body: JSON.stringify({ key, value, category })
            });
            if (res.ok) {
                setSettings(prev => {
                    if (prev.some(s => s.key === key)) {
                        return prev.map(s => s.key === key ? { ...s, value, updated_at: new Date().toISOString() } : s);
                    } else {
                        return [...prev, { key, value, category, is_secret: false, updated_at: new Date().toISOString() }];
                    }
                });
            } else {
                const text = await res.text();
                setGlobalError(`Failed to save setting: ${text}`);
            }
        } catch (error) {
            console.error("Failed to update setting", error);
            setGlobalError(String(error));
        } finally {
            setTimeout(() => setSaving(null), 500);
        }
    };

    const testConnection = async (service: string, url: string, model?: string) => {
        if (!url) {
            setTestResults(prev => ({ ...prev, [service]: { success: false, message: t('settings.urlRequired'), loading: false } }));
            return;
        }
        setTestResults(prev => ({ ...prev, [service]: { success: false, message: '', loading: true } }));
        try {
            const res = await authenticatedFetch(`${API_BASE}/api/v1/settings/test`, {
                method: 'POST',
                body: JSON.stringify({ service, url, model })
            });
            if (res.status === 404) {
                setTestResults(prev => ({ ...prev, [service]: { success: false, message: t('settings.testDisabledInProd', { defaultValue: 'Connection testing is disabled in production to protect against SSRF reconnaissance.' }) as string, loading: false } }));
                return;
            }
            const data = await res.json();
            setTestResults(prev => ({ ...prev, [service]: { success: data.success, message: data.message, loading: false } }));
        } catch (error) {
            setTestResults(prev => ({ ...prev, [service]: { success: false, message: t('settings.connectionFailed'), loading: false } }));
        }
    };

    const getSetting = (key: string) => settings.find(s => s.key === key)?.value || "";

    if (loading) {
        return (
            <div style={{ display: 'flex', justifyContent: 'center', alignItems: 'center', height: '50vh' }}>
                <Loader2 className="ani-spin" size={40} color="var(--accent-cyan)" />
            </div>
        );
    }

    const update_setting_handler = (val: string, k: string, cat: string) => updateSetting(k, val, cat);

    return (
        <div className="settings-page">
            {globalError && (
                <div style={{ display: 'flex', alignItems: 'center', gap: '8px', padding: '1rem', marginBottom: '1.5rem', backgroundColor: 'var(--accent-rose-10)', color: 'var(--accent-rose)', border: '1px solid var(--accent-rose-30)', borderRadius: 'var(--radius-md)' }}>
                    <AlertTriangle size={20} />
                    {globalError}
                </div>
            )}
            <div className="settings-grid">

                {/* 1. Appearance Section */}
                <section className="glass-panel" style={{ padding: 'var(--space-lg)', borderRadius: 'var(--radius-lg)' }}>
                    <div style={{ display: 'flex', alignItems: 'center', gap: '1rem', marginBottom: '2rem' }}>
                        <Monitor size={24} color="var(--accent-cyan)" />
                        <h3 style={{ margin: 0, fontSize: '1.2rem' }}>{t('settings.appearance')}</h3>
                    </div>

                    <div style={{ display: 'flex', flexDirection: 'column', gap: '2rem' }}>
                        <div>
                            <label style={labelStyle}>{t('settings.aiName')}</label>
                            <input
                                type="text"
                                value={getSetting('ai_name')}
                                placeholder={t('settings.aiNamePlaceholder', { defaultValue: 'Watchtower' }) as string}
                                onChange={(e) => updateSetting('ai_name', e.target.value, 'identity')}
                                style={inputStyle}
                            />
                        </div>

                        <div>
                            <label style={labelStyle}>{t('settings.avatarCharacter')}</label>
                            <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '1rem' }}>
                                <div onClick={() => setCharacter('female')} style={charCardStyle(character === 'female', 'purple')}>
                                    <div style={{ fontSize: '1.5rem', marginBottom: '0.5rem' }}>♀</div>
                                    <div style={{ fontSize: '0.9rem', fontWeight: 600 }}>{t('settings.female')}</div>
                                </div>
                                <div onClick={() => setCharacter('male')} style={charCardStyle(character === 'male', 'cyan')}>
                                    <div style={{ fontSize: '1.5rem', marginBottom: '0.5rem' }}>♂</div>
                                    <div style={{ fontSize: '0.9rem', fontWeight: 600 }}>{t('settings.male')}</div>
                                </div>
                            </div>
                        </div>

                        <div>
                            <label style={labelStyle}>{t('settings.avatarStyle')}</label>
                            <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '1rem' }}>
                                <div onClick={() => setProportion('chibi')} style={styleCardStyle(proportion === 'chibi', character)}>
                                    {t('settings.cuteChibi')}
                                </div>
                                <div onClick={() => setProportion('taller')} style={styleCardStyle(proportion === 'taller', character)}>
                                    {t('settings.modernTaller')}
                                </div>
                            </div>
                        </div>

                        <div>
                            <label style={labelStyle}>{t('settings.displayMode')}</label>
                            <div style={{ fontSize: '0.75rem', color: 'var(--text-muted)', marginBottom: '0.8rem' }}>
                                {t('settings.displayModeHelp', { defaultValue: 'Choose rendering fidelity. VRM requires GPU.' })}
                            </div>
                            <div style={{ display: 'flex', gap: '0.3rem', background: 'var(--white-05)', padding: '4px', borderRadius: '10px' }}>
                                {['vrm', 'lite', 'off'].map((m) => (
                                    <button
                                        key={m}
                                        onClick={() => setMode(m as any)}
                                        style={modeBtnStyle(mode === m)}
                                    >
                                        {m === 'vrm' ? '🌟 ' : m === 'lite' ? '⚡ ' : '🚫 '}{m}
                                    </button>
                                ))}
                            </div>
                        </div>

                        <div>
                            <label style={labelStyle}>{t('settings.interfaceComplexity')}</label>
                            <div style={{ fontSize: '0.75rem', color: 'var(--text-muted)', marginBottom: '0.8rem' }}>
                                {t('settings.interfaceComplexityHelp', { defaultValue: 'Adjusts available settings and logs based on your experience level.' })}
                            </div>
                            <div style={{ display: 'flex', gap: '0.3rem', background: 'var(--white-05)', padding: '4px', borderRadius: '10px' }}>
                                {['beginner', 'intermediate', 'advanced'].map((m) => (
                                    <button
                                        key={m}
                                        onClick={() => setViewMode(m as ViewMode)}
                                        style={{ ...modeBtnStyle(viewMode === m), textTransform: 'capitalize' as const }}
                                    >
                                        {t(`settings.viewMode_${m}`)}
                                    </button>
                                ))}
                            </div>
                        </div>
                    </div>
                </section>

                {/* 2. LLM Configuration Section */}
                <section className="glass-panel" style={{ padding: 'var(--space-lg)', borderRadius: 'var(--radius-lg)' }}>
                    <div style={{ display: 'flex', alignItems: 'center', gap: '1rem', marginBottom: '2rem' }}>
                        <Database size={24} color="var(--accent-purple)" />
                        <h3 style={{ margin: 0, fontSize: '1.2rem' }}>{t('settings.llmEngine')}</h3>
                    </div>

                    <div style={{ display: 'flex', flexDirection: 'column', gap: '1.5rem' }}>
                        <div>
                            <label style={labelStyle}>{t('settings.llmProvider')}</label>
                            <select
                                value={getSetting('llm_provider') || 'ollama'}
                                onChange={(e) => update_setting_handler(e.target.value, 'llm_provider', 'llm')}
                                style={selectStyle}
                            >
                                <option value="ollama" style={{ background: 'var(--bg-primary)' }}>Ollama (Local)</option>
                                <option value="lmstudio" style={{ background: 'var(--bg-primary)' }}>LM Studio (Local)</option>
                                <option value="gemini" style={{ background: 'var(--bg-primary)' }}>Google Gemini (Cloud)</option>
                                <option value="openai" style={{ background: 'var(--bg-primary)' }}>OpenAI (Cloud)</option>
                                <option value="claude" style={{ background: 'var(--bg-primary)' }}>Anthropic Claude (Cloud)</option>
                            </select>
                        </div>

                        {getSetting('llm_provider') === 'ollama' && (
                            <OllamaModelSelector 
                                value={getSetting('ollama_model')} 
                                onSelect={(v) => updateSetting('ollama_model', v, 'llm')}
                                saving={saving === 'ollama_model'}
                            />
                        )}

                        <SettingInput 
                            label={t('settings.apiUrl')} 
                            value={getSetting('llm_api_url')}
                            placeholder="e.g. http://localhost:11434"
                            onBlur={(v) => updateSetting('llm_api_url', v, 'llm')}
                            saving={saving === 'llm_api_url'}
                        />

                        <SettingInput 
                            label={t('settings.apiKey')} 
                            value={getSetting('llm_api_key')}
                            placeholder={t('settings.optionalApiKey', { defaultValue: 'Optional API Key' }) as string}
                            onBlur={(v) => updateSetting('llm_api_key', v, 'llm')}
                            saving={saving === 'llm_api_key'}
                            isPassword
                        />

                        <button 
                            onClick={() => testConnection('llm', getSetting('llm_api_url'), getSetting('ollama_model'))}
                            disabled={testResults['llm']?.loading}
                            style={testBtnStyle}
                        >
                            {testResults['llm']?.loading ? <Loader2 size={16} className="ani-spin" /> : <Shield size={16} />}
                            {t('settings.testLlmConnection')}
                        </button>
                        {testResults['llm'] && (
                            <div style={testResultStyle(testResults['llm'].success)}>
                                {testResults['llm'].success ? <Check size={14} /> : <X size={14} />}
                                {testResults['llm'].message}
                            </div>
                        )}
                    </div>
                </section>

                {/* Commerce Integration Section */}
                {viewMode !== 'beginner' && (
                <section className="glass-panel" style={{ padding: 'var(--space-lg)', borderRadius: 'var(--radius-lg)' }}>
                    <div style={{ display: 'flex', alignItems: 'center', gap: '1rem', marginBottom: '2rem' }}>
                        <Shield size={24} color="var(--accent-emerald)" />
                        <h3 style={{ margin: 0, fontSize: '1.2rem' }}>{t('settings.commerceEconomicBase')}</h3>
                    </div>

                    <div style={{ display: 'flex', flexDirection: 'column', gap: '1.5rem' }}>
                        <div>
                            <label style={labelStyle}>{t('settings.activeCommerceProvider', { defaultValue: 'Active Commerce Provider' })}</label>
                            <div style={{ fontSize: '0.75rem', color: 'var(--text-muted)', marginBottom: '0.8rem' }}>
                                {t('settings.commerceProviderHelp', { defaultValue: 'Select the economic engine powering your agent network.' })}
                            </div>
                            <select
                                value={getSetting('commerce_provider') || 'mock'}
                                onChange={(e) => update_setting_handler(e.target.value, 'commerce_provider', 'commerce')}
                                style={selectStyle}
                            >
                                <option value="mock" style={{ background: 'var(--bg-primary)' }}>{t('settings.commerceMock', { defaultValue: 'Mock (Local Only)' })}</option>
                                <option value="stripe" style={{ background: 'var(--bg-primary)' }}>{t('settings.commerceStripe', { defaultValue: 'Stripe (Global MoR)' })}</option>
                                <option value="polar" style={{ background: 'var(--bg-primary)' }}>{t('settings.commercePolar', { defaultValue: 'Polar (MoR & P2P)' })}</option>
                            </select>
                        </div>

                        {getSetting('commerce_provider') === 'stripe' && (
                            <>
                                <SettingInput 
                                    label={t('settings.stripeApiKey', { defaultValue: 'Stripe API Key' }) as string} 
                                    value={getSetting('stripe_api_key')}
                                    placeholder="sk_live_..."
                                    onBlur={(v) => updateSetting('stripe_api_key', v, 'commerce')}
                                    saving={saving === 'stripe_api_key'}
                                    isPassword
                                />
                                <SettingInput 
                                    label={t('settings.stripeWebhookSecret', { defaultValue: 'Stripe Webhook Secret' }) as string} 
                                    value={getSetting('stripe_webhook_secret')}
                                    placeholder="whsec_..."
                                    onBlur={(v) => updateSetting('stripe_webhook_secret', v, 'commerce')}
                                    saving={saving === 'stripe_webhook_secret'}
                                    isPassword
                                />
                            </>
                        )}

                        {getSetting('commerce_provider') === 'polar' && (
                            <>
                                <SettingInput 
                                    label={t('settings.polarApiKey', { defaultValue: 'Polar API Key' }) as string} 
                                    value={getSetting('polar_api_key')}
                                    placeholder="polar_at_..."
                                    onBlur={(v) => updateSetting('polar_api_key', v, 'commerce')}
                                    saving={saving === 'polar_api_key'}
                                    isPassword
                                />
                                <SettingInput 
                                    label={t('settings.polarWebhookSecret', { defaultValue: 'Polar Webhook Secret' }) as string} 
                                    value={getSetting('polar_webhook_secret')}
                                    placeholder="whsec_..."
                                    onBlur={(v) => updateSetting('polar_webhook_secret', v, 'commerce')}
                                    saving={saving === 'polar_webhook_secret'}
                                    isPassword
                                />
                            </>
                        )}
                    </div>
                </section>
                )}

                {/* Channel Bridges Section */}
                {viewMode !== 'beginner' && (
                <section className="glass-panel" style={{ padding: 'var(--space-lg)', borderRadius: 'var(--radius-lg)' }}>
                    <div style={{ display: 'flex', alignItems: 'center', gap: '1rem', marginBottom: '2rem' }}>
                        <Share2 size={24} color="var(--accent-cyan)" />
                        <h3 style={{ margin: 0, fontSize: '1.2rem' }}>{t('settings.channelBridges')}</h3>
                    </div>

                    <div style={{ display: 'flex', flexDirection: 'column', gap: '1.5rem' }}>
                        <SettingInput 
                            label={t('settings.xBearerToken')} 
                            value={getSetting('x_bearer_token')}
                            placeholder={t('settings.enterApiKey')}
                            onBlur={(v) => updateSetting('x_bearer_token', v, 'integrations')}
                            saving={saving === 'x_bearer_token'}
                            isPassword
                        />
                        <div style={{ fontSize: '0.75rem', color: 'var(--text-muted)' }}>
                            {t('settings.xBearerTokenNotice')}
                        </div>
                    </div>
                </section>
                )}

                {/* 3. Security & Infrastructure */}
                {viewMode !== 'beginner' && (
                <section className="glass-panel" style={{ padding: 'var(--space-lg)', borderRadius: 'var(--radius-lg)' }}>
                    <div style={{ display: 'flex', alignItems: 'center', gap: '1rem', marginBottom: '2rem' }}>
                        <Lock size={24} color="var(--accent-amber)" />
                        <h3 style={{ margin: 0, fontSize: '1.2rem' }}>{t('settings.securityInfrastructure')}</h3>
                    </div>
                    <div style={{ display: 'flex', flexDirection: 'column', gap: '1.5rem' }}>
                        <SecretUpdater />
                        <OriginManager 
                            value={getSetting('allowed_origins')} 
                            onUpdate={(v) => updateSetting('allowed_origins', v, 'security')}
                            saving={saving === 'allowed_origins'}
                        />
                        <ToxicityConfig 
                            value={getSetting('csam_toxicity_forbidden_words')}
                            onUpdate={(v) => updateSetting('csam_toxicity_forbidden_words', v, 'security')}
                            saving={saving === 'csam_toxicity_forbidden_words'}
                        />
                    </div>
                </section>
                )}

                {viewMode === 'advanced' && (
                <section className="glass-panel" style={{ padding: 'var(--space-lg)', borderRadius: 'var(--radius-lg)' }}>
                    <div style={{ display: 'flex', alignItems: 'center', gap: '1rem', marginBottom: '2rem' }}>
                        <Shield size={24} color="var(--accent-emerald)" />
                        <h3 style={{ margin: 0, fontSize: '1.2rem' }}>{t('settings.featureFlags')}</h3>
                    </div>
                    
                    <div style={{ display: 'flex', flexDirection: 'column', gap: '1.5rem' }}>
                        <FeatureToggle 
                            label={t('settings.ffSeoPublishing', { defaultValue: 'SEO Publishing' }) as string} 
                            flag="feature_flag.seo_publish" 
                            current={getSetting('feature_flag.seo_publish')} 
                            onUpdate={(v) => updateSetting('feature_flag.seo_publish', v, 'feature_flags')} 
                            saving={saving === 'feature_flag.seo_publish'} 
                        />
                        <FeatureToggle 
                            label={t('settings.ffP2pFederation', { defaultValue: 'P2P Federation' }) as string} 
                            flag="feature_flag.p2p_federation" 
                            current={getSetting('feature_flag.p2p_federation')} 
                            onUpdate={(v) => updateSetting('feature_flag.p2p_federation', v, 'feature_flags')} 
                            saving={saving === 'feature_flag.p2p_federation'} 
                        />
                        <FeatureToggle 
                            label={t('settings.ffLoraTraining', { defaultValue: 'LoRA Training' }) as string} 
                            flag="feature_flag.lora_training" 
                            current={getSetting('feature_flag.lora_training')} 
                            onUpdate={(v) => updateSetting('feature_flag.lora_training', v, 'feature_flags')} 
                            saving={saving === 'feature_flag.lora_training'} 
                        />
                        <FeatureToggle 
                            label={t('settings.ffGigMarketplace', { defaultValue: 'Gig Marketplace' }) as string} 
                            flag="feature_flag.gig_marketplace" 
                            current={getSetting('feature_flag.gig_marketplace')} 
                            onUpdate={(v) => updateSetting('feature_flag.gig_marketplace', v, 'feature_flags')} 
                            saving={saving === 'feature_flag.gig_marketplace'} 
                        />
                        <FeatureToggle 
                            label={t('settings.ffIntentFirstSuggestion', { defaultValue: 'Intent-First Suggestion' }) as string} 
                            flag="feature_flag.intent_first_suggestion" 
                            current={getSetting('feature_flag.intent_first_suggestion')} 
                            onUpdate={(v) => updateSetting('feature_flag.intent_first_suggestion', v, 'feature_flags')} 
                            saving={saving === 'feature_flag.intent_first_suggestion'} 
                        />
                        <FeatureToggle 
                            label={t('settings.ffSemanticToolReviewer', { defaultValue: 'Semantic Tool Reviewer' }) as string} 
                            flag="ENABLE_TOOL_REVIEWER" 
                            current={getSetting('ENABLE_TOOL_REVIEWER') || "true"} 
                            onUpdate={(v) => updateSetting('ENABLE_TOOL_REVIEWER', v, 'feature_flags')} 
                            saving={saving === 'ENABLE_TOOL_REVIEWER'} 
                        />
                    </div>
                </section>
                )}

                {viewMode === 'advanced' && (
                    <div style={{ display: 'flex', flexDirection: 'column', gap: 'var(--space-lg)' }}>
                        <EscrowManagementView />
                        <McpConfigManager />
                    </div>
                )}

            </div>
        </div>
    );
};

// --- Sub-Components ---

const OriginManager: React.FC<{ value: string, onUpdate: (v: string) => void, saving?: boolean }> = ({ value, onUpdate, saving }) => {
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

const SecretUpdater: React.FC = () => {
    const { t } = useTranslation();
    const [newSecret, setNewSecret] = useState('');
    const [result, setResult] = useState<{ success: boolean, message: string } | null>(null);
    const [testing, setTesting] = useState(false);

    const handleUpdate = async () => {
        if (!newSecret.trim()) return;
        setTesting(true);
        const oldSecret = sessionStorage.getItem('aiome_secret');
        setAuthToken(newSecret.trim());
        
        try {
            const res = await authenticatedFetch(`${API_BASE}/api/health`, {
                headers: { 'Authorization': `Bearer ${newSecret.trim()}` },
            });
            if (res.ok) {
                setResult({ success: true, message: t('settings.tokenVerified') });
                setNewSecret('');
            } else {
                setResult({ success: false, message: t('settings.authFailed', { status: res.status }) });
                if (oldSecret) setAuthToken(oldSecret);
                else clearAuthToken();
            }
        } catch {
            setResult({ success: false, message: t('settings.connectionFailed') });
            if (oldSecret) setAuthToken(oldSecret);
            else clearAuthToken();
        } finally {
            setTesting(false);
        }
    };

    return (
        <div>
            <label style={labelStyle}>{t('settings.updateApiSecret')}</label>
            <div style={{ display: 'flex', gap: '0.5rem' }}>
                <input
                    type="password" value={newSecret} placeholder={t('settings.enterNewSecret')}
                    onChange={(e) => { setNewSecret(e.target.value); setResult(null); }}
                    onKeyDown={(e) => { if (e.nativeEvent.isComposing) return; if (e.key === 'Enter') handleUpdate(); }}
                    style={{ ...inputStyle, flex: 1 }}
                />
                <button onClick={handleUpdate} disabled={testing} style={{ ...testBtnStyle, padding: '0.5rem 0.8rem' }}>
                    {testing ? <Loader2 size={14} className="ani-spin" /> : <Check size={14} />}
                    {t('settings.verify')}
                </button>
            </div>
            {result && (
                <div style={testResultStyle(result.success)}>
                    {result.success ? <Check size={12} /> : <X size={12} />}
                    {result.message}
                </div>
            )}
        </div>
    );
};

const ToxicityConfig: React.FC<{ value: string, onUpdate: (v: string) => void, saving?: boolean }> = ({ value, onUpdate, saving }) => {
    const { t } = useTranslation();
    const [draft, setDraft] = useState('');
    const items = value ? value.split(',').map(s => s.trim()).filter(Boolean) : [];

    const addWord = () => {
        if (!draft.trim()) return;
        const newWords = draft.split(',').map(s => s.trim()).filter(Boolean);
        if (newWords.length === 0) return;
        const updated = Array.from(new Set([...items, ...newWords])).join(',');
        onUpdate(updated);
        setDraft('');
    };

    const removeWord = (idx: number) => {
        const updated = items.filter((_, i) => i !== idx).join(',');
        onUpdate(updated);
    };

    return (
        <div style={{ marginTop: '1.5rem', paddingTop: '1.5rem', borderTop: '1px solid var(--border-glass)' }}>
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '0.6rem' }}>
                <label style={{ ...labelStyle, marginBottom: 0 }}>{t('settings.contentSafetyFilter', { defaultValue: 'Content Safety Filter' })}</label>
                {saving && <Loader2 size={12} className="ani-spin" color="var(--accent-amber)" />}
            </div>
            <div style={{ fontSize: '0.75rem', color: 'var(--text-muted)', marginBottom: '0.8rem' }}>
                {t('settings.contentSafetyDesc', { defaultValue: 'Words added here will be blocked during AI generation and Federation P2P messaging.' })}
            </div>
            <div style={{ display: 'flex', flexWrap: 'wrap', gap: 'var(--space-xs)', marginBottom: '0.6rem' }}>
                {items.map((item, i) => (
                    <div key={i} style={{
                        display: 'flex', alignItems: 'center', gap: '0.4rem',
                        background: 'var(--accent-rose-glass)', border: '1px solid var(--accent-rose-20)',
                        borderRadius: '6px', padding: '0.3rem 0.6rem', fontSize: '0.75rem',
                        color: 'var(--accent-rose)'
                    }}>
                        <span>{item}</span>
                        <X size={12} style={{ cursor: 'pointer', opacity: 0.6 }} onClick={() => removeWord(i)} />
                    </div>
                ))}
                {items.length === 0 && <span style={{ fontSize: '0.75rem', color: 'var(--text-muted)' }}>{t('settings.noBlockedWords', { defaultValue: 'No blocked words.' })}</span>}
            </div>
            <div style={{ display: 'flex', gap: '0.5rem' }}>
                <input
                    type="text" value={draft} placeholder={t('settings.enterBannedWord', { defaultValue: 'Enter a banned word...' }) as string}
                    onChange={(e) => setDraft(e.target.value)}
                    onKeyDown={(e) => { if (e.nativeEvent.isComposing) return; if (e.key === 'Enter') addWord(); }}
                    style={{ ...inputStyle, flex: 1 }}
                />
                <button onClick={addWord} style={{ ...testBtnStyle, padding: '0.5rem 0.8rem' }}>
                    <Plus size={14} /> {t('settings.add')}
                </button>
            </div>
        </div>
    );
};

const SettingInput: React.FC<{ label: string, value: string, placeholder?: string, onBlur: (v: string) => void, saving?: boolean, isPassword?: boolean }> = ({ label, value, placeholder, onBlur, saving, isPassword }) => {
    const [local, setLocal] = useState(value);
    useEffect(() => setLocal(value), [value]);

    return (
        <div>
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '0.6rem' }}>
                <label style={{ ...labelStyle, marginBottom: 0 }}>{label}</label>
                {saving && <Loader2 size={12} className="ani-spin" color="var(--accent-cyan)" />}
            </div>
            <input
                type={isPassword ? "password" : "text"}
                value={local}
                placeholder={placeholder}
                onChange={(e) => setLocal(e.target.value)}
                onBlur={() => onBlur(local)}
                style={inputStyle}
            />
        </div>
    );
};

const OllamaModelSelector: React.FC<{ value: string, onSelect: (v: string) => void, saving?: boolean }> = ({ value, onSelect, saving }) => {
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
                <button onClick={fetchModels} disabled={loading} title="Refresh Models" style={{ ...testBtnStyle, padding: '0.5rem 0.8rem' }}>
                    {loading ? <Loader2 size={14} className="ani-spin" /> : t('settings.refresh')}
                </button>
            </div>
            {error && <div style={{ fontSize: '0.7rem', color: 'var(--accent-rose)', marginTop: '0.4rem' }}>{error}</div>}
        </div>
    );
};

const McpConfigManager: React.FC = () => {
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
                        <button 
                            onClick={saveConfig} 
                            disabled={saving}
                            className="primary-button"
                            style={{ 
                                padding: '0.6rem 1.2rem', background: 'var(--accent-amber)', color: 'var(--bg-primary)', 
                                border: 'none', borderRadius: 'var(--radius-md)', fontWeight: 700, cursor: 'pointer',
                                display: 'flex', alignItems: 'center', gap: '0.5rem'
                            }}
                        >
                            {saving ? <Loader2 size={16} className="ani-spin" /> : <Database size={16} />}
                            {t('settings.saveSyncTools', { defaultValue: 'Save & Sync Tools' })}
                        </button>
                    </div>
                </div>
            )}
        </section>
    );
};

const FeatureToggle: React.FC<{ label: string, flag: string, current: string, onUpdate: (v: string) => void, saving?: boolean }> = ({ label, flag, current, onUpdate, saving }) => {
    const isEnabled = current === "true" || current === "1";
    return (
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', background: 'var(--white-03)', padding: '0.8rem', borderRadius: 'var(--radius-sm)', border: '1px solid var(--border-glass)' }}>
            <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
                <span style={{ fontSize: '0.85rem', color: 'var(--text-primary)', fontWeight: 600 }}>{label}</span>
            </div>
            <div style={{ display: 'flex', alignItems: 'center', gap: '1rem' }}>
                {saving && <Loader2 size={14} className="ani-spin" color="var(--accent-emerald)" />}
                <label style={{ position: 'relative', display: 'inline-block', width: '40px', height: '22px' }}>
                    <input type="checkbox" checked={isEnabled} onChange={(e) => onUpdate(e.target.checked ? "true" : "false")} style={{ opacity: 0, width: 0, height: 0 }} />
                    <span style={{ 
                        position: 'absolute', cursor: 'pointer', top: 0, left: 0, right: 0, bottom: 0, 
                        backgroundColor: isEnabled ? 'var(--accent-emerald)' : 'var(--black-40)', 
                        transition: '.4s', borderRadius: '22px' 
                    }}>
                        <span style={{ 
                            position: 'absolute', content: '""', height: '16px', width: '16px', left: '3px', bottom: '3px', 
                            backgroundColor: 'white', transition: '.4s', borderRadius: '50%',
                            transform: isEnabled ? 'translateX(18px)' : 'translateX(0)'
                        }}></span>
                    </span>
                </label>
            </div>
        </div>
    );
};

// --- Styles ---

const labelStyle: React.CSSProperties = {
    display: 'block',
    color: 'var(--text-secondary)',
    fontSize: '0.8rem',
    marginBottom: '0.8rem',
    fontWeight: 700,
    textTransform: 'uppercase',
    letterSpacing: '0.05em'
};

const inputStyle: React.CSSProperties = {
    width: '100%',
    background: 'var(--white-03)',
    border: '1px solid var(--border-glass)',
    borderRadius: 'var(--radius-sm)',
    padding: '0.8rem',
    color: 'var(--text-primary)',
    fontSize: '0.85rem',
    outline: 'none',
    transition: 'all var(--speed-normal)',
    boxSizing: 'border-box'
};

const selectStyle: React.CSSProperties = {
    ...inputStyle,
    cursor: 'pointer'
};

const charCardStyle = (active: boolean, tint: 'purple' | 'cyan'): React.CSSProperties => ({
    padding: '1.2rem',
    borderRadius: 'var(--radius-md)',
    background: active ? `var(--accent-${tint}-glass)` : 'var(--bg-glass-light)',
    border: `1px solid ${active ? `var(--accent-${tint})` : 'var(--border-glass)'}`,
    cursor: 'pointer',
    transition: 'all var(--speed-normal)',
    textAlign: 'center',
    boxShadow: active ? `var(--glow-${tint})` : 'var(--shadow-shallow)'
});

const styleCardStyle = (active: boolean, character: string): React.CSSProperties => ({
    padding: '0.8rem',
    borderRadius: 'var(--radius-md)',
    background: active ? 'var(--white-06)' : 'transparent',
    border: `1px solid ${active ? (character === 'male' ? 'var(--accent-cyan)' : 'var(--accent-purple)') : 'var(--border-glass)'}`,
    cursor: 'pointer',
    textAlign: 'center',
    fontSize: '0.8rem',
    transition: 'all var(--speed-normal)',
    color: active ? 'var(--text-primary)' : 'var(--text-muted)'
});

const modeBtnStyle = (active: boolean): React.CSSProperties => ({
    flex: 1,
    padding: '10px',
    border: 'none',
    background: active ? 'var(--accent-cyan)' : 'transparent',
    color: active ? 'var(--bg-primary)' : 'var(--text-muted)',
    borderRadius: 'var(--radius-sm)',
    cursor: 'pointer',
    fontSize: '0.8rem',
    fontWeight: active ? 800 : 400,
    textTransform: 'capitalize',
    transition: 'all var(--speed-normal)'
});

const testBtnStyle: React.CSSProperties = {
    display: 'flex',
    alignItems: 'center',
    gap: '0.5rem',
    padding: '0.6rem 1rem',
    background: 'var(--bg-glass-light)',
    border: '1px solid var(--border-glass)',
    borderRadius: 'var(--radius-sm)',
    color: 'var(--text-primary)',
    fontSize: '0.75rem',
    cursor: 'pointer',
    transition: 'all var(--speed-normal)',
    fontWeight: 600
};

const testResultStyle = (success: boolean): React.CSSProperties => ({
    marginTop: '0.6rem',
    fontSize: '0.7rem',
    color: success ? 'var(--accent-emerald)' : 'var(--accent-rose)',
    display: 'flex',
    alignItems: 'center',
    gap: '0.4rem',
    background: success ? 'var(--accent-emerald-10)' : 'var(--accent-rose-10)',
    padding: '0.5rem',
    borderRadius: 'var(--radius-sm)',
    border: `1px solid ${success ? 'var(--accent-emerald-20)' : 'var(--accent-rose-20)'}`,
    fontWeight: 700
});

export default SettingsPage;

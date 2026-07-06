/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import React, { useState, useEffect } from 'react';
import { useAvatarCharacter } from '../hooks/AvatarContext';
import { useTranslation, useLanguage } from '../i18n';
import { useDisplayMode } from '../hooks/useDisplayMode';
import { useViewMode } from '../hooks/useViewMode';
import { SettingEntry, ViewMode } from '../types';
import {
    Monitor, Lock, Database,
    Shield, Check, X, Loader2, Share2, AlertTriangle
} from 'lucide-react';
import { API_BASE } from '../config';
import { OllamaModelSelector } from './OllamaModelSelector';
import { McpConfigManager } from './McpConfigManager';
import { setAuthToken, authenticatedFetch, clearAuthToken } from '../lib/auth';
import EscrowManagementView from './EscrowManagementView';
import { OriginManager } from './OriginManager';
import { ToxicityConfig } from './ToxicityConfig';
import { VaultSecretsManager } from './VaultSecretsManager';
import { VaultKeyStatus } from './VaultKeyStatus';
import { useToast } from './common/Toast';
import { LoadingState } from './ui/LoadingState';
import { SectionHeader } from './ui/SectionHeader';


const SettingsPage: React.FC = () => {
    const { character, setCharacter, proportion, setProportion } = useAvatarCharacter();
    const { mode, setMode } = useDisplayMode();
    const { viewMode, setViewMode } = useViewMode();
    const { t } = useTranslation();
    const { lang, setLang } = useLanguage();
    const { showToast } = useToast();
    const [settings, setSettings] = useState<SettingEntry[]>([]);
    const [loading, setLoading] = useState(true);
    const [loadError, setLoadError] = useState<string | null>(null);
    const [saving, setSaving] = useState<string | null>(null);
    const [testResults, setTestResults] = useState<Record<string, { success: boolean, message: string, loading: boolean }>>({});
    const [globalError, setGlobalError] = useState<string | null>(null);
    const [vaultSecrets, setVaultSecrets] = useState<{key: string, is_set: boolean}[]>([]);

    const fetchVaultStatus = async () => {
        try {
            const res = await authenticatedFetch(`${API_BASE}/api/v1/vault/status`);
            if (res.ok) {
                const data = await res.json();
                setVaultSecrets(data.secrets || []);
            }
        } catch (e) {
            console.error("Failed to fetch vault status in SettingsPage", e);
        }
    };

    const isVaultSet = (key: string): boolean => {
        return vaultSecrets.find(s => s.key === key)?.is_set || false;
    };

    useEffect(() => {
        fetchSettings();
        fetchVaultStatus();
    }, []);

    const fetchSettings = async () => {
        setLoadError(null);
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
            } else {
                const message = t('settings.loadFailed', { defaultValue: 'Failed to load settings.' });
                setLoadError(message);
                showToast('error', message);
            }
        } catch (error) {
            console.error("Failed to fetch settings", error);
            const message = t('common.networkError', { defaultValue: 'A network error occurred.' });
            setLoadError(message);
            showToast('error', message);
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
                showToast('success', t('settings.saveSuccess', { defaultValue: 'Setting saved successfully.' }));
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
        return <LoadingState />;
    }

    if (loadError) {
        return (
            <div className="ui-center-state">
                <AlertTriangle size={40} color="var(--accent-rose)" />
                <p style={{ color: 'var(--text-secondary)' }}>{loadError}</p>
                <button className="primary-button" onClick={() => { setLoading(true); fetchSettings(); }}>
                    {t('error.retry', { defaultValue: 'Retry' })}
                </button>
            </div>
        );
    }

    const update_setting_handler = (val: string, k: string, cat: string) => updateSetting(k, val, cat);

    return (
        <div className="settings-page">
            {globalError && (
                <div className="ui-error-banner">
                    <AlertTriangle size={20} />
                    {globalError}
                </div>
            )}
            <div className="settings-grid">

                {/* 1. Appearance Section */}
                <section className="glass-panel ui-card--pad-lg">
                    <SectionHeader icon={<Monitor size={24} color="var(--accent-cyan)" />} title={t('settings.appearance')} />

                    <div className="ui-field-stack">
                        <SettingInput
                            label={t('settings.aiName')}
                            value={getSetting('ai_name')}
                            placeholder={t('settings.aiNamePlaceholder', { defaultValue: 'Watchtower' }) as string}
                            onBlur={(v) => updateSetting('ai_name', v, 'identity')}
                            saving={saving === 'ai_name'}
                        />

                        <div>
                            <label className="ui-field-label">{t('settings.avatarCharacter')}</label>
                            <div className="ui-char-grid">
                                <div onClick={() => setCharacter('female')} className={`ui-select-card${character === "female" ? " ui-select-card--active ui-select-card--purple" : ""}`}>
                                    <div className="ui-select-card__emoji">♀</div>
                                    <div className="ui-select-card__label">{t('settings.female')}</div>
                                </div>
                                <div onClick={() => setCharacter('male')} className={`ui-select-card${character === "male" ? " ui-select-card--active ui-select-card--cyan" : ""}`}>
                                    <div className="ui-select-card__emoji">♂</div>
                                    <div className="ui-select-card__label">{t('settings.male')}</div>
                                </div>
                            </div>
                        </div>

                        <div>
                            <label className="ui-field-label">{t('settings.avatarStyle')}</label>
                            <div className="ui-char-grid">
                                <div
                                    onClick={() => setProportion('chibi')}
                                    className={`ui-style-card${proportion === 'chibi' ? ` ui-style-card--active ui-style-card--${character === 'male' ? 'male' : 'female'}` : ''}`}
                                >
                                    {t('settings.cuteChibi')}
                                </div>
                                <div
                                    onClick={() => setProportion('taller')}
                                    className={`ui-style-card${proportion === 'taller' ? ` ui-style-card--active ui-style-card--${character === 'male' ? 'male' : 'female'}` : ''}`}
                                >
                                    {t('settings.modernTaller')}
                                </div>
                            </div>
                        </div>

                        <div>
                            <label className="ui-field-label">{t('settings.displayMode')}</label>
                            <div className="ui-help-text">
                                {t('settings.displayModeHelp', { defaultValue: 'Choose rendering fidelity. VRM requires GPU.' })}
                            </div>
                            <div className="ui-segment-group">
                                {['vrm', 'lite', 'off'].map((m) => (
                                    <button
                                        key={m}
                                        onClick={() => setMode(m as 'vrm' | 'lite' | 'off')}
                                        className={`ui-segment-btn${mode === m ? " ui-segment-btn--active" : ""}`}
                                    >
                                        {m === 'vrm' ? '🌟 ' : m === 'lite' ? '⚡ ' : '🚫 '}{m}
                                    </button>
                                ))}
                            </div>
                        </div>

                        <div>
                            <label className="ui-field-label">{t('settings.language')}</label>
                            <div className="ui-segment-group">
                                <button onClick={() => setLang('en')} className={`ui-segment-btn${lang === 'en' ? " ui-segment-btn--active" : ""}`}>
                                    🇺🇸 {t('language.en')}
                                </button>
                                <button onClick={() => setLang('ja')} className={`ui-segment-btn${lang === 'ja' ? " ui-segment-btn--active" : ""}`}>
                                    🇯🇵 {t('language.ja')}
                                </button>
                            </div>
                        </div>

                        <div>
                            <label className="ui-field-label">{t('settings.interfaceComplexity')}</label>
                            <div className="ui-help-text">
                                {t('settings.interfaceComplexityHelp', { defaultValue: 'Adjusts available settings and logs based on your experience level.' })}
                            </div>
                            <div className="ui-segment-group">
                                {([
                                    { value: 'simple' as ViewMode, labelKey: 'settings.viewMode_beginner' },
                                    { value: 'cockpit' as ViewMode, labelKey: 'settings.viewMode_advanced' },
                                ]).map(({ value, labelKey }) => (
                                    <button
                                        key={value}
                                        onClick={() => setViewMode(value)}
                                        className={`ui-segment-btn${viewMode === value ? ' ui-segment-btn--active' : ''}`}
                                        style={{ textTransform: 'capitalize' }}
                                    >
                                        {t(labelKey)}
                                    </button>
                                ))}
                            </div>
                        </div>

                        {/* U6-7: デモはサイドバー常設から降格し、設定から再生できるようにする */}
                        <div>
                            <label className="ui-field-label">{t('settings.demoLauncher', { defaultValue: 'Autonomous AI Demo' })}</label>
                            <div className="ui-help-text">
                                {t('settings.demoLauncherHelp', { defaultValue: 'Replay the guided demo of the autonomous AI workflow.' })}
                            </div>
                            <button
                                type="button"
                                data-testid="settings-launch-demo"
                                onClick={() => window.dispatchEvent(new CustomEvent('a2ui-navigate', { detail: { tab: 'demo' } }))}
                                className={`ui-segment-btn${false ? " ui-segment-btn--active" : ""}`}
                            >
                                ▶ {t('settings.demoLauncherStart', { defaultValue: 'Play Demo' })}
                            </button>
                        </div>
                    </div>
                </section>

                {/* 2. LLM Configuration Section */}
                <section className="glass-panel ui-card--pad-lg">
                    <SectionHeader icon={<Database size={24} color="var(--accent-purple)" />} title={t('settings.llmEngine')} />

                    <div className="ui-field-stack ui-field-stack--compact">
                        <div>
                            <label className="ui-field-label">{t('settings.llmProvider')}</label>
                            <select
                                value={getSetting('llm_provider') || 'ollama'}
                                onChange={(e) => update_setting_handler(e.target.value, 'llm_provider', 'llm')}
                                className="ui-select"
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
                            className="ui-test-btn"
                        >
                            {testResults['llm']?.loading ? <Loader2 size={16} className="ani-spin" /> : <Shield size={16} />}
                            {t('settings.testLlmConnection')}
                        </button>
                        {testResults['llm'] && (
                            <div className={`ui-test-result ${(testResults['llm'].success) ? "ui-test-result--success" : "ui-test-result--error"}`}>
                                {testResults['llm'].success ? <Check size={14} /> : <X size={14} />}
                                {testResults['llm'].message}
                            </div>
                        )}
                    </div>
                </section>

                {/* Commerce Integration Section */}
                {viewMode === 'cockpit' && (
                <section className="glass-panel ui-card--pad-lg">
                    <SectionHeader icon={<Shield size={24} color="var(--accent-emerald)" />} title={t('settings.commerceEconomicBase')} />

                    <div className="ui-field-stack ui-field-stack--compact">
                        <div>
                            <label className="ui-field-label">{t('settings.activeCommerceProvider', { defaultValue: 'Active Commerce Provider' })}</label>
                            <div className="ui-help-text">
                                {t('settings.commerceProviderHelp', { defaultValue: 'Select the economic engine powering your agent network.' })}
                            </div>
                            <select
                                value={getSetting('commerce_provider') || 'mock'}
                                onChange={(e) => update_setting_handler(e.target.value, 'commerce_provider', 'commerce')}
                                className="ui-select"
                            >
                                <option value="mock" style={{ background: 'var(--bg-primary)' }}>{t('settings.commerceMock', { defaultValue: 'Mock (Local Only)' })}</option>
                                <option value="stripe" style={{ background: 'var(--bg-primary)' }}>{t('settings.commerceStripe', { defaultValue: 'Stripe (Global MoR)' })}</option>
                                <option value="polar" style={{ background: 'var(--bg-primary)' }}>{t('settings.commercePolar', { defaultValue: 'Polar (MoR & P2P)' })}</option>
                            </select>
                        </div>

                        <SettingInput
                            label={t('settings.monthlySpendLimit', { defaultValue: 'Monthly spend limit (KC, 0 = unlimited)' }) as string}
                            value={getSetting('economy.monthly_spend_limit')}
                            placeholder="0"
                            onBlur={(v) => updateSetting('economy.monthly_spend_limit', v, 'commerce')}
                            saving={saving === 'economy.monthly_spend_limit'}
                        />

                        {getSetting('commerce_provider') === 'stripe' && (
                            <>
                                <div className="ui-field-stack ui-field-stack--xs ui-field-stack--mb-md">
                                    <SettingInput 
                                        label={t('settings.stripeApiKey', { defaultValue: 'Stripe API Key' }) as string} 
                                        value={getSetting('stripe_api_key')}
                                        placeholder="sk_live_..."
                                        onBlur={(v) => updateSetting('stripe_api_key', v, 'commerce')}
                                        saving={saving === 'stripe_api_key'}
                                        isPassword
                                    />
                                    <VaultKeyStatus isSet={isVaultSet('STRIPE_API_KEY')} />
                                </div>
                                <div className="ui-field-stack ui-field-stack--xs ui-field-stack--mb-md">
                                    <SettingInput 
                                        label={t('settings.stripeWebhookSecret', { defaultValue: 'Stripe Webhook Secret' }) as string} 
                                        value={getSetting('stripe_webhook_secret')}
                                        placeholder="whsec_..."
                                        onBlur={(v) => updateSetting('stripe_webhook_secret', v, 'commerce')}
                                        saving={saving === 'stripe_webhook_secret'}
                                        isPassword
                                    />
                                    <VaultKeyStatus isSet={isVaultSet('STRIPE_WEBHOOK_SECRET')} />
                                </div>
                            </>
                        )}

                        {getSetting('commerce_provider') === 'polar' && (
                            <>
                                <div className="ui-field-stack ui-field-stack--xs ui-field-stack--mb-md">
                                    <SettingInput 
                                        label={t('settings.polarApiKey', { defaultValue: 'Polar API Key' }) as string} 
                                        value={getSetting('polar_api_key')}
                                        placeholder="polar_at_..."
                                        onBlur={(v) => updateSetting('polar_api_key', v, 'commerce')}
                                        saving={saving === 'polar_api_key'}
                                        isPassword
                                    />
                                    <VaultKeyStatus isSet={isVaultSet('POLAR_API_KEY')} />
                                </div>
                                <div className="ui-field-stack ui-field-stack--xs ui-field-stack--mb-md">
                                    <SettingInput 
                                        label={t('settings.polarWebhookSecret', { defaultValue: 'Polar Webhook Secret' }) as string} 
                                        value={getSetting('polar_webhook_secret')}
                                        placeholder="whsec_..."
                                        onBlur={(v) => updateSetting('polar_webhook_secret', v, 'commerce')}
                                        saving={saving === 'polar_webhook_secret'}
                                        isPassword
                                    />
                                    <VaultKeyStatus isSet={isVaultSet('POLAR_WEBHOOK_SECRET')} />
                                </div>
                            </>
                        )}
                    </div>
                </section>
                )}

                {/* Channel Bridges Section */}
                {viewMode === 'cockpit' && (
                <section className="glass-panel ui-card--pad-lg">
                    <SectionHeader icon={<Share2 size={24} color="var(--accent-cyan)" />} title={t('settings.channelBridges')} />

                    <div className="ui-field-stack ui-field-stack--compact">
                        <div style={{ display: 'flex', flexDirection: 'column', gap: 'var(--space-xs)' }}>
                            <SettingInput 
                                label={t('settings.xBearerToken')} 
                                value={getSetting('x_bearer_token')}
                                placeholder={t('settings.enterApiKey')}
                                onBlur={(v) => updateSetting('x_bearer_token', v, 'integrations')}
                                saving={saving === 'x_bearer_token'}
                                isPassword
                            />
                            <VaultKeyStatus isSet={isVaultSet('X_BEARER_TOKEN')} />
                        </div>
                        <div style={{ fontSize: '0.75rem', color: 'var(--text-muted)' }}>
                            {t('settings.xBearerTokenNotice')}
                        </div>
                    </div>
                </section>
                )}

                {/* 🔐 Vault Secrets Manager — 全モードで常時表示 */}
                <section className="glass-panel ui-card--pad-lg">
                    <VaultSecretsManager />
                </section>

                {/* 3. Security & Infrastructure */}
                {viewMode === 'cockpit' && (
                <section className="glass-panel ui-card--pad-lg">
                    <SectionHeader icon={<Lock size={24} color="var(--accent-amber)" />} title={t('settings.securityInfrastructure')} />
                    <div className="ui-field-stack ui-field-stack--compact">
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

                {viewMode === 'cockpit' && (
                <section className="glass-panel ui-card--pad-lg">
                    <SectionHeader icon={<Shield size={24} color="var(--accent-emerald)" />} title={t('settings.featureFlags')} />
                    
                    <div className="ui-field-stack ui-field-stack--compact">
                        <FeatureToggle 
                            label={t('settings.ffSeoPublishing', { defaultValue: 'SEO Publishing' }) as string} 
                            current={getSetting('feature_flag.seo_publish')} 
                            onUpdate={(v) => updateSetting('feature_flag.seo_publish', v, 'feature_flags')} 
                            saving={saving === 'feature_flag.seo_publish'} 
                        />
                        <FeatureToggle 
                            label={t('settings.ffJsFallback', { defaultValue: 'Headless Browser Fallback' }) as string} 
                            current={getSetting('feature_flag.js_fallback')} 
                            onUpdate={(v) => updateSetting('feature_flag.js_fallback', v, 'feature_flags')} 
                            saving={saving === 'feature_flag.js_fallback'} 
                        />
                        <FeatureToggle 
                            label={t('settings.ffP2pFederation', { defaultValue: 'P2P Federation' }) as string} 
                            current={getSetting('feature_flag.p2p_federation')} 
                            onUpdate={(v) => updateSetting('feature_flag.p2p_federation', v, 'feature_flags')} 
                            saving={saving === 'feature_flag.p2p_federation'} 
                        />
                        <FeatureToggle 
                            label={t('settings.ffLoraTraining', { defaultValue: 'LoRA Training' }) as string} 
                            current={getSetting('feature_flag.lora_training')} 
                            onUpdate={(v) => updateSetting('feature_flag.lora_training', v, 'feature_flags')} 
                            saving={saving === 'feature_flag.lora_training'} 
                        />
                        <FeatureToggle 
                            label={t('settings.ffGigMarketplace', { defaultValue: 'Gig Marketplace' }) as string} 
                            current={getSetting('feature_flag.gig_marketplace')} 
                            onUpdate={(v) => updateSetting('feature_flag.gig_marketplace', v, 'feature_flags')} 
                            saving={saving === 'feature_flag.gig_marketplace'} 
                        />
                        <FeatureToggle 
                            label={t('settings.ffIntentFirstSuggestion', { defaultValue: 'Intent-First Suggestion' }) as string} 
                            current={getSetting('feature_flag.intent_first_suggestion')} 
                            onUpdate={(v) => updateSetting('feature_flag.intent_first_suggestion', v, 'feature_flags')} 
                            saving={saving === 'feature_flag.intent_first_suggestion'} 
                        />
                        <FeatureToggle 
                            label={t('settings.ffSemanticToolReviewer', { defaultValue: 'Semantic Tool Reviewer' }) as string} 
                            current={getSetting('ENABLE_TOOL_REVIEWER') || "true"} 
                            onUpdate={(v) => updateSetting('ENABLE_TOOL_REVIEWER', v, 'feature_flags')} 
                            saving={saving === 'ENABLE_TOOL_REVIEWER'} 
                        />
                        <FeatureToggle 
                            label={t('settings.ffA2uiGenerativeUi', { defaultValue: 'Generative UI (A2UI)' }) as string} 
                            current={getSetting('feature_flag.a2ui_generative_ui')} 
                            onUpdate={(v) => updateSetting('feature_flag.a2ui_generative_ui', v, 'feature_flags')} 
                            saving={saving === 'feature_flag.a2ui_generative_ui'} 
                        />
                    </div>
                </section>
                )}

                {viewMode === 'cockpit' && (
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

// OriginManager: imported at file top

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
            <label className="ui-field-label">{t('settings.updateApiSecret')}</label>
            <div className="ui-field-row">
                <input
                    type="password" value={newSecret} placeholder={t('settings.enterNewSecret')}
                    onChange={(e) => { setNewSecret(e.target.value); setResult(null); }}
                    onKeyDown={(e) => { if (e.nativeEvent.isComposing) return; if (e.key === 'Enter') handleUpdate(); }}
                    className="ui-input ui-input--flex"
                />
                <button onClick={handleUpdate} disabled={testing} className="ui-test-btn ui-test-btn--compact">
                    {testing ? <Loader2 size={14} className="ani-spin" /> : <Check size={14} />}
                    {t('settings.verify')}
                </button>
            </div>
            {result && (
                <div className={`ui-test-result ${(result.success) ? "ui-test-result--success" : "ui-test-result--error"}`}>
                    {result.success ? <Check size={12} /> : <X size={12} />}
                    {result.message}
                </div>
            )}
        </div>
    );
};

// ToxicityConfig: imported at file top

const SettingInput: React.FC<{ label: string, value: string, placeholder?: string, onBlur: (v: string) => void, saving?: boolean, isPassword?: boolean }> = ({ label, value, placeholder, onBlur, saving, isPassword }) => {
    const [local, setLocal] = useState(value);
    useEffect(() => setLocal(value), [value]);

    return (
        <div>
            <div className="ui-field-row ui-field-row--between">
                <label className="ui-field-label ui-field-label--inline">{label}</label>
                {saving && <Loader2 size={12} className="ani-spin" color="var(--accent-cyan)" />}
            </div>
            <input
                type={isPassword ? "password" : "text"}
                value={local}
                placeholder={placeholder}
                onChange={(e) => setLocal(e.target.value)}
                onBlur={() => onBlur(local)}
                className="ui-input"
            />
        </div>
    );
};

// The implementations for OllamaModelSelector and McpConfigManager have been extracted to their own files.

const FeatureToggle: React.FC<{ label: string, current: string, onUpdate: (v: string) => void, saving?: boolean }> = ({ label, current, onUpdate, saving }) => {
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


export default SettingsPage;

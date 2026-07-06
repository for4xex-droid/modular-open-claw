/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import React, { useState, useMemo, useEffect } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { Sparkles, User, ShieldAlert, Loader2, Eye, EyeOff, BookOpen, Check } from 'lucide-react';
import { useTranslation } from '../i18n';
import { API_BASE } from '../config';
import { setAuthToken, authenticatedFetch } from '../lib/auth';
import { reloadApp } from '../lib/navigation';

interface SetupWizardProps {
    onComplete: () => void;
}

interface PlaybookSummary {
    id: string;
    name: string;
    description: string;
    tags: string[];
    workflow_count: number;
    required_skills: string[];
    required_mcp_servers: string[];
}

/** Password strength evaluator (0-4 scale) — pure function, no i18n dependency */
function evaluatePasswordScore(pw: string): number {
    let score = 0;
    if (pw.length >= 12) score++;
    if (pw.length >= 16) score++;
    if (/[A-Z]/.test(pw) && /[a-z]/.test(pw)) score++;
    if (/\d/.test(pw)) score++;
    if (/[^A-Za-z0-9]/.test(pw)) score++;
    return Math.min(score, 4);
}

const PW_COLORS = [
    'var(--accent-rose)',
    'var(--accent-rose)',
    'var(--accent-amber)',
    'var(--accent-emerald)',
    'var(--accent-cyan)'
];

/** Minimal email format check */
function isValidEmail(email: string): boolean {
    return /^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(email);
}

const SetupWizard: React.FC<SetupWizardProps> = ({ onComplete }) => {
    const { t } = useTranslation();
    const [step, setStep] = useState(0);

    // State for all steps
    const [tosAccepted, setTosAccepted] = useState(false);
    const [aiName, setAiName] = useState("");
    const [viewMode, setViewMode] = useState("cockpit");
    const [email, setEmail] = useState("");
    const [password, setPassword] = useState("");
    const [confirmPassword, setConfirmPassword] = useState("");
    const [showPassword, setShowPassword] = useState(false);

    const [isSaving, setIsSaving] = useState(false);
    const [errorMsg, setErrorMsg] = useState<string | null>(null);

    // STEP 6: Playbook selection state
    const [playbooks, setPlaybooks] = useState<PlaybookSummary[]>([]);
    const [installedPlaybooks, setInstalledPlaybooks] = useState<string[]>([]);
    const [installingPlaybook, setInstallingPlaybook] = useState<string | null>(null);
    const [playbookErrors, setPlaybookErrors] = useState<Record<string, string>>({});

    useEffect(() => {
        if (step !== 6) return;
        let cancelled = false;
        (async () => {
            try {
                const res = await authenticatedFetch(`${API_BASE}/api/v1/playbooks`);
                if (!res.ok) throw new Error(`Failed to load playbooks: ${res.status}`);
                const data: PlaybookSummary[] = await res.json();
                if (!cancelled) setPlaybooks(data);
            } catch (error) {
                console.error("Failed to load playbooks", error);
                if (!cancelled) setPlaybooks([]);
            }
        })();
        return () => { cancelled = true; };
    }, [step]);

    const handleInstallPlaybook = async (id: string) => {
        setInstallingPlaybook(id);
        setPlaybookErrors(prev => ({ ...prev, [id]: '' }));
        try {
            const res = await authenticatedFetch(`${API_BASE}/api/v1/playbooks/${id}/install`, { method: 'POST' });
            if (res.status === 422) {
                const body = await res.json().catch(() => ({}));
                const missing = [
                    ...(body.missing_skills || []),
                    ...(body.missing_mcp_servers || [])
                ].join(', ');
                throw new Error(`${t('setup.playbookMissingDeps') || 'Missing dependencies'}: ${missing}`);
            }
            if (!res.ok) throw new Error(`${t('setup.playbookError') || 'Installation failed'} (${res.status})`);
            setInstalledPlaybooks(prev => [...prev, id]);
        } catch (error: unknown) {
            const msg = error instanceof Error ? error.message : (t('setup.playbookError') || 'Installation failed');
            setPlaybookErrors(prev => ({ ...prev, [id]: msg }));
        } finally {
            setInstallingPlaybook(null);
        }
    };

    const pwStrength = useMemo(() => {
        const score = evaluatePasswordScore(password);
        const labels = [t('setup.pwTooShort') || 'Too Short', t('setup.pwWeak') || 'Weak', t('setup.pwFair') || 'Fair', t('setup.pwStrong') || 'Strong', t('setup.pwExcellent') || 'Excellent'];
        return { score, label: labels[score] || '', color: PW_COLORS[score] || 'var(--text-muted)' };
    }, [password, t]);

    const canSubmit = !isSaving && tosAccepted && isValidEmail(email) && password.length >= 12 && password === confirmPassword;

    const handleFinalize = async () => {
        if (password !== confirmPassword) {
            setErrorMsg(t('setup.passwordMismatch') || "Passwords do not match");
            return;
        }
        if (password.length < 12) {
            setErrorMsg(t('setup.passwordTooShort') || "Password must be at least 12 characters");
            return;
        }
        if (!isValidEmail(email)) {
            setErrorMsg(t('setup.invalidEmail') || "Please enter a valid email address");
            return;
        }
        
        setStep(5); // Generating step
        setIsSaving(true);
        setErrorMsg(null);
        
        try {
            const payload = {
                admin_email: email,
                admin_password: password,
                ai_name: aiName || "Watchtower",
                view_mode: viewMode,
                language: navigator.language.startsWith('ja') ? 'ja' : 'en',
                tos_accepted: tosAccepted
            };

            const res = await fetch(`${API_BASE}/api/v1/setup/init`, {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify(payload)
            });

            if (!res.ok) {
                const data = await res.json().catch(() => ({}));
                throw new Error(data.message || (t('setup.failed') || 'Setup failed'));
            }

            const data = await res.json();
            if (!data.access_token) throw new Error(t('setup.noToken') || 'Server did not return an access token');
            setAuthToken(data.access_token);
            onComplete();

            // Proceed to Playbook selection (reload happens on Start/Skip there)
            setStep(6);
        } catch (error: unknown) {
            console.error("Setup failed", error);
            const msg = error instanceof Error ? error.message : '';
            // Provide user-friendly message for network errors
            const isNetworkError = msg.includes('Failed to fetch') || msg.includes('NetworkError');
            setErrorMsg(isNetworkError
                ? (t('setup.networkError') || 'Unable to reach the server. Please check your connection and try again.')
                : (msg || (t('setup.unknownError') || 'Unknown error'))
            );
            setStep(4); // Go back to password step to show error
        } finally {
            setIsSaving(false);
        }
    };

    return (
        <div style={{
            position: 'fixed',
            inset: 0,
            background: 'var(--bg-base)',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            zIndex: 9999,
            backgroundImage: 'radial-gradient(circle at 50% -20%, var(--accent-cyan-10), transparent 60%)'
        }}>
            <motion.div
                initial={{ scale: 0.95, opacity: 0, y: 10 }}
                animate={{ scale: 1, opacity: 1, y: 0 }}
                style={{
                    width: '550px',
                    minHeight: '520px',
                    padding: '3rem',
                    background: 'var(--bg-glass-heavy)',
                    border: '1px solid var(--border-glass-bright)',
                    borderRadius: 'var(--radius-xl)',
                    textAlign: 'center',
                    boxShadow: 'var(--shadow-deep)',
                    position: 'relative',
                    overflow: 'hidden'
                }}
            >
                {/* Progress indicator */}
                <div style={{ display: 'flex', justifyContent: 'center', gap: '0.5rem', marginBottom: '2rem' }}>
                    {[0, 1, 2, 3, 4].map(i => (
                        <div key={i} style={{
                            width: i <= step ? '2rem' : '0.5rem',
                            height: '4px',
                            borderRadius: '2px',
                            background: i <= step ? 'var(--accent-cyan)' : 'var(--white-10)',
                            transition: 'all 0.3s ease'
                        }} />
                    ))}
                </div>

                <AnimatePresence mode="wait">
                    {/* STEP 0: Intro */}
                    {step === 0 && (
                        <motion.div key="step0" initial={{ opacity: 0, x: 20 }} animate={{ opacity: 1, x: 0 }} exit={{ opacity: 0, x: -20 }}>
                            <div style={{ padding: '2rem', background: 'var(--white-03)', borderRadius: '50%', marginBottom: '1.5rem', display: 'inline-block' }}>
                                <Sparkles size={48} color="var(--accent-cyan)" />
                            </div>
                            <h2 style={{ fontSize: '2rem', fontWeight: 800, marginBottom: '1rem' }}>
                                {t('setup.welcome') || 'Welcome to Aiome'}
                            </h2>
                            <p style={{ color: 'var(--text-secondary)', lineHeight: 1.6, marginBottom: '2rem' }}>
                                {t('setup.welcomeDesc') || 'Your autonomous AI operating system is ready to be initialized. Let\'s set up your environment securely.'}
                            </p>
                            <button
                                onClick={() => setStep(1)}
                                style={{
                                    padding: '0.8rem 2.5rem',
                                    background: 'var(--accent-cyan)',
                                    color: 'var(--text-inverse)',
                                    border: 'none',
                                    borderRadius: 'var(--radius-md)',
                                    fontWeight: 700,
                                    cursor: 'pointer'
                                }}
                            >
                                {t('setup.startSetup') || 'Start Setup'}
                            </button>
                        </motion.div>
                    )}

                    {/* STEP 1: TOS */}
                    {step === 1 && (
                        <motion.div key="step1" initial={{ opacity: 0, x: 20 }} animate={{ opacity: 1, x: 0 }} exit={{ opacity: 0, x: -20 }}>
                            <h2 style={{ fontSize: '1.8rem', fontWeight: 800, marginBottom: '1rem' }}>
                                {t('setup.tos') || 'Terms of Service'}
                            </h2>
                            <div style={{ 
                                height: '200px', 
                                overflowY: 'auto', 
                                background: 'var(--black-30)', 
                                padding: '1rem', 
                                borderRadius: 'var(--radius-md)',
                                textAlign: 'left',
                                fontSize: '0.9rem',
                                color: 'var(--text-secondary)',
                                marginBottom: '1.5rem'
                            }}>
                                <p>{t('setup.tosContent') || 'By using Aiome, you agree to local data processing. You are responsible for the actions of your autonomous agents.'}</p>
                            </div>
                            
                            <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', gap: '0.5rem', marginBottom: '2rem' }}>
                                <input 
                                    type="checkbox" 
                                    id="tos" 
                                    checked={tosAccepted} 
                                    onChange={(e) => setTosAccepted(e.target.checked)} 
                                    style={{ accentColor: 'var(--accent-cyan)' }}
                                />
                                <label htmlFor="tos">{t('setup.agreeTos') || 'I agree to the Terms of Service'}</label>
                            </div>

                            <button
                                onClick={() => setStep(2)}
                                disabled={!tosAccepted}
                                style={{
                                    padding: '0.8rem 2.5rem',
                                    background: 'var(--accent-cyan)',
                                    color: 'var(--text-inverse)',
                                    border: 'none',
                                    borderRadius: 'var(--radius-md)',
                                    fontWeight: 700,
                                    cursor: tosAccepted ? 'pointer' : 'not-allowed',
                                    opacity: tosAccepted ? 1 : 0.5
                                }}
                            >
                                {t('setup.next') || 'Next'}
                            </button>
                        </motion.div>
                    )}

                    {/* STEP 2: AI Name */}
                    {step === 2 && (
                        <motion.div key="step2" initial={{ opacity: 0, x: 20 }} animate={{ opacity: 1, x: 0 }} exit={{ opacity: 0, x: -20 }}>
                            <div style={{ padding: '2rem', background: 'var(--white-03)', borderRadius: '50%', marginBottom: '1.5rem', display: 'inline-block' }}>
                                <User size={48} color="var(--accent-cyan)" />
                            </div>
                            <h2 style={{ fontSize: '1.8rem', fontWeight: 800, marginBottom: '1rem' }}>
                                {t('setup.nameYourAi') || 'Name your AI'}
                            </h2>
                            <p style={{ color: 'var(--text-secondary)', marginBottom: '2rem' }}>
                                {t('setup.nameDesc') || 'Give your operating system an identity.'}
                            </p>
                            
                            <input
                                id="aiName"
                                type="text"
                                value={aiName}
                                onChange={(e) => setAiName(e.target.value)}
                                maxLength={64}
                                placeholder={t('setup.namePlaceholder') || 'e.g. Watchtower'}
                                aria-label={t('setup.nameYourAi') || 'Name your AI'}
                                style={{
                                    width: '100%',
                                    background: 'var(--black-30)',
                                    border: '1px solid var(--border-glass)',
                                    borderRadius: 'var(--radius-md)',
                                    padding: '1rem',
                                    color: 'var(--text-primary)',
                                    fontSize: '1.1rem',
                                    outline: 'none',
                                    marginBottom: '2rem'
                                }}
                            />

                            <button
                                onClick={() => setStep(3)}
                                style={{
                                    padding: '0.8rem 2.5rem',
                                    background: 'var(--accent-cyan)',
                                    color: 'var(--text-inverse)',
                                    border: 'none',
                                    borderRadius: 'var(--radius-md)',
                                    fontWeight: 700,
                                    cursor: 'pointer'
                                }}
                            >
                                {t('setup.next') || 'Next'}
                            </button>
                        </motion.div>
                    )}

                    {/* STEP 3: View Mode */}
                    {step === 3 && (
                        <motion.div key="step3" initial={{ opacity: 0, x: 20 }} animate={{ opacity: 1, x: 0 }} exit={{ opacity: 0, x: -20 }}>
                            <h2 style={{ fontSize: '1.8rem', fontWeight: 800, marginBottom: '1rem' }}>
                                {t('setup.chooseExperience') || 'Choose your experience'}
                            </h2>
                            <p style={{ color: 'var(--text-secondary)', marginBottom: '2rem' }}>
                                {t('setup.chooseExperienceDesc') || 'Select how much complexity you want to see.'}
                            </p>
                            
                            <div style={{ display: 'flex', flexDirection: 'column', gap: '0.8rem' }}>
                                <button
                                    onClick={() => { setViewMode('simple'); setStep(4); }}
                                    style={{ padding: '1.5rem', background: 'var(--white-03)', border: '1px solid var(--border-glass)', borderRadius: 'var(--radius-md)', color: 'var(--text-primary)', cursor: 'pointer', textAlign: 'left' }}
                                >
                                    <strong style={{ display: 'block', marginBottom: '0.5rem', color: 'var(--accent-cyan)' }}>
                                        {t('setup.simpleMode') || 'Simple Mode'}
                                    </strong>
                                    <span style={{ fontSize: '0.9rem', color: 'var(--text-secondary)' }}>
                                        {t('setup.simpleModeDesc') || 'Streamlined interface for general usage.'}
                                    </span>
                                </button>
                                <button
                                    onClick={() => { setViewMode('cockpit'); setStep(4); }}
                                    style={{ padding: '1.5rem', background: 'var(--white-03)', border: '1px solid var(--accent-cyan-10)', borderRadius: 'var(--radius-md)', color: 'var(--text-primary)', cursor: 'pointer', textAlign: 'left' }}
                                >
                                    <strong style={{ display: 'block', marginBottom: '0.5rem', color: 'var(--accent-cyan)' }}>
                                        {t('setup.cockpitMode') || t('setup.expertMode') || 'Cockpit Mode'}
                                        <span style={{ marginLeft: '0.5rem', fontSize: '0.7rem', background: 'var(--accent-cyan-10)', color: 'var(--accent-cyan)', padding: '0.15rem 0.5rem', borderRadius: 'var(--radius-sm)' }}>
                                            {t('setup.recommended') || 'Recommended'}
                                        </span>
                                    </strong>
                                    <span style={{ fontSize: '0.9rem', color: 'var(--text-secondary)' }}>
                                        {t('setup.cockpitModeDesc') || t('setup.expertModeDesc') || 'Full observability, logs, and developer tools.'}
                                    </span>
                                </button>
                            </div>
                        </motion.div>
                    )}

                    {/* STEP 4: Admin Credentials */}
                    {step === 4 && (
                        <motion.div key="step4" initial={{ opacity: 0, x: 20 }} animate={{ opacity: 1, x: 0 }} exit={{ opacity: 0, x: -20 }}>
                            <div style={{ padding: '2rem', background: 'var(--white-03)', borderRadius: '50%', marginBottom: '1.5rem', display: 'inline-block' }}>
                                <ShieldAlert size={48} color="var(--accent-cyan)" />
                            </div>
                            <h2 style={{ fontSize: '1.8rem', fontWeight: 800, marginBottom: '1rem' }}>
                                {t('setup.createAdmin') || 'Create Admin'}
                            </h2>
                            <p style={{ color: 'var(--text-secondary)', marginBottom: '1.5rem' }}>
                                {t('setup.createAdminDesc') || 'Secure your instance with a strong password.'}
                            </p>
                            
                            {errorMsg && (
                                <div role="alert" style={{ color: 'var(--accent-rose)', background: 'var(--accent-rose-10)', padding: '0.75rem', borderRadius: 'var(--radius-sm)', marginBottom: '1rem', border: '1px solid var(--accent-rose-20)', display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
                                    <ShieldAlert size={16} />
                                    {errorMsg}
                                </div>
                            )}

                            <div style={{ textAlign: 'left', marginBottom: '1rem' }}>
                                <label htmlFor="email" style={{ display: 'block', fontSize: '0.8rem', color: 'var(--text-secondary)', marginBottom: '0.5rem' }}>
                                    {t('setup.emailLabel') || 'Email'}
                                </label>
                                <input
                                    id="email"
                                    type="email"
                                    value={email}
                                    onChange={(e) => setEmail(e.target.value)}
                                    placeholder="admin@example.com"
                                    autoComplete="email"
                                    style={{ width: '100%', background: 'var(--black-30)', border: `1px solid ${email && !isValidEmail(email) ? 'var(--accent-rose-50)' : 'var(--border-glass)'}`, borderRadius: 'var(--radius-md)', padding: '1rem', color: 'var(--text-primary)' }}
                                />
                                {email && !isValidEmail(email) && (
                                    <span style={{ fontSize: '0.75rem', color: 'var(--accent-rose)', marginTop: '0.25rem', display: 'block' }}>
                                        {t('setup.invalidEmail') || 'Please enter a valid email address'}
                                    </span>
                                )}
                            </div>

                            <div style={{ textAlign: 'left', marginBottom: '1rem' }}>
                                <label htmlFor="password" style={{ display: 'block', fontSize: '0.8rem', color: 'var(--text-secondary)', marginBottom: '0.5rem' }}>
                                    {t('setup.passwordLabel') || 'Password (min 12 chars)'}
                                </label>
                                <div style={{ position: 'relative' }}>
                                    <input
                                        id="password"
                                        type={showPassword ? 'text' : 'password'}
                                        value={password}
                                        onChange={(e) => setPassword(e.target.value)}
                                        autoComplete="new-password"
                                        style={{ width: '100%', background: 'var(--black-30)', border: '1px solid var(--border-glass)', borderRadius: 'var(--radius-md)', padding: '1rem', paddingRight: '3rem', color: 'var(--text-primary)' }}
                                    />
                                    <button
                                        type="button"
                                        onClick={() => setShowPassword(v => !v)}
                                        style={{ position: 'absolute', right: '0.75rem', top: '50%', transform: 'translateY(-50%)', background: 'none', border: 'none', cursor: 'pointer', color: 'var(--text-muted)', padding: '0.25rem' }}
                                        aria-label={showPassword ? 'Hide password' : 'Show password'}
                                    >
                                        {showPassword ? <EyeOff size={18} /> : <Eye size={18} />}
                                    </button>
                                </div>
                                {/* Password strength meter */}
                                {password.length > 0 && (
                                    <div style={{ marginTop: '0.5rem' }}>
                                        <div style={{ display: 'flex', gap: '3px', marginBottom: '0.25rem' }}>
                                            {[1, 2, 3, 4].map(i => (
                                                <div key={i} style={{
                                                    flex: 1, height: '3px', borderRadius: '2px',
                                                    background: i <= pwStrength.score ? pwStrength.color : 'var(--white-10)',
                                                    transition: 'background 0.2s'
                                                }} />
                                            ))}
                                        </div>
                                        <span style={{ fontSize: '0.7rem', color: pwStrength.color }}>{pwStrength.label}</span>
                                    </div>
                                )}
                            </div>

                            <div style={{ textAlign: 'left', marginBottom: '2rem' }}>
                                <label htmlFor="confirmPassword" style={{ display: 'block', fontSize: '0.8rem', color: 'var(--text-secondary)', marginBottom: '0.5rem' }}>
                                    {t('setup.confirmPassword') || 'Confirm Password'}
                                </label>
                                <input
                                    id="confirmPassword"
                                    type="password"
                                    value={confirmPassword}
                                    onChange={(e) => setConfirmPassword(e.target.value)}
                                    autoComplete="new-password"
                                    style={{ width: '100%', background: 'var(--black-30)', border: `1px solid ${confirmPassword && password !== confirmPassword ? 'var(--accent-rose-50)' : 'var(--border-glass)'}`, borderRadius: 'var(--radius-md)', padding: '1rem', color: 'var(--text-primary)' }}
                                />
                                {confirmPassword && password !== confirmPassword && (
                                    <span style={{ fontSize: '0.75rem', color: 'var(--accent-rose)', marginTop: '0.25rem', display: 'block' }}>
                                        {t('setup.passwordMismatch') || 'Passwords do not match'}
                                    </span>
                                )}
                            </div>

                            <button
                                onClick={handleFinalize}
                                disabled={!canSubmit}
                                style={{
                                    padding: '0.8rem 2.5rem',
                                    background: 'linear-gradient(135deg, var(--accent-cyan), var(--accent-purple))',
                                    color: 'var(--text-inverse)',
                                    border: 'none',
                                    borderRadius: 'var(--radius-md)',
                                    fontWeight: 700,
                                    cursor: canSubmit ? 'pointer' : 'not-allowed',
                                    opacity: canSubmit ? 1 : 0.5,
                                    width: '100%'
                                }}
                            >
                                {t('setup.initialize') || 'Initialize System'}
                            </button>
                        </motion.div>
                    )}

                    {/* STEP 5: Finalizing */}
                    {step === 5 && (
                        <motion.div key="step5" initial={{ opacity: 0, scale: 0.9 }} animate={{ opacity: 1, scale: 1 }}>
                            <div style={{ padding: '2rem', marginBottom: '1rem' }}>
                                <Loader2 size={64} color="var(--accent-cyan)" className="animate-spin" />
                            </div>
                            <h2 style={{ fontSize: '1.8rem', fontWeight: 800, marginBottom: '1rem' }}>
                                {t('setup.awakening') || 'Awakening System...'}
                            </h2>
                            <p style={{ color: 'var(--text-secondary)' }}>
                                {t('setup.awakeningDesc') || 'Securing credentials and booting core modules.'}
                            </p>
                        </motion.div>
                    )}

                    {/* STEP 6: Playbook selection (after successful init; reload on Start/Skip) */}
                    {step === 6 && (
                        <motion.div key="step6" initial={{ opacity: 0, x: 20 }} animate={{ opacity: 1, x: 0 }} exit={{ opacity: 0, x: -20 }}>
                            <div style={{ padding: '1.5rem', background: 'var(--white-03)', borderRadius: '50%', marginBottom: '1rem', display: 'inline-block' }}>
                                <BookOpen size={40} color="var(--accent-cyan)" />
                            </div>
                            <h2 style={{ fontSize: '1.8rem', fontWeight: 800, marginBottom: '0.5rem' }}>
                                {t('setup.playbookTitle') || 'Choose a Playbook'}
                            </h2>
                            <p style={{ color: 'var(--text-secondary)', marginBottom: '1.5rem' }}>
                                {t('setup.playbookDesc') || 'Install ready-made business workflows so your agent can start working today. You can skip this and add them later.'}
                            </p>

                            <div style={{ display: 'flex', flexDirection: 'column', gap: '0.6rem', marginBottom: '1.5rem', maxHeight: '260px', overflowY: 'auto' }}>
                                {playbooks.map(pb => {
                                    const installed = installedPlaybooks.includes(pb.id);
                                    const installing = installingPlaybook === pb.id;
                                    return (
                                        <div key={pb.id} style={{ padding: '1rem', background: 'var(--white-03)', border: `1px solid ${installed ? 'var(--accent-emerald)' : 'var(--border-glass)'}`, borderRadius: 'var(--radius-md)', textAlign: 'left' }}>
                                            <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', gap: '1rem' }}>
                                                <div>
                                                    <strong style={{ display: 'block', color: 'var(--accent-cyan)' }}>{pb.name}</strong>
                                                    <span style={{ fontSize: '0.85rem', color: 'var(--text-secondary)' }}>{pb.description}</span>
                                                </div>
                                                <button
                                                    onClick={() => handleInstallPlaybook(pb.id)}
                                                    disabled={installed || installing}
                                                    style={{
                                                        padding: '0.5rem 1rem',
                                                        background: installed ? 'var(--accent-emerald)' : 'var(--accent-cyan)',
                                                        color: 'var(--text-inverse)',
                                                        border: 'none',
                                                        borderRadius: 'var(--radius-md)',
                                                        fontWeight: 700,
                                                        cursor: installed || installing ? 'default' : 'pointer',
                                                        opacity: installing ? 0.6 : 1,
                                                        whiteSpace: 'nowrap',
                                                        display: 'flex',
                                                        alignItems: 'center',
                                                        gap: '0.3rem'
                                                    }}
                                                >
                                                    {installed
                                                        ? (<><Check size={14} />{t('setup.playbookInstalled') || 'Installed'}</>)
                                                        : installing
                                                            ? (t('setup.playbookInstalling') || 'Installing...')
                                                            : (t('setup.playbookInstall') || 'Install')}
                                                </button>
                                            </div>
                                            {playbookErrors[pb.id] && (
                                                <div role="alert" style={{ marginTop: '0.5rem', fontSize: '0.8rem', color: 'var(--accent-rose)' }}>
                                                    {playbookErrors[pb.id]}
                                                </div>
                                            )}
                                        </div>
                                    );
                                })}
                            </div>

                            <div style={{ display: 'flex', justifyContent: 'center', gap: '1rem' }}>
                                <button
                                    onClick={() => reloadApp()}
                                    style={{ padding: '0.8rem 2rem', background: 'transparent', color: 'var(--text-secondary)', border: '1px solid var(--border-glass)', borderRadius: 'var(--radius-md)', fontWeight: 700, cursor: 'pointer' }}
                                >
                                    {t('setup.playbookSkip') || 'Skip'}
                                </button>
                                <button
                                    onClick={() => reloadApp()}
                                    style={{ padding: '0.8rem 2rem', background: 'var(--accent-cyan)', color: 'var(--text-inverse)', border: 'none', borderRadius: 'var(--radius-md)', fontWeight: 700, cursor: 'pointer' }}
                                >
                                    {t('setup.playbookStart') || 'Start Aiome'}
                                </button>
                            </div>
                        </motion.div>
                    )}
                </AnimatePresence>
            </motion.div>
        </div>
    );
};

export default SetupWizard;

/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import React, { useState } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { Lock, ShieldAlert, Zap, Loader2 } from 'lucide-react';
import { API_BASE } from '../config';
import { setAuthToken } from '../lib/auth';
import { useTranslation } from '../i18n';

interface LoginScreenProps {
    onAuthenticated: () => void;
}

const LoginScreen: React.FC<LoginScreenProps> = ({ onAuthenticated }) => {
    const [password, setPassword] = useState('');
    const [error, setError] = useState('');
    const [isLoading, setIsLoading] = useState(false);
    const { t } = useTranslation();

    const handleLogin = async (e: React.FormEvent) => {
        e.preventDefault();
        if (!password) return;

        setIsLoading(true);
        setError('');

        try {
            const res = await fetch(`${API_BASE}/api/v1/auth/token`, {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({
                    grant_type: 'password',
                    client_id: 'admin',
                    client_secret: password
                })
            });

            if (!res.ok) {
                const data = await res.json().catch(() => ({}));
                throw new Error(data.message || (t('auth.invalidCredentials') || 'Invalid credentials'));
            }

            const data = await res.json();
            if (!data.access_token) throw new Error(t('auth.noToken') || 'Server did not return an access token');
            setAuthToken(data.access_token);
            onAuthenticated();
            // Force reload to apply BootMode::Normal changes and trigger system load
            window.location.reload();
        } catch (err: unknown) {
            console.error("Login failed", err);
            setError(err instanceof Error ? err.message : (t('auth.authFailed') || "Authentication failed"));
        } finally {
            setIsLoading(false);
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
            backgroundImage: 'radial-gradient(circle at 50% -20%, var(--accent-cyan-10), transparent 70%)'
        }}>
            <motion.div
                initial={{ opacity: 0, y: 20 }}
                animate={{ opacity: 1, y: 0 }}
                style={{
                    width: '400px',
                    padding: '3rem',
                    background: 'var(--bg-glass-heavy)',
                    border: '1px solid var(--border-glass-bright)',
                    borderRadius: 'var(--radius-xl)',
                    textAlign: 'center',
                    boxShadow: 'var(--shadow-deep)',
                    backdropFilter: 'blur(20px)'
                }}
            >
                <div style={{ display: 'flex', justifyContent: 'center', marginBottom: '1.5rem' }}>
                    <div style={{ padding: '1rem', background: 'var(--white-03)', borderRadius: '50%' }}>
                        <Lock size={40} color="var(--accent-cyan)" />
                    </div>
                </div>

                <h1 style={{ fontSize: '1.8rem', fontWeight: 800, marginBottom: '0.5rem' }}>
                    {t('auth.title') || 'Aiome Identity'}
                </h1>
                <p style={{ color: 'var(--text-secondary)', marginBottom: '2.5rem', fontSize: '0.95rem' }}>
                    {t('auth.subtitle') || 'Provide credentials to unlock the ecosystem.'}
                </p>

                <form onSubmit={handleLogin}>
                    <div style={{ textAlign: 'left', marginBottom: '1.5rem' }}>
                        <label htmlFor="login-password" style={{ display: 'block', fontSize: '0.8rem', color: 'var(--text-secondary)', marginBottom: '0.5rem', fontWeight: 600 }}>
                            {t('auth.passwordLabel') || 'Password'}
                        </label>
                        <input
                            id="login-password"
                            type="password"
                            value={password}
                            onChange={(e) => setPassword(e.target.value)}
                            placeholder="••••••••••••"
                            autoComplete="current-password"
                            style={{
                                width: '100%',
                                background: 'var(--black-30)',
                                border: '1px solid var(--border-glass)',
                                borderRadius: 'var(--radius-md)',
                                padding: '1rem',
                                color: 'var(--text-primary)',
                                outline: 'none',
                                fontSize: '1rem',
                                transition: 'all var(--speed-fast)'
                            }}
                            autoFocus
                        />
                    </div>

                    <AnimatePresence>
                        {error && (
                            <motion.div
                                role="alert"
                                initial={{ opacity: 0, height: 0 }}
                                animate={{ opacity: 1, height: 'auto' }}
                                exit={{ opacity: 0, height: 0 }}
                                style={{
                                    display: 'flex',
                                    alignItems: 'center',
                                    gap: '0.75rem',
                                    color: 'var(--accent-rose)',
                                    fontSize: '0.85rem',
                                    marginBottom: '1.5rem',
                                    padding: '0.75rem',
                                    background: 'var(--accent-rose-10)',
                                    borderRadius: 'var(--radius-sm)',
                                    border: '1px solid var(--accent-rose-20)'
                                }}
                            >
                                <ShieldAlert size={16} />
                                <span>{error}</span>
                            </motion.div>
                        )}
                    </AnimatePresence>

                    <button
                        type="submit"
                        disabled={isLoading || !password}
                        style={{
                            width: '100%',
                            padding: '1rem',
                            borderRadius: 'var(--radius-md)',
                            background: isLoading ? 'var(--white-05)' : 'linear-gradient(90deg, var(--accent-cyan), var(--accent-purple))',
                            border: 'none',
                            color: 'var(--text-inverse)',
                            fontWeight: 700,
                            fontSize: '1rem',
                            cursor: (isLoading || !password) ? 'not-allowed' : 'pointer',
                            display: 'flex',
                            alignItems: 'center',
                            justifyContent: 'center',
                            gap: '0.5rem',
                            opacity: (isLoading || !password) ? 0.5 : 1,
                            transition: 'all var(--speed-fast)'
                        }}
                    >
                        {isLoading ? (
                            <Loader2 className="animate-spin" size={20} color="var(--accent-cyan)" />
                        ) : (
                            <>
                                <Zap size={20} />
                                {t('auth.login') || 'Login'}
                            </>
                        )}
                    </button>
                </form>
            </motion.div>
        </div>
    );
};

export default LoginScreen;

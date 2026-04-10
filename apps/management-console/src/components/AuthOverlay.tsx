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

interface AuthOverlayProps {
    onAuthenticated: () => void;
}

const AuthOverlay: React.FC<AuthOverlayProps> = ({ onAuthenticated }) => {
    const [token, setToken] = useState('');
    const [error, setError] = useState('');
    const [isLoading, setIsLoading] = useState(false);
    const { t } = useTranslation();

    const handleLogin = async (e: React.FormEvent) => {
        e.preventDefault();
        const trimmedToken = token.trim();
        if (!trimmedToken) return;

        setIsLoading(true);
        setError('');

        const abortController = new AbortController();
        const timeoutId = setTimeout(() => abortController.abort(), 10000);

        try {
            // 認証が必要なエンドポイントで検証する（/api/health は公開なのでトークン検証にならない）
            const response = await fetch(`${API_BASE}/api/v1/settings`, {
                headers: {
                    'Authorization': `Bearer ${trimmedToken}`
                },
                signal: abortController.signal
            });

            clearTimeout(timeoutId);

            if (response.ok || response.status === 200) {
                setAuthToken(trimmedToken);
                onAuthenticated();
            } else if (response.status === 401) {
                setError(t('auth.errorInvalidKey'));
            } else {
                setError(t('auth.errorServer', { status: response.status }));
            }
        } catch (err: any) {
            clearTimeout(timeoutId);
            if (err.name === 'AbortError') {
                setError(t('auth.errorTimeout') || 'Connection timed out');
            } else {
                setError(t('auth.errorConnection'));
            }
        } finally {
            setIsLoading(false);
        }
    };

    return (
        <motion.div 
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            className="auth-overlay"
            style={{
                position: 'fixed',
                inset: 0,
                zIndex: 9999,
                background: 'var(--black-85)',
                backdropFilter: 'blur(20px)',
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                padding: '2rem'
            }}
        >
            <motion.div
                initial={{ scale: 0.9, y: 20 }}
                animate={{ scale: 1, y: 0 }}
                style={{
                    width: '100%',
                    maxWidth: '450px',
                    background: 'linear-gradient(135deg, var(--black-80) 0%, var(--black-90) 100%)',
                    border: '1px solid var(--accent-cyan-10)',
                    borderRadius: '24px',
                    padding: '3rem',
                    textAlign: 'center',
                    boxShadow: '0 20px 50px var(--black-50), 0 0 30px var(--accent-cyan-05)'
                }}
            >
                <div style={{ marginBottom: '2rem', display: 'flex', justifyContent: 'center' }}>
                    <div style={{ 
                        width: '80px', 
                        height: '80px', 
                        borderRadius: '50%', 
                        background: 'var(--accent-cyan-05)',
                        display: 'flex',
                        alignItems: 'center',
                        justifyContent: 'center',
                        border: '1px solid var(--accent-cyan-20)'
                    }}>
                        <Lock color="var(--accent-cyan)" size={40} />
                    </div>
                </div>

                <h2 style={{ 
                    color: 'var(--text-primary)', 
                    fontSize: '1.8rem', 
                    fontWeight: 800, 
                    marginBottom: '0.5rem',
                    letterSpacing: '-0.02em'
                }}>
                    {t('auth.title')}
                </h2>
                <p style={{ 
                    color: 'var(--white-50)', 
                    fontSize: '0.9rem', 
                    marginBottom: '2rem' 
                }}>
                    {t('auth.description')}
                </p>

                <form onSubmit={handleLogin} style={{ textAlign: 'left' }}>
                    <div style={{ marginBottom: '1.5rem' }}>
                        <label style={{ 
                            display: 'block', 
                            color: 'var(--accent-cyan-70)', 
                            fontSize: '0.75rem', 
                            textTransform: 'uppercase',
                            letterSpacing: '0.1em',
                            marginBottom: '0.5rem',
                            fontWeight: 600
                        }}>
                            {t('auth.secretKeyLabel')}
                        </label>
                        <input 
                            type="password"
                            value={token}
                            onChange={(e) => setToken(e.target.value)}
                            placeholder="••••••••••••••••"
                            style={{
                                width: '100%',
                                background: 'var(--white-03)',
                                border: '1px solid var(--white-10)',
                                borderRadius: '12px',
                                padding: '1rem',
                                color: 'var(--text-primary)',
                                outline: 'none',
                                fontSize: '1rem',
                                transition: 'all 0.2s'
                            }}
                            autoFocus
                        />
                    </div>

                    <AnimatePresence>
                        {error && (
                            <motion.div 
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
                                    background: 'var(--accent-rose-05)',
                                    borderRadius: '10px',
                                    border: '1px solid var(--accent-rose-10)'
                                }}
                            >
                                <ShieldAlert size={16} />
                                <span>{error}</span>
                            </motion.div>
                        )}
                    </AnimatePresence>

                    <button
                        type="submit"
                        disabled={isLoading || !token}
                        style={{
                            width: '100%',
                            padding: '1rem',
                            borderRadius: '12px',
                            background: isLoading ? 'transparent' : 'linear-gradient(90deg, var(--accent-cyan), var(--accent-purple))',
                            border: 'none',
                            color: 'var(--bg-primary)',
                            fontWeight: 700,
                            fontSize: '1rem',
                            cursor: (isLoading || !token) ? 'not-allowed' : 'pointer',
                            display: 'flex',
                            alignItems: 'center',
                            justifyContent: 'center',
                            gap: '0.5rem',
                            opacity: (isLoading || !token) ? 0.5 : 1,
                            transition: 'all 0.2s'
                        }}
                    >
                        {isLoading ? (
                            <Loader2 className="animate-spin" size={20} color="var(--accent-cyan)" />
                        ) : (
                            <>
                                <Zap size={20} />
                                {t('auth.synchronize')}
                            </>
                        )}
                    </button>
                </form>

                <div style={{ marginTop: '2rem', fontSize: '0.75rem', color: 'var(--white-30)' }}>
                    {t('auth.gateway')}
                </div>
            </motion.div>
        </motion.div>
    );
};

export default AuthOverlay;

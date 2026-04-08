/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import React, { useState } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { BrainCircuit, Sparkles, Shield, User, UserCheck } from 'lucide-react';
import { useAvatarCharacter } from '../hooks/AvatarContext';
import { API_BASE } from '../config';
import { authenticatedFetch } from '../lib/auth';
import { useTranslation } from '../i18n';
import { ModelSetupStep } from './ModelSetupStep';

interface OnboardingModalProps {
    isOpen: boolean;
    onClose: () => void;
}

const OnboardingModal: React.FC<OnboardingModalProps> = ({ isOpen, onClose }) => {
    const { t } = useTranslation();
    const [step, setStep] = useState(0);
    const [aiName, setAiName] = useState("Watchtower");
    const { character, setCharacter, proportion, setProportion } = useAvatarCharacter();
    const [isSaving, setIsSaving] = useState(false);
    const [errorMsg, setErrorMsg] = useState<string | null>(null);
    const [viewMode, setViewMode] = useState<string>('intermediate');

    const handleFinalize = async () => {
        setIsSaving(true);
        setErrorMsg(null);
        try {
            // Save AI Name to DB
            await authenticatedFetch(`${API_BASE}/api/v1/settings`, {
                method: 'PUT',
                body: JSON.stringify({ key: 'ai_name', value: aiName, category: 'identity' })
            });
            // Save View Mode
            await authenticatedFetch(`${API_BASE}/api/v1/settings`, {
                method: 'PUT',
                body: JSON.stringify({ key: 'view_mode', value: viewMode, category: 'ui' })
            });
            // Initialize SOUL.md with LLM
            await authenticatedFetch(`${API_BASE}/api/v1/soul/init`, {
                method: 'POST',
                body: JSON.stringify({ ai_name: aiName })
            });

            onClose();
            // Force reload to apply BootMode::Normal changes and trigger system load
            window.location.reload();
        } catch (error: any) {
            console.error("Failed to save onboarding settings", error);
            setErrorMsg(error.message || "Failed to initialize AI startup. Is your LLM running?");
        } finally {
            setIsSaving(false);
        }
    };

    const steps = [
        {
            title: t('onboarding.welcome'),
            description: t('onboarding.welcomeDesc'),
            icon: <BrainCircuit size={48} color="var(--accent-cyan)" />,
            content: (
                <div className="onboarding-step active">
                    <Sparkles size={24} color="var(--accent-cyan)" />
                    <div style={{ fontSize: '0.9rem', color: 'var(--accent-cyan)', fontWeight: 700, letterSpacing: '0.1em' }}>
                        {t('onboarding.manifestInit')}
                    </div>
                </div>
            )
        },
        {
            title: t('onboarding.setupTitle'),
            description: t('onboarding.nameDesc'),
            icon: <User size={48} color="var(--accent-cyan)" />,
            content: (
                <div style={{ background: 'var(--white-03)', borderRadius: 'var(--radius-lg)', padding: '2rem', border: '1px solid var(--border-glass)' }}>
                    <div style={{ marginBottom: '1.5rem' }}>
                        <label style={{ display: 'block', fontSize: '0.7rem', color: 'var(--text-muted)', marginBottom: '0.5rem', fontWeight: 700 }}>AI AGENT NAME</label>
                        <input
                            type="text"
                            value={aiName}
                            onChange={(e) => setAiName(e.target.value)}
                            placeholder={t('onboarding.namePlaceholder')}
                            style={{
                                width: '100%',
                                background: 'var(--black-30)',
                                border: '1px solid var(--border-glass)',
                                borderRadius: 'var(--radius-md)',
                                padding: '1rem',
                                color: 'var(--text-primary)',
                                fontSize: '1.1rem',
                                outline: 'none'
                            }}
                        />
                    </div>
                </div>
            )
        },
        {
            title: t('onboarding.llmSetup.title') || "LLM Engine Setup",
            description: t('onboarding.llmSetup.desc') || "Select the AI engine that will power your companion.",
            icon: <BrainCircuit size={48} color="var(--accent-cyan)" />,
            content: <ModelSetupStep onNext={() => setStep(step + 1)} onSkip={() => setStep(step + 1)} />,
            hideNext: true // We hide the global "next" button for this step
        },
        {
            title: t('onboarding.chooseManifestation'),
            description: t('onboarding.manifestationDesc'),
            icon: <UserCheck size={48} color="var(--accent-purple)" />,
            content: (
                <div style={{ display: 'flex', flexDirection: 'column', gap: '1.5rem', width: '100%', marginTop: '0.5rem' }}>
                    <div style={{ display: 'flex', gap: '1rem', justifyContent: 'center' }}>
                         <button 
                            onClick={() => setCharacter('female')}
                            style={{ 
                                flex: 1, padding: '1rem', borderRadius: 'var(--radius-md)', 
                                border: `2px solid ${character === 'female' ? 'var(--accent-purple)' : 'transparent'}`,
                                background: character === 'female' ? 'var(--accent-purple-10)' : 'var(--white-03)',
                                cursor: 'pointer', transition: 'all var(--speed-normal) ease'
                            }}
                         >
                             <div style={{ fontSize: '1.5rem', marginBottom: '0.2rem' }}>♀</div>
                             <div style={{ fontSize: '0.8rem', fontWeight: 600 }}>{t('settings.female')}</div>
                         </button>
                         <button 
                            onClick={() => setCharacter('male')}
                            style={{ 
                                flex: 1, padding: '1rem', borderRadius: 'var(--radius-md)', 
                                border: `2px solid ${character === 'male' ? 'var(--accent-cyan)' : 'transparent'}`,
                                background: character === 'male' ? 'var(--accent-cyan-10)' : 'var(--white-03)',
                                cursor: 'pointer', transition: 'all var(--speed-normal) ease'
                            }}
                         >
                             <div style={{ fontSize: '1.5rem', marginBottom: '0.2rem' }}>♂</div>
                             <div style={{ fontSize: '0.8rem', fontWeight: 600 }}>{t('settings.male')}</div>
                         </button>
                    </div>
                    <div style={{ display: 'flex', gap: '1rem', justifyContent: 'center' }}>
                         <button 
                            onClick={() => setProportion('chibi')}
                            style={{ 
                                flex: 1, padding: '0.8rem', borderRadius: 'var(--radius-md)', 
                                border: `2px solid ${proportion === 'chibi' ? 'var(--accent-cyan)' : 'transparent'}`,
                                background: proportion === 'chibi' ? 'var(--accent-cyan-05)' : 'var(--white-03)',
                                fontSize: '0.8rem', cursor: 'pointer'
                            }}
                         >
                             {t('onboarding.cuteChibi')}
                         </button>
                         <button 
                            onClick={() => setProportion('taller')}
                            style={{ 
                                flex: 1, padding: '0.8rem', borderRadius: 'var(--radius-md)', 
                                border: `2px solid ${proportion === 'taller' ? 'var(--accent-cyan)' : 'transparent'}`,
                                background: proportion === 'taller' ? 'var(--accent-cyan-05)' : 'var(--white-03)',
                                fontSize: '0.8rem', cursor: 'pointer'
                            }}
                         >
                             {t('onboarding.modernTaller')}
                         </button>
                    </div>
                </div>
            )
        },
        {
            title: t('onboarding.abyssSecurity'),
            description: t('onboarding.abyssSecurityDesc'),
            icon: <Shield size={48} color="var(--accent-rose)" />,
        },
        {
            title: t('onboarding.chooseExperience'),
            description: t('onboarding.chooseExperienceDesc'),
            icon: <Sparkles size={48} color="var(--accent-cyan)" />,
            content: (
                <div style={{ display: 'flex', flexDirection: 'column', gap: '0.8rem', width: '100%', marginTop: '0.5rem' }}>
                    {[
                        { id: 'beginner', label: t('onboarding.beginner'), desc: t('onboarding.beginnerDesc') },
                        { id: 'intermediate', label: t('onboarding.intermediate'), desc: t('onboarding.intermediateDesc') },
                        { id: 'advanced', label: t('onboarding.advanced'), desc: t('onboarding.advancedDesc') }
                    ].map((lvl) => (
                        <button
                            key={lvl.id}
                            onClick={() => {
                                setViewMode(lvl.id);
                                setStep(steps.length - 1); // Skip to last or trigger handleFinalize
                            }}
                            style={{
                                padding: '1rem', borderRadius: 'var(--radius-md)', textAlign: 'left',
                                border: '1px solid var(--border-glass-bright)',
                                background: 'var(--white-03)',
                                cursor: 'pointer', transition: 'all var(--speed-normal) ease'
                            }}
                        >
                            <div style={{ fontWeight: 800, color: 'var(--accent-cyan)' }}>{lvl.label}</div>
                            <div style={{ fontSize: '0.8rem', color: 'var(--text-secondary)' }}>{lvl.desc}</div>
                        </button>
                    ))}
                </div>
            )
        }
    ];

    return (
        <AnimatePresence>
            {isOpen && (
                <motion.div
                    className="modal-overlay"
                    initial={{ opacity: 0 }}
                    animate={{ opacity: 1 }}
                    exit={{ opacity: 0 }}
                    style={{
                        position: 'fixed',
                        inset: 0,
                        background: 'var(--black-85)',
                        backdropFilter: 'blur(20px)',
                        display: 'flex',
                        alignItems: 'center',
                        justifyContent: 'center',
                        zIndex: 1000,
                    }}
                >
                    <motion.div
                        initial={{ scale: 0.9, opacity: 0, y: 20 }}
                        animate={{ scale: 1, opacity: 1, y: 0 }}
                        exit={{ scale: 0.9, opacity: 0, y: 20 }}
                        className="modal-container"
                        style={{
                            width: '500px',
                            minHeight: '520px',
                            padding: '3rem',
                            background: 'var(--bg-glass-heavy)',
                            border: '1px solid var(--border-glass-bright)',
                            borderRadius: 'var(--radius-xl)',
                            textAlign: 'center',
                            boxShadow: 'var(--shadow-deep)',
                            display: 'flex',
                            flexDirection: 'column'
                        }}
                    >
                        <div style={{ flex: 1 }}>
                            <AnimatePresence mode="wait">
                                <motion.div
                                    key={step}
                                    initial={{ x: 20, opacity: 0 }}
                                    animate={{ x: 0, opacity: 1 }}
                                    exit={{ x: -20, opacity: 0 }}
                                    style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', gap: '1.2rem' }}
                                >
                                    <div style={{ padding: '2rem', background: 'var(--white-03)', borderRadius: '50%', marginBottom: '0.5rem' }}>
                                        {steps[step].icon}
                                    </div>
                                    <h2 style={{ fontSize: '1.8rem', fontWeight: 800 }}>{steps[step].title}</h2>
                                    <p style={{ color: 'var(--text-secondary)', lineHeight: 1.6, fontSize: '1rem' }}>
                                        {steps[step].description}
                                    </p>
                                    {steps[step].content}
                                </motion.div>
                            </AnimatePresence>
                        </div>

                        <div style={{ marginTop: '2rem' }}>
                            <div style={{ display: 'flex', justifyContent: 'center', gap: '1rem' }}>
                                {(!steps[step].hideNext && step < steps.length - 1) ? (
                                    <button
                                        onClick={() => setStep(step + 1)}
                                        style={{
                                            padding: '0.8rem 2.5rem',
                                            background: 'var(--accent-cyan)',
                                            color: 'var(--text-inverse)',
                                            border: 'none',
                                            borderRadius: 'var(--radius-md)',
                                            fontWeight: 700,
                                            cursor: 'pointer',
                                        }}
                                    >
                                        {t('onboarding.next')}
                                    </button>
                                ) : (
                                    <button
                                        onClick={handleFinalize}
                                        disabled={isSaving}
                                        style={{
                                            padding: '0.8rem 2.5rem',
                                            background: 'linear-gradient(135deg, var(--accent-cyan), var(--accent-purple))',
                                            color: 'var(--text-inverse)',
                                            border: 'none',
                                            borderRadius: 'var(--radius-md)',
                                            fontWeight: 700,
                                            cursor: 'pointer',
                                            display: 'flex',
                                            alignItems: 'center',
                                            gap: '0.5rem',
                                            opacity: isSaving ? 0.7 : 1
                                        }}
                                    >
                                        <Sparkles size={20} />
                                        {isSaving ? t('onboarding.finalizing') : t('onboarding.awaken')}
                                    </button>
                                )}
                            </div>

                            <AnimatePresence>
                                {errorMsg && (
                                    <motion.div
                                        initial={{ opacity: 0, height: 0 }}
                                        animate={{ opacity: 1, height: 'auto' }}
                                        exit={{ opacity: 0, height: 0 }}
                                        style={{ marginTop: '1rem', color: 'var(--accent-rose)', fontSize: '0.9rem', fontWeight: 600 }}
                                    >
                                        {errorMsg}
                                    </motion.div>
                                )}
                            </AnimatePresence>

                            <div style={{ marginTop: '2rem', display: 'flex', justifyContent: 'center', gap: '0.5rem' }}>
                                {steps.map((_, i) => (
                                    <div
                                        key={i}
                                        style={{
                                            width: '8px',
                                            height: '8px',
                                            borderRadius: '50%',
                                            background: i === step ? 'var(--accent-cyan)' : 'var(--text-muted)',
                                            transition: 'all var(--speed-normal) ease'
                                        }}
                                    />
                                ))}
                            </div>
                        </div>
                    </motion.div>
                </motion.div>
            )}
        </AnimatePresence>
    );
};

export default OnboardingModal;

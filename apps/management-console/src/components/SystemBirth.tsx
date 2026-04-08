/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import React, { useEffect, useState } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { useTranslation } from '../i18n';

interface SystemBirthProps {
    onComplete: () => void;
}

const SYNAPSES = [...Array(20)].map((_, i) => ({
    id: i,
    x: (Math.random() - 0.5) * 1000,
    y: (Math.random() - 0.5) * 1000,
    duration: 2 + Math.random() * 2,
    delay: Math.random() * 2,
    isCyan: Math.random() > 0.5
}));

const SystemBirth: React.FC<SystemBirthProps> = ({ onComplete }) => {
    const { t } = useTranslation();
    const [phase, setPhase] = useState(0);

    useEffect(() => {
        const t1 = setTimeout(() => setPhase(1), 800);
        const t2 = setTimeout(() => setPhase(2), 2200);
        const t3 = setTimeout(() => onComplete(), 4500);

        return () => {
            clearTimeout(t1);
            clearTimeout(t2);
            clearTimeout(t3);
        };
    }, [onComplete]);

    return (
        <div style={{
            position: 'fixed',
            inset: 0,
            background: 'var(--bg-primary)',
            zIndex: 2000,
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            overflow: 'hidden'
        }}>
            {/* Background Grid */}
            <div style={{
                position: 'absolute',
                inset: 0,
                background: 'linear-gradient(var(--accent-cyan-05) 1px, transparent 1px), linear-gradient(90deg, var(--accent-cyan-05) 1px, transparent 1px)',
                backgroundSize: '40px 40px',
                opacity: phase >= 1 ? 1 : 0,
                transition: 'opacity var(--speed-genesis) ease'
            }} />

            <AnimatePresence>
                {phase === 1 && (
                    <motion.div
                        initial={{ scale: 0, opacity: 0 }}
                        animate={{ scale: 1.5, opacity: 0.8 }}
                        exit={{ scale: 3, opacity: 0 }}
                        transition={{ duration: 1.5, ease: "easeOut" }}
                        style={{
                            width: '100px',
                            height: '100px',
                            borderRadius: '50%',
                            background: 'radial-gradient(circle, var(--accent-cyan), transparent 70%)',
                            filter: 'blur(10px)',
                            position: 'absolute'
                        }}
                    />
                )}
            </AnimatePresence>

            <div style={{ textAlign: 'center', zIndex: 10 }}>
                {phase >= 1 && (
                    <motion.div
                        initial={{ opacity: 0, y: 20 }}
                        animate={{ opacity: 1, y: 0 }}
                        style={{ marginBottom: '2rem' }}
                    >
                        <h1 className="font-display" style={{
                            fontSize: '4rem',
                            fontWeight: 900,
                            letterSpacing: '0.4em',
                            background: 'linear-gradient(135deg, var(--text-primary) 30%, var(--accent-cyan) 100%)',
                            WebkitBackgroundClip: 'text',
                            WebkitTextFillColor: 'transparent',
                            textShadow: '0 0 30px var(--accent-cyan-30)'
                        }}>
                            AIOME
                        </h1>
                    </motion.div>
                )}

                {phase === 2 && (
                    <motion.div
                        initial={{ opacity: 0 }}
                        animate={{ opacity: 1 }}
                        style={{ display: 'flex', flexDirection: 'column', gap: '0.5rem', alignItems: 'center' }}
                    >
                        <div style={{ fontSize: '0.9rem', color: 'var(--accent-purple)', letterSpacing: '0.2em' }}>
                            {t('system.calibrating')}
                        </div>
                        <div style={{ width: '300px', height: '2px', background: 'var(--white-10)', overflow: 'hidden', borderRadius: '1px' }}>
                            <motion.div
                                initial={{ x: '-100%' }}
                                animate={{ x: '100%' }}
                                transition={{ duration: 2, ease: "easeInOut" }}
                                style={{ width: '100%', height: '100%', background: 'var(--accent-cyan)' }}
                            />
                        </div>
                        <div style={{ fontSize: '0.7rem', color: 'var(--text-muted)', marginTop: '0.5rem' }}>
                            {t('system.genesisProtocol')}
                        </div>
                    </motion.div>
                )}
            </div>

            {/* Decorative Synapses */}
            {phase >= 1 && SYNAPSES.map((s) => (
                <motion.div
                    key={s.id}
                    initial={{ opacity: 0 }}
                    animate={{
                        opacity: [0, 1, 0],
                        x: [0, s.x],
                        y: [0, s.y]
                    }}
                    transition={{
                        duration: s.duration,
                        repeat: Infinity,
                        delay: s.delay
                    }}
                    style={{
                        position: 'absolute',
                        width: '2px',
                        height: '2px',
                        background: s.isCyan ? 'var(--accent-cyan)' : 'var(--accent-purple)',
                        boxShadow: '0 0 8px currentColor'
                    }}
                />
            ))}
        </div>
    );
};

export default SystemBirth;

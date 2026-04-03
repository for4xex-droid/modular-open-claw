/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import React, { useEffect, useRef } from 'react';
import { motion } from 'framer-motion';
import { Bot, Send, Cpu, Brain, Sparkles, ThumbsUp, ThumbsDown, BookOpen } from 'lucide-react';
import { Volume2, VolumeX } from 'lucide-react';
import { useTranslation } from '../i18n';
import { useAgentChat } from '../hooks/useAgentChat';

const AgentConsole: React.FC = () => {
    const { t } = useTranslation();
    const {
        history,
        input,
        isTyping,
        streamingText,
        status,
        autoTts,
        relevantKarma,
        relevantKarmaData,
        activeKnowledge,
        setInput,
        sendMessage,
        setAutoTts,
        handleFeedback,
    } = useAgentChat();

    const chatEndRef = useRef<HTMLDivElement>(null);

    const scrollToBottom = () => {
        chatEndRef.current?.scrollIntoView({ behavior: "smooth" });
    };

    useEffect(scrollToBottom, [history, streamingText]);

    return (
        <div className="main-panel ani-fade" style={{ height: '78vh', display: 'flex', flexDirection: 'column', padding: 0, overflow: 'hidden' }}>
            {/* Header */}
            <div className="panel-header" style={{ padding: '1rem 1.5rem', borderBottom: '1px solid var(--border-glass)' }}>
                <div style={{ display: 'flex', alignItems: 'center', gap: '0.75rem' }}>
                    <div style={{ position: 'relative' }}>
                        <Bot size={24} color="var(--accent-cyan)" />
                        {isTyping && (
                            <motion.div
                                animate={{ scale: [1, 1.5, 1], opacity: [1, 0, 1] }}
                                transition={{ duration: 1, repeat: Infinity }}
                                style={{ position: 'absolute', inset: -2, border: '2px solid var(--accent-cyan)', borderRadius: '50%' }}
                            />
                        )}
                    </div>
                    <div>
                        <h3 style={{ fontSize: '1rem', fontWeight: 700 }}>{t('agent.title')}</h3>
                        <div style={{ fontSize: '0.7rem', color: isTyping ? 'var(--accent-cyan)' : 'var(--text-muted)', display: 'flex', alignItems: 'center', gap: '0.4rem' }}>
                            <span style={{ width: 6, height: 6, borderRadius: '50%', background: isTyping ? 'var(--accent-cyan)' : 'var(--accent-emerald)' }} />
                            {status}
                        </div>
                    </div>
                </div>
                <div style={{ display: 'flex', gap: '0.8rem', alignItems: 'center' }}>
                    <button 
                        onClick={() => setAutoTts(!autoTts)}
                        className="stat-badge" 
                        style={{ 
                            fontSize: '0.7rem', 
                            background: autoTts ? 'rgba(0, 243, 255, 0.1)' : 'rgba(255,255,255,0.03)',
                            border: `1px solid ${autoTts ? 'var(--accent-cyan)' : 'transparent'}`,
                            cursor: 'pointer',
                            display: 'flex',
                            alignItems: 'center',
                            gap: '0.4rem',
                            color: autoTts ? 'var(--accent-cyan)' : 'var(--text-muted)'
                        }}
                    >
                        {autoTts ? <Volume2 size={12} /> : <VolumeX size={12} />}
                        VOICE: {autoTts ? 'ON' : 'OFF'}
                    </button>
                    <div className="stat-badge" style={{ fontSize: '0.7rem', background: 'rgba(255,255,255,0.03)' }}>3.5B MODEL</div>
                </div>
            </div>

            {/* Chat Area */}
            <div style={{ flex: 1, overflowY: 'auto', padding: '2rem', display: 'flex', flexDirection: 'column', gap: '1.5rem', background: 'rgba(0,0,0,0.2)' }}>
                {history.length === 0 && !streamingText && (
                    <div style={{ flex: 1, display: 'flex', flexDirection: 'column', alignItems: 'center', justifyContent: 'center', color: 'var(--text-muted)', textAlign: 'center' }}>
                        <Cpu size={48} style={{ opacity: 0.1, marginBottom: '1.5rem' }} />
                        <h4 style={{ fontWeight: 600, color: 'rgba(255,255,255,0.2)' }}>{t('agent.ready')}</h4>
                        <p style={{ fontSize: '0.85rem', maxWidth: '300px', marginTop: '0.5rem' }}>Issue natural language commands to synthesize skills or explore the biome.</p>
                    </div>
                )}

                {relevantKarma && (
                    <motion.div
                        initial={{ opacity: 0, y: -10 }}
                        animate={{ opacity: 1, y: 0 }}
                        className="glass-panel"
                        style={{
                            padding: '1.2rem',
                            background: relevantKarma.includes('見つかりませんでした')
                                ? 'rgba(255, 82, 82, 0.08)'
                                : 'rgba(0, 243, 255, 0.03)',
                            border: `1px solid ${relevantKarma.includes('見つかりませんでした') ? 'rgba(255, 82, 82, 0.2)' : 'rgba(0, 243, 255, 0.1)'}`,
                            borderLeftWidth: '4px',
                            borderLeftColor: relevantKarma.includes('見つかりませんでした') ? '#ff5252' : '#00f3ff',
                            fontSize: '0.8rem',
                            marginBottom: '1rem',
                        }}
                    >
                        <div style={{ fontWeight: 800, fontSize: '0.7rem', color: 'rgba(255,255,255,0.5)', marginBottom: '0.8rem', display: 'flex', alignItems: 'center', gap: '0.6rem', letterSpacing: '0.1em' }}>
                            <Brain size={14} color={relevantKarma.includes('見つかりませんでした') ? '#ff5252' : '#00f3ff'} />
                            {relevantKarma.includes('見つかりませんでした') ? 'OUT-OF-DOMAIN DETECTED' : 'SYNAPTIC MEMORY RETRIEVED'}
                        </div>
                        <div style={{ whiteSpace: 'pre-wrap', lineHeight: 1.5, color: 'rgba(255,255,255,0.8)' }}>
                            {relevantKarma}
                        </div>
                    </motion.div>
                )}

                {activeKnowledge && (
                    <motion.div
                        initial={{ opacity: 0, y: -10 }}
                        animate={{ opacity: 1, y: 0 }}
                        style={{
                            padding: '1rem',
                            background: 'rgba(255, 171, 0, 0.1)',
                            border: '1px solid rgba(255, 171, 0, 0.2)',
                            borderLeft: '4px solid #ffab00',
                            borderRadius: '4px 12px 12px 4px',
                            marginBottom: '1rem',
                        }}
                    >
                        <div style={{ fontWeight: 800, fontSize: '0.7rem', color: 'rgba(255,171,0,0.8)', marginBottom: '0.5rem', display: 'flex', alignItems: 'center', gap: '0.6rem', letterSpacing: '0.1em' }}>
                            <BookOpen size={14} />
                            PROJECT KNOWLEDGE ACCESSED
                        </div>
                        <div style={{ fontSize: '0.85rem', color: 'rgba(255,255,255,0.9)', fontWeight: 500 }}>
                            {activeKnowledge}
                        </div>
                    </motion.div>
                )}

                {history.map((m, i) => (
                    <motion.div
                        key={i}
                        initial={{ opacity: 0, x: m.role === 'user' ? 20 : -20 }}
                        animate={{ opacity: 1, x: 0 }}
                        style={{
                            alignSelf: m.role === 'user' ? 'flex-end' : 'flex-start',
                            maxWidth: '85%',
                            display: 'flex',
                            flexDirection: 'column',
                            alignItems: m.role === 'user' ? 'flex-end' : 'flex-start',
                            gap: '0.5rem'
                        }}
                    >
                        <div style={{ fontSize: '0.7rem', color: 'var(--text-muted)', fontWeight: 700, letterSpacing: '0.05em' }}>
                            {m.role === 'user' ? 'OPERATOR' : 'AIOME'}
                        </div>
                        <div style={{
                            padding: '1.25rem',
                            borderRadius: m.role === 'user' ? '20px 20px 4px 20px' : '4px 20px 20px 20px',
                            background: m.role === 'user' ? 'var(--accent-cyan-glass)' : 'var(--bg-glass-heavy)',
                            border: m.role === 'user' ? '1px solid rgba(0, 242, 255, 0.3)' : '1px solid var(--border-glass)',
                            color: m.isError ? 'var(--accent-rose)' : 'var(--text-primary)',
                            fontSize: '0.95rem',
                            lineHeight: 1.6,
                            boxShadow: '0 4px 15px rgba(0,0,0,0.1)',
                            whiteSpace: 'pre-wrap'
                        }}>
                            {m.content}
                        </div>

                        {m.role === 'assistant' && !m.isError && i === history.length - 1 && (relevantKarmaData?.entries?.length ?? 0) > 0 && (
                            <div style={{ display: 'flex', gap: '0.5rem', marginTop: '0.2rem', opacity: 0.6 }}>
                                <button
                                    onClick={() => handleFeedback(i, 'positive')}
                                    style={{ background: 'none', border: 'none', cursor: 'pointer', color: 'var(--text-muted)' }}
                                    title="Helpful Lesson"
                                >
                                    <ThumbsUp size={14} />
                                </button>
                                <button
                                    onClick={() => handleFeedback(i, 'negative')}
                                    style={{ background: 'none', border: 'none', cursor: 'pointer', color: 'var(--text-muted)' }}
                                    title="Not Helpful Lesson"
                                >
                                    <ThumbsDown size={14} />
                                </button>
                            </div>
                        )}
                    </motion.div>
                ))}

                {streamingText && (
                    <motion.div
                        initial={{ opacity: 0 }}
                        animate={{ opacity: 1 }}
                        style={{ alignSelf: 'flex-start', maxWidth: '85%', display: 'flex', flexDirection: 'column', gap: '0.5rem' }}
                    >
                        <div style={{ fontSize: '0.7rem', color: 'var(--accent-cyan)', fontWeight: 700 }}>AIOME (STREAMING)</div>
                        <div style={{
                            padding: '1.25rem',
                            borderRadius: '4px 20px 20px 20px',
                            background: 'var(--bg-glass-heavy)',
                            border: '1px solid var(--accent-cyan-glass)',
                            fontSize: '0.95rem',
                            lineHeight: 1.6,
                            whiteSpace: 'pre-wrap'
                        }}>
                            {streamingText}
                            <motion.span
                                animate={{ opacity: [0, 1, 0] }}
                                transition={{ duration: 0.8, repeat: Infinity }}
                                style={{ display: 'inline-block', width: '8px', height: '1.2em', background: 'var(--accent-cyan)', marginLeft: '4px', verticalAlign: 'middle' }}
                            />
                        </div>
                    </motion.div>
                )}

                <div ref={chatEndRef} />
            </div>

            {/* Input Area */}
            <div style={{ padding: '1.5rem 2rem', background: 'rgba(0,0,0,0.4)', borderTop: '1px solid var(--border-glass)' }}>
                <div style={{ position: 'relative' }}>
                    <textarea
                        value={input}
                        onChange={e => setInput(e.target.value)}
                        onKeyDown={e => {
                            if (e.nativeEvent.isComposing) return;
                            if (e.key === 'Enter' && !e.shiftKey) {
                                e.preventDefault();
                                sendMessage();
                            }
                        }}
                        placeholder="Type a command or ask a question..."
                        rows={1}
                        style={{
                            width: '100%',
                            background: 'rgba(255,255,255,0.03)',
                            border: '1px solid var(--border-glass)',
                            borderRadius: '16px',
                            padding: '1.2rem 4rem 1.2rem 1.5rem',
                            color: '#fff',
                            outline: 'none',
                            fontSize: '1rem',
                            resize: 'none',
                            transition: 'all 0.3s ease',
                            boxShadow: 'inset 0 2px 4px rgba(0,0,0,0.2)'
                        }}
                    />
                    <button
                        onClick={sendMessage}
                        disabled={!input.trim() || isTyping}
                        style={{
                            position: 'absolute',
                            right: '8px',
                            top: '50%',
                            transform: 'translateY(-50%)',
                            width: '44px',
                            height: '44px',
                            borderRadius: '12px',
                            background: input.trim() && !isTyping ? 'var(--accent-cyan)' : 'rgba(255,255,255,0.05)',
                            color: input.trim() && !isTyping ? '#000' : 'rgba(255,255,255,0.2)',
                            border: 'none',
                            cursor: 'pointer',
                            display: 'flex',
                            alignItems: 'center',
                            justifyContent: 'center',
                            transition: 'all 0.2s ease'
                        }}
                    >
                        <Send size={20} />
                    </button>
                </div>
                <div style={{ marginTop: '0.75rem', display: 'flex', justifyContent: 'space-between', alignItems: 'center', padding: '0 0.5rem' }}>
                    <div style={{ display: 'flex', gap: '1rem' }}>
                        <div style={{ display: 'flex', alignItems: 'center', gap: '0.4rem', fontSize: '0.7rem', color: 'var(--text-muted)' }}>
                            <kbd style={{ background: 'rgba(255,255,255,0.1)', padding: '2px 4px', borderRadius: '4px' }}>Shift+Enter</kbd> to newline
                        </div>
                    </div>
                    <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem', fontSize: '0.7rem', color: 'var(--text-muted)' }}>
                        <Sparkles size={12} color="var(--accent-purple)" /> PROMPT ENHANCEMENT ACTIVE
                    </div>
                </div>
            </div>
        </div>
    );
};

export default AgentConsole;

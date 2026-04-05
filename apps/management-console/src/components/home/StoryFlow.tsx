/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import React, { useEffect, useRef, useMemo, useState } from 'react';
import { AnimatePresence, motion } from 'framer-motion';
import { Send, Sparkles, Volume2, VolumeX, Cpu, Wifi, WifiOff } from 'lucide-react';
import FlowCard, { FlowCardType } from './FlowCard';
import { useAgentChat } from '../../hooks/useAgentChat';
import { VitalityEvent } from '../../hooks/useSystemVitality';
import { useTranslation } from '../../i18n';
import { useCortexSuggestions } from '../../hooks/useCortexSuggestions';

/** Unified timeline entry for rendering */
interface TimelineEntry {
    id: string;
    type: FlowCardType;
    title: string;
    content: string;
    timestamp: number;
    isError?: boolean;
    isOod?: boolean;
    showFeedback?: boolean;
}

/**
 * Map a VitalityEvent to a TimelineEntry for display.
 */
const mapVitalityEvent = (event: VitalityEvent, index: number): TimelineEntry | null => {
    const ts = Date.now() - index * 100; // approximate ordering
    const d = (event.data || {}) as Record<string, unknown>;

    switch (event.type) {
        case 'level_up':
            return { id: `sys-${index}`, type: 'system', title: 'LEVEL UP', content: `Level ${String(d?.level ?? '?')} reached! 🎉`, timestamp: ts };
        case 'karma_update':
            return { id: `sys-${index}`, type: 'system', title: 'KARMA', content: `${String(d?.lesson ?? 'Memory updated')}`, timestamp: ts };
        case 'job_started':
            return { id: `sys-${index}`, type: 'tool_exec', title: 'JOB STARTED', content: `${String(d?.job_type ?? 'Task')} initiated`, timestamp: ts };
        case 'job_completed':
            return { id: `sys-${index}`, type: 'system', title: 'JOB COMPLETE', content: `Task finished successfully`, timestamp: ts };
        case 'skill_loaded':
        case 'skill_ready':
            return { id: `sys-${index}`, type: 'system', title: 'SKILL', content: `${String(d?.name ?? 'Skill')} loaded`, timestamp: ts };
        case 'immune_alert':
            return { id: `sys-${index}`, type: 'system', title: '🛡️ IMMUNE ALERT', content: `${String(d?.message ?? 'Security event detected')}`, timestamp: ts };
        case 'skill_execution':
            return { id: `sys-${index}`, type: 'tool_exec', title: 'EXECUTING', content: `${String(d?.name ?? 'Tool')}`, timestamp: ts };
        case 'proactive_talk':
            return { id: `sys-${index}`, type: 'chat_assistant', title: 'AIOME (PROACTIVE)', content: `${String(d?.message ?? '')}`, timestamp: ts };
        case 'sot_progress':
            return { id: `sys-${index}`, type: 'system', title: 'SOCIETY OF THOUGHT', content: `${String(d?.message ?? 'Deliberation in progress')}`, timestamp: ts };
        default:
            return null;
    }
};

interface StoryFlowProps {
    sysEvents?: VitalityEvent[];
    connectionStatus?: string;
}

const StoryFlow: React.FC<StoryFlowProps> = ({ sysEvents = [], connectionStatus = 'disconnected' }) => {
    const { t } = useTranslation();
    const chat = useAgentChat();
    const scrollRef = useRef<HTMLDivElement>(null);
    const [isInputFocused, setIsInputFocused] = useState(false);
    const { suggestions, fetchSuggestions } = useCortexSuggestions();

    useEffect(() => {
        if (isInputFocused) {
            fetchSuggestions();
        }
    }, [isInputFocused, fetchSuggestions]);

    // Build unified timeline: chat history + system events
    const timeline = useMemo<TimelineEntry[]>(() => {
        const entries: TimelineEntry[] = [];
        const baseTime = Date.now();

        // 1. Chat history entries
        chat.history.forEach((msg, i) => {
            entries.push({
                id: `chat-${i}`,
                type: msg.role === 'user' ? 'chat_user' : 'chat_assistant',
                title: msg.role === 'user' ? 'OPERATOR' : 'AIOME',
                content: msg.content,
                timestamp: baseTime - (chat.history.length - i) * 1000,
                isError: msg.isError,
                showFeedback: msg.role === 'assistant' && !msg.isError && i === chat.history.length - 1 && (chat.relevantKarmaData?.entries?.length ?? 0) > 0,
            });
        });

        // 2. Karma context (if available, insert before last assistant message)
        if (chat.relevantKarma) {
            entries.push({
                id: 'karma-context',
                type: 'karma',
                title: chat.relevantKarma.includes('見つかりませんでした') ? 'OUT-OF-DOMAIN' : 'MEMORY RETRIEVED',
                content: chat.relevantKarma,
                timestamp: baseTime - 500,
                isOod: chat.relevantKarma.includes('見つかりませんでした'),
            });
        }

        // 3. Knowledge context
        if (chat.activeKnowledge) {
            entries.push({
                id: 'knowledge-context',
                type: 'knowledge',
                title: 'PROJECT KNOWLEDGE',
                content: chat.activeKnowledge,
                timestamp: baseTime - 400,
            });
        }

        // 4. Streaming text
        if (chat.streamingText) {
            entries.push({
                id: 'streaming',
                type: 'chat_streaming',
                title: 'AIOME (STREAMING)',
                content: chat.streamingText,
                timestamp: baseTime,
            });
        }

        // 5. System events (latest few, avoiding excessive noise)
        const recentSysEvents = sysEvents.slice(0, 8);
        recentSysEvents.forEach((evt, i) => {
            const mapped = mapVitalityEvent(evt, i);
            if (mapped) entries.push(mapped);
        });

        // Sort by timestamp (oldest first)
        entries.sort((a, b) => a.timestamp - b.timestamp);

        return entries;
    }, [chat.history, chat.streamingText, chat.relevantKarma, chat.activeKnowledge, chat.relevantKarmaData, sysEvents]);

    // Auto-scroll on new entries
    useEffect(() => {
        if (scrollRef.current) {
            scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
        }
    }, [timeline]);

    return (
        <div className="story-flow" style={{
            background: 'var(--panel-bg)',
            border: '1px solid var(--border-glass)',
            borderRadius: '16px',
            height: '100%',
            display: 'flex',
            flexDirection: 'column',
            overflow: 'hidden'
        }}>
            {/* Header */}
            <div style={{
                padding: '1rem 1.5rem',
                borderBottom: '1px solid var(--border-glass)',
                display: 'flex',
                justifyContent: 'space-between',
                alignItems: 'center',
                background: 'rgba(255,255,255,0.01)',
            }}>
                <h2 style={{ margin: 0, fontSize: '1.15rem', display: 'flex', alignItems: 'center', gap: '0.5rem', fontWeight: 900, letterSpacing: '0.05em', textTransform: 'uppercase' as const }}>
                    <span style={{
                        display: 'inline-block', width: '8px', height: '8px', borderRadius: '50%',
                        background: connectionStatus === 'connected' ? 'var(--accent-emerald)' : 'var(--accent-amber)',
                        boxShadow: connectionStatus === 'connected' ? '0 0 10px var(--accent-emerald)' : 'none',
                    }} />
                    Active Feed
                </h2>
                <div style={{ display: 'flex', gap: '0.6rem', alignItems: 'center' }}>
                    {/* TTS Toggle */}
                    <button
                        onClick={() => chat.setAutoTts(!chat.autoTts)}
                        style={{
                            background: chat.autoTts ? 'rgba(0, 243, 255, 0.1)' : 'rgba(255,255,255,0.03)',
                            border: `1px solid ${chat.autoTts ? 'rgba(0,243,255,0.3)' : 'transparent'}`,
                            borderRadius: '8px',
                            padding: '4px 10px',
                            cursor: 'pointer',
                            display: 'flex',
                            alignItems: 'center',
                            gap: '0.3rem',
                            fontSize: '0.65rem',
                            fontWeight: 700,
                            color: chat.autoTts ? 'var(--accent-cyan)' : 'var(--text-muted)',
                        }}
                    >
                        {chat.autoTts ? <Volume2 size={11} /> : <VolumeX size={11} />}
                        TTS
                    </button>

                    {/* Connection Status */}
                    <div style={{
                        display: 'flex',
                        alignItems: 'center',
                        gap: '0.3rem',
                        fontSize: '0.6rem',
                        fontWeight: 600,
                        color: connectionStatus === 'connected' ? 'var(--accent-emerald)' : 'var(--text-muted)',
                    }}>
                        {connectionStatus === 'connected' ? <Wifi size={11} /> : <WifiOff size={11} />}
                        SSE
                    </div>

                    {/* Status Badge */}
                    {chat.status !== 'IDLE' && (
                        <motion.div
                            initial={{ opacity: 0, scale: 0.8 }}
                            animate={{ opacity: 1, scale: 1 }}
                            style={{
                                fontSize: '0.6rem',
                                fontWeight: 800,
                                color: '#000',
                                background: 'var(--accent-amber)',
                                padding: '2px 8px',
                                borderRadius: '6px',
                                letterSpacing: '0.05em',
                            }}
                        >
                            {chat.status}
                        </motion.div>
                    )}
                </div>
            </div>

            {/* Feed Area */}
            <div
                ref={scrollRef}
                style={{
                    flex: 1,
                    overflowY: 'auto',
                    padding: '1.25rem',
                    display: 'flex',
                    flexDirection: 'column',
                    gap: '0.75rem',
                    background: 'rgba(0,0,0,0.15)',
                }}
            >
                {timeline.length === 0 && (
                    <div style={{
                        flex: 1,
                        display: 'flex',
                        flexDirection: 'column',
                        alignItems: 'center',
                        justifyContent: 'center',
                        color: 'var(--text-muted)',
                        textAlign: 'center',
                        gap: '0.75rem',
                    }}>
                        <Cpu size={40} style={{ opacity: 0.1 }} />
                        <div className="artemis-status" style={{ color: 'rgba(255,255,255,0.15)', fontSize: '0.95rem' }}>
                            {t('agent.ready')}
                        </div>
                        <p style={{ fontSize: '0.8rem', maxWidth: '280px', lineHeight: 1.5, opacity: 0.4 }}>
                            Send a message to start a conversation. System events will appear here in real-time.
                        </p>
                    </div>
                )}

                <AnimatePresence mode="popLayout">
                    {timeline.map(entry => (
                        <FlowCard
                            key={entry.id}
                            type={entry.type}
                            title={entry.title}
                            content={entry.content}
                            timestamp={entry.timestamp}
                            isError={entry.isError}
                            isStreaming={entry.type === 'chat_streaming'}
                            isOod={entry.isOod}
                            showFeedback={entry.showFeedback}
                            onFeedback={entry.showFeedback ? (type: 'positive' | 'negative') => chat.handleFeedback(0, type) : undefined}
                        />
                    ))}
                </AnimatePresence>
            </div>

            {/* Input Area */}
            <AnimatePresence>
                {isInputFocused && suggestions.length > 0 && (
                    <motion.div
                        initial={{ opacity: 0, y: 10 }}
                        animate={{ opacity: 1, y: 0 }}
                        exit={{ opacity: 0, y: 5 }}
                        className="cortex-suggestions"
                        style={{
                            display: 'flex',
                            gap: '0.4rem',
                            padding: '0 1rem 0.5rem 1rem',
                            overflowX: 'auto',
                            scrollbarWidth: 'none',
                        }}
                    >
                        {suggestions.map((sug, i) => (
                            <button
                                key={i}
                                className="suggestion-chip"
                                onClick={() => {
                                    chat.setInput(sug);
                                }}
                                style={{
                                    background: 'rgba(0, 243, 255, 0.08)',
                                    border: '1px solid rgba(0, 243, 255, 0.25)',
                                    color: 'var(--accent-cyan)',
                                    borderRadius: '16px',
                                    padding: '0.35rem 0.75rem',
                                    fontSize: '0.75rem',
                                    fontWeight: 600,
                                    cursor: 'pointer',
                                    whiteSpace: 'nowrap',
                                    transition: 'all 0.2s',
                                }}
                                onMouseEnter={e => { (e.target as HTMLButtonElement).style.background = 'rgba(0, 243, 255, 0.15)'; }}
                                onMouseLeave={e => { (e.target as HTMLButtonElement).style.background = 'rgba(0, 243, 255, 0.08)'; }}
                            >
                                <Sparkles size={10} style={{ marginRight: '4px', display: 'inline-block', verticalAlign: 'middle', marginTop: '-2px' }} />
                                {sug}
                            </button>
                        ))}
                    </motion.div>
                )}
            </AnimatePresence>
            <div style={{
                padding: '0.75rem 1rem',
                borderTop: '1px solid var(--border-glass)',
                background: 'rgba(0,0,0,0.3)',
            }}>
                <div style={{ position: 'relative', display: 'flex', gap: '0.5rem' }}>
                    <textarea
                        value={chat.input}
                        onChange={e => chat.setInput(e.target.value)}
                        onKeyDown={e => {
                            if (e.nativeEvent.isComposing) return;
                            if (e.key === 'Enter' && !e.shiftKey) {
                                e.preventDefault();
                                chat.sendMessage();
                            }
                        }}
                        placeholder={t('agent.ready')}
                        rows={1}
                        style={{
                            flex: 1,
                            background: 'rgba(255,255,255,0.04)',
                            border: '1px solid rgba(255,255,255,0.08)',
                            padding: '0.7rem 1rem',
                            borderRadius: '20px',
                            color: 'white',
                            outline: 'none',
                            fontSize: '0.9rem',
                            resize: 'none',
                            transition: 'border-color 0.2s',
                        }}
                        onFocus={e => { 
                            setIsInputFocused(true);
                            (e.target as HTMLTextAreaElement).style.borderColor = 'rgba(0, 242, 255, 0.3)'; 
                        }}
                        onBlur={e => { 
                            // Delay hiding to allow chip click to register
                            setTimeout(() => setIsInputFocused(false), 200);
                            (e.target as HTMLTextAreaElement).style.borderColor = 'rgba(255,255,255,0.08)'; 
                        }}
                    />
                    <button
                        onClick={chat.sendMessage}
                        disabled={!chat.input.trim() || chat.isTyping}
                        style={{
                            width: '40px',
                            height: '40px',
                            borderRadius: '12px',
                            background: chat.input.trim() && !chat.isTyping ? 'var(--accent-cyan)' : 'rgba(255,255,255,0.05)',
                            color: chat.input.trim() && !chat.isTyping ? '#000' : 'rgba(255,255,255,0.15)',
                            border: 'none',
                            cursor: chat.input.trim() && !chat.isTyping ? 'pointer' : 'default',
                            display: 'flex',
                            alignItems: 'center',
                            justifyContent: 'center',
                            transition: 'all 0.2s',
                            flexShrink: 0,
                        }}
                    >
                        <Send size={18} />
                    </button>
                </div>
                <div style={{
                    display: 'flex',
                    justifyContent: 'space-between',
                    marginTop: '0.4rem',
                    padding: '0 0.5rem',
                    fontSize: '0.6rem',
                    color: 'var(--text-muted)',
                    opacity: 0.5,
                }}>
                    <span>
                        <kbd style={{ background: 'rgba(255,255,255,0.1)', padding: '1px 3px', borderRadius: '3px', fontSize: '0.55rem' }}>Shift+Enter</kbd> newline
                    </span>
                    <span className="font-mono" style={{ display: 'flex', alignItems: 'center', gap: '0.3rem' }}>
                        <Sparkles size={9} color="var(--accent-purple)" /> ENHANCED
                    </span>
                </div>
            </div>
        </div>
    );
};

export default StoryFlow;

/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import React from 'react';
import { motion } from 'framer-motion';
import { MessageSquare, Sparkles, Activity, Zap, Brain, BookOpen, ThumbsUp, ThumbsDown, Cpu } from 'lucide-react';

export type FlowCardType = 'chat_user' | 'chat_assistant' | 'chat_streaming' | 'system' | 'karma' | 'knowledge' | 'tool_exec';

export interface FlowCardProps {
    type: FlowCardType;
    title: string;
    content: string;
    timestamp: number;
    // Chat-specific
    isError?: boolean;
    isStreaming?: boolean;
    // Karma-specific
    isOod?: boolean;
    // Feedback
    showFeedback?: boolean;
    onFeedback?: (type: 'positive' | 'negative') => void;
}

const FlowCard: React.FC<FlowCardProps> = ({ type, title, content, timestamp, isError, isStreaming, isOod, showFeedback, onFeedback }) => {
    const getIcon = () => {
        switch (type) {
            case 'chat_user': return <MessageSquare size={14} color="var(--accent-cyan)" />;
            case 'chat_assistant': return <Cpu size={14} color="var(--accent-purple)" />;
            case 'chat_streaming': return <Sparkles size={14} color="var(--accent-cyan)" />;
            case 'karma': return <Brain size={14} color={isOod ? 'var(--accent-rose)' : 'var(--accent-cyan)'} />;
            case 'knowledge': return <BookOpen size={14} color="var(--accent-amber)" />;
            case 'tool_exec': return <Zap size={14} color="var(--accent-amber)" />;
            case 'system':
            default: return <Activity size={14} color="var(--accent-emerald)" />;
        }
    };

    const getBorderColor = () => {
        switch (type) {
            case 'chat_user': return 'rgba(0, 242, 255, 0.3)';
            case 'chat_assistant': return isError ? 'rgba(255, 77, 148, 0.3)' : 'rgba(188, 140, 255, 0.2)';
            case 'chat_streaming': return 'rgba(0, 242, 255, 0.4)';
            case 'karma': return isOod ? 'rgba(255, 82, 82, 0.3)' : 'rgba(0, 243, 255, 0.2)';
            case 'knowledge': return 'rgba(255, 171, 0, 0.3)';
            case 'tool_exec': return 'rgba(245, 158, 11, 0.3)';
            case 'system':
            default: return 'rgba(52, 211, 153, 0.2)';
        }
    };

    const isChat = type === 'chat_user' || type === 'chat_assistant' || type === 'chat_streaming';

    return (
        <motion.div
            initial={{ opacity: 0, y: 8 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0, y: -8 }}
            transition={{ duration: 0.2 }}
            style={{
                display: 'flex',
                flexDirection: 'column',
                alignItems: type === 'chat_user' ? 'flex-end' : 'flex-start',
                width: '100%',
            }}
        >
            {/* Label */}
            <div className="font-mono" style={{
                display: 'flex',
                alignItems: 'center',
                gap: '0.4rem',
                fontSize: '0.65rem',
                fontWeight: 700,
                color: 'var(--text-muted)',
                letterSpacing: '0.12em',
                textTransform: 'uppercase' as const,
                marginBottom: '0.3rem',
                paddingLeft: type === 'chat_user' ? 0 : '0.25rem',
                paddingRight: type === 'chat_user' ? '0.25rem' : 0,
            }}>
                {getIcon()}
                <span>{title}</span>
                <span style={{ opacity: 0.4, fontWeight: 400, marginLeft: '0.3rem' }}>
                    {new Date(timestamp).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit' })}
                </span>
            </div>

            {/* Content Bubble */}
            <div style={{
                maxWidth: isChat ? '85%' : '100%',
                padding: isChat ? '0.9rem 1.2rem' : '0.75rem 1rem',
                borderRadius: type === 'chat_user'
                    ? '16px 16px 4px 16px'
                    : (type === 'chat_assistant' || type === 'chat_streaming')
                        ? '4px 16px 16px 16px'
                        : '10px',
                background: type === 'chat_user'
                    ? 'rgba(0, 242, 255, 0.08)'
                    : (type === 'karma' || type === 'knowledge' || type === 'tool_exec')
                        ? 'rgba(255,255,255,0.015)'
                        : 'rgba(255,255,255,0.025)',
                border: `1px solid ${getBorderColor()}`,
                borderLeftWidth: !isChat ? '3px' : undefined,
                borderLeftColor: !isChat ? getBorderColor() : undefined,
                color: isError ? 'var(--accent-rose)' : 'var(--text-primary)',
                fontSize: isChat ? '0.9rem' : '0.8rem',
                lineHeight: 1.55,
                whiteSpace: 'pre-wrap',
                wordBreak: 'break-word',
            }}>
                {content}
                {isStreaming && (
                    <motion.span
                        animate={{ opacity: [0, 1, 0] }}
                        transition={{ duration: 0.8, repeat: Infinity }}
                        style={{ display: 'inline-block', width: '7px', height: '1.1em', background: 'var(--accent-cyan)', marginLeft: '3px', verticalAlign: 'middle', borderRadius: '1px' }}
                    />
                )}
            </div>

            {/* Feedback Buttons */}
            {showFeedback && onFeedback && (
                <div style={{ display: 'flex', gap: '0.4rem', marginTop: '0.3rem', opacity: 0.5 }}>
                    <button
                        onClick={() => onFeedback('positive')}
                        style={{ background: 'none', border: 'none', cursor: 'pointer', color: 'var(--text-muted)', padding: '2px' }}
                        title="Helpful"
                    >
                        <ThumbsUp size={12} />
                    </button>
                    <button
                        onClick={() => onFeedback('negative')}
                        style={{ background: 'none', border: 'none', cursor: 'pointer', color: 'var(--text-muted)', padding: '2px' }}
                        title="Not Helpful"
                    >
                        <ThumbsDown size={12} />
                    </button>
                </div>
            )}
        </motion.div>
    );
};

export default FlowCard;

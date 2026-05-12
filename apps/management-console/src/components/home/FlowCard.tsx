import React from 'react';
import { motion } from 'framer-motion';
import { MessageSquare, Sparkles, Activity, Zap, Brain, BookOpen, ThumbsUp, ThumbsDown, Cpu } from 'lucide-react';
import { A2uiRenderer } from '../A2uiRenderer';
import ErrorBoundary from '../common/ErrorBoundary';

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
    // Generative UI
    a2uiEnvelope?: any;
}

const FlowCard: React.FC<FlowCardProps> = ({ type, title, content, timestamp, isError, isStreaming, isOod, showFeedback, onFeedback, a2uiEnvelope }) => {
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
            case 'chat_user': return 'var(--accent-cyan-30)';
            case 'chat_assistant': return isError ? 'var(--accent-rose-30)' : 'var(--accent-purple-20)';
            case 'chat_streaming': return 'var(--accent-cyan-40)';
            case 'karma': return isOod ? 'var(--accent-rose-30)' : 'var(--accent-cyan-20)';
            case 'knowledge': return 'var(--accent-amber-30)';
            case 'tool_exec': return 'var(--accent-amber-30)';
            case 'system':
            default: return 'var(--accent-emerald-20)';
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
                maxWidth: (isChat && !a2uiEnvelope) ? '85%' : '100%',
                padding: isChat ? '0.9rem 1.2rem' : '0.75rem 1rem',
                borderRadius: type === 'chat_user'
                    ? '16px 16px 4px 16px'
                    : (type === 'chat_assistant' || type === 'chat_streaming')
                        ? '4px 16px 16px 16px'
                        : '10px',
                background: type === 'chat_user'
                    ? 'var(--accent-cyan-05)'
                    : (type === 'karma' || type === 'knowledge' || type === 'tool_exec')
                        ? 'var(--white-01)'
                        : 'var(--white-02)',
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
                {a2uiEnvelope && (
                    <ErrorBoundary fallback={<div style={{color: 'var(--accent-rose)', fontSize:'0.75rem', marginTop: '0.5rem'}}>A2UI render failed — invalid surface data</div>}>
                        <div style={{ marginTop: content ? '0.75rem' : '0' }}>
                            <A2uiRenderer envelope={a2uiEnvelope} />
                        </div>
                    </ErrorBoundary>
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

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
import { authenticatedFetch } from '../lib/auth';
import { API_BASE } from '../config';
import { SLASH_COMMANDS } from '../constants/slashCommands';
import { useTranslation } from '../i18n';
import { useAgentChat } from '../hooks/AgentChatProvider';
import ReactMarkdown from 'react-markdown';
import rehypeSanitize from 'rehype-sanitize';
import { TokenSavingsIndicator } from './common/TokenSavingsIndicator';
import { ProofPowerIndicator } from './common/ProofPowerIndicator';
import ErrorBoundary from './common/ErrorBoundary';
import { MermaidRenderer } from './MermaidRenderer';
import { A2uiRenderer } from './A2uiRenderer';
import { useWorkspacePersona } from '../hooks/useWorkspacePersona';
import { LockedOverlay } from './ui/LockedOverlay';
import { Activity, Clock, DollarSign, TrendingUp } from 'lucide-react';
import { ActivityFeed } from './common/ActivityFeed';

export interface AgentConsoleProps {
    sessionSavedChars?: number;
}

interface RoiStats {
    tasksExecuted: number;
    savings: number;
    activeBlueprints: number;
    instances: Array<{ id: string, name: string, status: string, nextRun: string, roi: string }>;
}

interface AuditLedgerEntry {
    record_id?: string;
    table_name?: string;
    operation?: string;
    timestamp?: string;
}

const SAVINGS_PER_TASK = 5;

interface ArtifactRecord {
    id: string;
    category?: string;
    name?: string;
}

const ICON_MAP: Record<string, React.ReactNode> = {
    Volume2: <Volume2 size={14} />, Sparkles: <Sparkles size={14} />,
    Brain: <Brain size={14} />, Cpu: <Cpu size={14} />,
};

const AgentConsole: React.FC<AgentConsoleProps> = ({ sessionSavedChars = 0 }) => {
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

    const { mode } = useWorkspacePersona();
    const [activeTab, setActiveTab] = React.useState<'chat' | 'automations'>('chat');

    const chatEndRef = useRef<HTMLDivElement>(null);
    const [stats, setStats] = React.useState<RoiStats | null>(null);

    const [slashIndex, setSlashIndex] = React.useState(0);
    const COMMANDS = SLASH_COMMANDS.map(c => ({
        ...c,
        icon: ICON_MAP[c.iconName] || <Cpu size={14} />,
    }));
    const trimmedInput = input.trimStart().replace('／', '/');
    const showSlash = trimmedInput.startsWith('/');
    const filteredCmds = showSlash ? COMMANDS.filter(c => c.cmd.startsWith(trimmedInput.toLowerCase())) : [];

    const scrollToBottom = () => {
        chatEndRef.current?.scrollIntoView({ behavior: "smooth" });
    };

    useEffect(() => {
        if (activeTab === 'automations' && mode === 'agency' && !stats) {
            const fetchStats = async () => {
                try {
                    // Fetch real metrics from the API
                    const [artifactsRes, ledgerRes] = await Promise.all([
                        authenticatedFetch(`${API_BASE}/api/artifacts`),
                        authenticatedFetch(`${API_BASE}/api/v1/audit/ledger`)
                    ]);
                    
                    const artifactsRaw = artifactsRes.ok ? await artifactsRes.json() : [];
                    const ledgerRaw = ledgerRes.ok ? await ledgerRes.json() : [];
                    
                    // Structural validation: ensure API responses are arrays (same pattern as ArtifactVault)
                    const artifacts = Array.isArray(artifactsRaw) ? artifactsRaw : [];
                    const ledger = Array.isArray(ledgerRaw) ? ledgerRaw : [];
                    
                    const blueprints = (artifacts as ArtifactRecord[]).filter(a => a.category === 'Blueprint' || a.category === 'blueprint');
                    const tasksCount = ledger.length || 0;
                    
                    setStats({
                        tasksExecuted: tasksCount,
                        savings: tasksCount * SAVINGS_PER_TASK, // Use constant saving per automated task
                        activeBlueprints: blueprints.length,
                        instances: blueprints.map(bp => {
                            const bpTasksCount = ledger.filter((l: AuditLedgerEntry) => l.record_id === bp.id).length;
                            const bpRoi = (bpTasksCount * SAVINGS_PER_TASK) + 100;
                            return {
                                id: bp.id,
                                name: bp.name || 'Untitled Blueprint',
                                status: 'Running',
                                nextRun: 'Active',
                                roi: '+$' + bpRoi + '/mo'
                            };
                        })
                    });
                } catch (e) {
                    console.error("Failed to fetch ROI stats:", e);
                    // Fallback to safe empty stats to prevent loader freeze and infinite retries
                    setStats({
                        tasksExecuted: 0,
                        savings: 0,
                        activeBlueprints: 0,
                        instances: []
                    });
                }
            };
            fetchStats();
        }
    }, [activeTab, mode, stats]);

    useEffect(scrollToBottom, [history, streamingText]);

    const karmaNotFound = relevantKarma
        ? relevantKarma.includes('見つかりませんでした') || relevantKarma.includes('not found')
        : false;

    // Shared ReactMarkdown component overrides (DRY: used in both history and streaming renders)
    const markdownComponents = {
        h1: ({node, ...props}: React.ComponentPropsWithoutRef<'h1'> & { node?: unknown }) => <h1 className="agent-console-md-h1" {...props} />,
        h2: ({node, ...props}: React.ComponentPropsWithoutRef<'h2'> & { node?: unknown }) => <h2 className="agent-console-md-h2" {...props} />,
        h3: ({node, ...props}: React.ComponentPropsWithoutRef<'h3'> & { node?: unknown }) => <h3 className="agent-console-md-h3" {...props} />,
        a: ({node, ...props}: React.ComponentPropsWithoutRef<'a'> & { node?: unknown }) => <a className="agent-console-md-a" {...props} />,
        code: ({node, className, children, ...props}: React.ComponentPropsWithoutRef<'code'> & { node?: unknown; children?: React.ReactNode }) => {
            const match = /language-(\w+)/.exec(className || '');
            if (match && match[1] === 'mermaid') {
                return (
                    <ErrorBoundary fallback={<div className="agent-console-md-error">Failed to render mermaid diagram (React error)</div>}>
                        <MermaidRenderer code={String(children).replace(/\n$/, '')} />
                    </ErrorBoundary>
                );
            }
            return <code className={`agent-console-md-code ${className || ''}`} {...props}>{children}</code>;
        },
        pre: ({node, ...props}: React.ComponentPropsWithoutRef<'pre'> & { node?: unknown }) => <pre className="agent-console-md-pre" {...props} />
    };

    return (
        <div className="main-panel ani-fade agent-console-root">
            <ActivityFeed maxItems={5} />
            {/* Header */}
            <div className="panel-header agent-console-header">
                <div className="agent-console-header__identity">
                    <div className="agent-console-header__avatar-wrap">
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
                        <h3 className="agent-console-header__title">{t('agent.title')}</h3>
                        <div className={`agent-console-header__status${isTyping ? ' agent-console-header__status--typing' : ''}`}>
                            <span className={`agent-console-header__status-dot${isTyping ? ' agent-console-header__status-dot--typing' : ''}`} />
                            {status}
                        </div>
                    </div>
                </div>

                {mode === 'agency' && (
                    <div className="agent-console-header__tabs">
                        <button
                            onClick={() => setActiveTab('chat')}
                            className={`agent-console-tab-btn${activeTab === 'chat' ? ' agent-console-tab-btn--active' : ''}`}
                        >
                            {t('agent.copilotChat') || 'Copilot Chat'}
                        </button>
                        <button
                            onClick={() => setActiveTab('automations')}
                            className={`agent-console-tab-btn${activeTab === 'automations' ? ' agent-console-tab-btn--active' : ''}`}
                        >
                            {t('agent.automationsTab') || 'Automations (ROI)'}
                        </button>
                    </div>
                )}

                <div className="agent-console-header__actions">
                    <LockedOverlay featureNameKey="pro.featureTts" variant="badge">
                    <button 
                        onClick={() => setAutoTts(!autoTts)}
                        className={`stat-badge agent-console-tts-btn${autoTts ? ' agent-console-tts-btn--on' : ''}`}
                    >
                        {autoTts ? <Volume2 size={12} /> : <VolumeX size={12} />}
                        {t('agent.voice')}: {autoTts ? 'ON' : 'OFF'}
                    </button>
                    </LockedOverlay>
                    <ProofPowerIndicator variant="compact" />
                    <TokenSavingsIndicator savedChars={sessionSavedChars} variant="compact" />
                    <div className="stat-badge agent-console-model-badge">{t('agent.modelBadge') || '3.5B MODEL'}</div>
                </div>
            </div>

            {activeTab === 'automations' && mode === 'agency' ? (
                <div className="agent-console-scroll-panel">
                    <h2 className="agent-console-page-title">
                        <Activity size={24} color="var(--accent-cyan)" />
                        {t('agent.scheduledAutomations') || 'Scheduled Automations'}
                    </h2>

                    <div className="agent-console-stat-grid">
                        <div className="glass-panel agent-console-stat-card">
                            <div className="agent-console-stat-label"><Clock size={16} /> {t('agent.tasksExecuted') || 'Tasks Executed'}</div>
                            <div className="agent-console-stat-value agent-console-stat-value--cyan">{stats ? stats.tasksExecuted.toLocaleString() : '...'}</div>
                            <div className="agent-console-stat-caption">{t('agent.lifetimeMetrics') || 'Lifetime metrics'}</div>
                        </div>
                        <div className="glass-panel agent-console-stat-card">
                            <div className="agent-console-stat-label"><DollarSign size={16} /> {t('agent.estimatedSavings') || 'Estimated Savings'} (推定値)</div>
                            <div className="agent-console-stat-value agent-console-stat-value--emerald">${stats ? stats.savings.toLocaleString() : '...'}</div>
                            <div className="agent-console-stat-caption agent-console-stat-caption--emerald">{t('agent.basedOnVolume') || 'Based on task volume'}</div>
                        </div>
                        <div className="glass-panel agent-console-stat-card">
                            <div className="agent-console-stat-label"><TrendingUp size={16} /> {t('agent.activeBlueprints') || 'Active Blueprints'}</div>
                            <div className="agent-console-stat-value agent-console-stat-value--purple">{stats ? stats.activeBlueprints : '...'}</div>
                            <div className="agent-console-stat-caption">{t('agent.runningFlawlessly') || 'Running flawlessly'}</div>
                        </div>
                    </div>

                    <div className="glass-panel agent-console-instances-panel">
                        <h3 className="agent-console-instances-title">{t('agent.blueprintInstances') || 'Active Blueprint Instances'}</h3>
                        <div className="agent-console-instances-list">
                            {!stats ? (
                                <div className="agent-console-empty-state">{t('common.loading') || 'Loading instances...'}</div>
                            ) : stats.instances.length === 0 ? (
                                <div className="agent-console-empty-state">{t('agent.noBlueprintInstances') || 'No active blueprint instances found.'}</div>
                            ) : (
                                stats.instances.map(bp => (
                                    <div key={bp.id} className="agent-console-instance-row">
                                        <div className="agent-console-instance-info">
                                            <div className="agent-console-instance-name">{bp.name}</div>
                                            <div className="agent-console-instance-meta">ID: {bp.id} • Next: {bp.nextRun}</div>
                                        </div>
                                        <div className="agent-console-instance-actions">
                                            <div className="agent-console-instance-roi">
                                                 <span className="agent-console-instance-roi-label">
                                                     {t('agent.estimatedRoiLabel') || 'Estimated:'}
                                                 </span>
                                                 {bp.roi}
                                             </div>
                                            <div className={`agent-console-status-badge ${bp.status.includes('Running') ? 'agent-console-status-badge--running' : 'agent-console-status-badge--stopped'}`}>
                                                {bp.status}
                                            </div>
                                        </div>
                                    </div>
                                ))
                            )}
                        </div>
                    </div>
                </div>
            ) : (
                <>
            {/* Chat Area */}
            <div className="agent-console-chat">
                {history.length === 0 && !streamingText && (
                    <div className="agent-console-welcome">
                        <Cpu size={48} className="agent-console-welcome__icon" />
                        <h4 className="agent-console-welcome__title">{t('agent.ready') || 'How can I help you today?'}</h4>
                        <p className="agent-console-welcome__desc">{t('agent.issueCommands') || 'Choose a template below or type your own command.'}</p>
                        
                        <div className="agent-console-template-grid">
                            {[
                                { icon: <Sparkles size={20} color="var(--accent-cyan)" />, title: t('agent.template.writeCode') || 'Write code', desc: t('agent.template.writeCodeDesc') || 'Create a new React component', prompt: 'Create a new React component with Tailwind CSS.' },
                                { icon: <Brain size={20} color="var(--accent-purple)" />, title: t('agent.template.analyzeData') || 'Analyze data', desc: t('agent.template.analyzeDataDesc') || 'Find trends in recent logs', prompt: 'Analyze the system logs from the past 24 hours and identify any anomalies.' },
                                { icon: <Activity size={20} color="var(--accent-emerald)" />, title: t('agent.template.automate') || 'Automate', desc: t('agent.template.automateDesc') || 'Schedule a daily backup task', prompt: 'Create an automation blueprint that runs a daily database backup.' },
                                { icon: <BookOpen size={20} color="var(--accent-amber)" />, title: t('agent.template.learn') || 'Learn', desc: t('agent.template.learnDesc') || 'Explain how the Hub works', prompt: 'Explain the architecture of the Sync Hub.' }
                            ].map((card, idx) => (
                                <button
                                    key={idx}
                                    onClick={() => {
                                        setInput(card.prompt);
                                    }}
                                    className="agent-console-template-card"
                                >
                                    {card.icon}
                                    <div className="agent-console-template-card__title">{card.title}</div>
                                    <div className="agent-console-template-card__desc">{card.desc}</div>
                                </button>
                            ))}
                        </div>
                    </div>
                )}

                {relevantKarma && (
                    <motion.div
                        initial={{ opacity: 0, y: -10 }}
                        animate={{ opacity: 1, y: 0 }}
                        className={`glass-panel agent-console-karma-panel ${karmaNotFound ? 'agent-console-karma-panel--missing' : 'agent-console-karma-panel--found'}`}
                    >
                        <div className="agent-console-karma-header">
                            <Brain size={14} color={karmaNotFound ? 'var(--accent-rose)' : 'var(--accent-cyan)'} />
                            {karmaNotFound ? t('agent.outOfDomain') : t('agent.synapticMemory')}
                        </div>
                        <div className="agent-console-karma-body">
                            {relevantKarma}
                        </div>
                    </motion.div>
                )}

                {activeKnowledge && (
                    <motion.div
                        initial={{ opacity: 0, y: -10 }}
                        animate={{ opacity: 1, y: 0 }}
                        className="agent-console-knowledge-panel"
                    >
                        <div className="agent-console-knowledge-header">
                            <BookOpen size={14} />
                            {t('agent.knowledgeAccessed')}
                        </div>
                        <div className="agent-console-knowledge-body">
                            {activeKnowledge}
                        </div>
                    </motion.div>
                )}

                {history.map((m, i) => (
                    <motion.div
                        key={i}
                        initial={{ opacity: 0, x: m.role === 'user' ? 20 : -20 }}
                        animate={{ opacity: 1, x: 0 }}
                        className={`agent-console-msg-wrap agent-console-msg-wrap--${m.role === 'user' ? 'user' : 'assistant'}`}
                    >
                        <div className="agent-console-msg-role">
                            {m.role === 'user' ? t('agent.roleUser') : t('agent.roleAiome')}
                        </div>
                        <div className={`agent-console-msg-bubble agent-console-msg-bubble--${m.role === 'user' ? 'user' : 'assistant'}${m.isError ? ' agent-console-msg-bubble--error' : ''}`}>
                            {m.content && (
                                <ReactMarkdown
                                    rehypePlugins={[rehypeSanitize]}
                                    components={markdownComponents}
                                >
                                    {m.content}
                                </ReactMarkdown>
                            )}
                            {m.reasoning && (
                                <details className="agent-console-reasoning">
                                    <summary className="agent-console-reasoning__summary">{t('agent.thinkingProcess') || '🧠 Thinking Process'}</summary>
                                    <div className="agent-console-reasoning__body">
                                        {m.reasoning}
                                    </div>
                                </details>
                            )}
                            {m.a2uiEnvelope && (
                                <ErrorBoundary fallback={<div className="agent-console-a2ui-error">{t('error.a2uiFailed') || 'A2UI render failed — invalid surface data'}</div>}>
                                    <A2uiRenderer envelope={m.a2uiEnvelope} />
                                </ErrorBoundary>
                            )}
                        </div>

                        {m.role === 'assistant' && !m.isError && i === history.length - 1 && (relevantKarmaData?.entries?.length ?? 0) > 0 && (
                            <div className="agent-console-feedback">
                                <button
                                    onClick={() => handleFeedback(i, 'positive')}
                                    className="agent-console-feedback-btn"
                                    title={t('agent.helpfulLesson')}
                                >
                                    <ThumbsUp size={14} />
                                </button>
                                <button
                                    onClick={() => handleFeedback(i, 'negative')}
                                    className="agent-console-feedback-btn"
                                    title={t('agent.notHelpfulLesson')}
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
                        className="agent-console-stream-wrap"
                    >
                        <div className="agent-console-stream-label">AIOME ({t('agent.streaming')})</div>
                        <div className="agent-console-stream-bubble">
                            <ReactMarkdown
                                rehypePlugins={[rehypeSanitize]}
                                components={markdownComponents}
                            >
                                {streamingText}
                            </ReactMarkdown>
                            <motion.span
                                animate={{ opacity: [0, 1, 0] }}
                                transition={{ duration: 0.8, repeat: Infinity }}
                                className="agent-console-stream-cursor"
                            />
                        </div>
                    </motion.div>
                )}

                <div ref={chatEndRef} />
            </div>

            {/* Slash Command Suggestions */}
            {showSlash && filteredCmds.length > 0 && (
                <div className="agent-console-slash-wrap">
                    <div className="agent-console-slash-panel">
                        {filteredCmds.map((cmd, i) => (
                            <div key={cmd.cmd}
                                onClick={() => {
                                    setInput(cmd.cmd);
                                    setTimeout(() => sendMessage(cmd.cmd), 0);
                                }}
                                className={`agent-console-slash-item${i === slashIndex ? ' agent-console-slash-item--active' : ''}`}
                                onMouseEnter={() => setSlashIndex(i)}
                            >
                                <div className={`agent-console-slash-icon${i === slashIndex ? ' agent-console-slash-icon--active' : ''}`}>{cmd.icon}</div>
                                <div>
                                    <div className="agent-console-slash-label">{cmd.label} <span className="agent-console-slash-cmd">{cmd.cmd}</span></div>
                                    <div className="agent-console-slash-desc">{cmd.desc}</div>
                                </div>
                            </div>
                        ))}
                    </div>
                </div>
            )}

            {/* Input Area */}
            <div className="agent-console-input-area">
                <div className="agent-console-input-wrap">
                    <textarea
                        value={input}
                        onChange={e => {
                            setInput(e.target.value);
                            setSlashIndex(0);
                        }}
                        onKeyDown={e => {
                            if (e.nativeEvent.isComposing) return;
                            if (showSlash && filteredCmds.length > 0) {
                                if (e.key === 'ArrowDown') {
                                    e.preventDefault();
                                    setSlashIndex((prev) => (prev + 1) % filteredCmds.length);
                                    return;
                                }
                                if (e.key === 'ArrowUp') {
                                    e.preventDefault();
                                    setSlashIndex((prev) => (prev - 1 + filteredCmds.length) % filteredCmds.length);
                                    return;
                                }
                                if (e.key === 'Enter') {
                                    e.preventDefault();
                                    const selectedCmd = filteredCmds[slashIndex].cmd;
                                    setInput(selectedCmd);
                                    setTimeout(() => sendMessage(selectedCmd), 0);
                                    return;
                                }
                            }
                            if (e.key === 'Enter' && !e.shiftKey) {
                                e.preventDefault();
                                sendMessage();
                            }
                        }}
                        placeholder={t('agent.chatPlaceholder') || "Ask agent or type '/' for commands..."}
                        rows={1}
                        className="agent-console-textarea"
                    />
                    <button
                        onClick={() => sendMessage()}
                        disabled={!input.trim() || isTyping}
                        className={`agent-console-send-btn ${input.trim() && !isTyping ? 'agent-console-send-btn--enabled' : 'agent-console-send-btn--disabled'}`}
                    >
                        <Send size={20} />
                    </button>
                </div>
                <div className="agent-console-input-footer">
                    <div className="agent-console-input-hints">
                        <div className="agent-console-kbd-hint">
                            <kbd className="agent-console-kbd">Shift+Enter</kbd> {t('agent.toNewline')}
                        </div>
                    </div>
                    <div className="agent-console-enhance-hint">
                        <Sparkles size={12} color="var(--accent-purple)" /> {t('agent.promptEnhancement')}
                    </div>
                </div>
            </div>
            </>
            )}
        </div>
    );
};

export default AgentConsole;

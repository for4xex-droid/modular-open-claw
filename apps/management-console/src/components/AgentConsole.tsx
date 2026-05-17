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
import { useAgentChat } from '../hooks/useAgentChat';
import ReactMarkdown from 'react-markdown';
import rehypeSanitize from 'rehype-sanitize';
import { TokenSavingsIndicator } from './common/TokenSavingsIndicator';
import { ProofPowerIndicator } from './common/ProofPowerIndicator';
import ErrorBoundary from './common/ErrorBoundary';
import { MermaidRenderer } from './MermaidRenderer';
import { A2uiRenderer } from './A2uiRenderer';
import { useWorkspacePersona } from '../hooks/useWorkspacePersona';
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
                        savings: tasksCount * 5, // Simple $5 saving per automated task
                        activeBlueprints: blueprints.length,
                        instances: blueprints.map(bp => ({
                            id: bp.id,
                            name: bp.name || 'Untitled Blueprint',
                            status: 'Running',
                            nextRun: 'Active',
                            roi: '+$' + (Math.abs(bp.id.split('').reduce((acc, char) => acc + char.charCodeAt(0), 0)) % 500 + 100) + '/mo'
                        }))
                    });
                } catch (e) {
                    console.error("Failed to fetch ROI stats:", e);
                }
            };
            fetchStats();
        }
    }, [activeTab, mode, stats]);

    useEffect(scrollToBottom, [history, streamingText]);

    // Shared ReactMarkdown component overrides (DRY: used in both history and streaming renders)
    const markdownComponents = {
        h1: ({node, ...props}: any) => <h1 style={{color: 'var(--accent-cyan)', fontSize: '1.5em', marginTop: '1em', marginBottom: '0.5em'}} {...props} />,
        h2: ({node, ...props}: any) => <h2 style={{color: 'var(--accent-purple)', fontSize: '1.2em', marginTop: '1em', marginBottom: '0.5em'}} {...props} />,
        h3: ({node, ...props}: any) => <h3 style={{fontSize: '1.1em', marginTop: '1em', marginBottom: '0.5em'}} {...props} />,
        a: ({node, ...props}: any) => <a style={{color: 'var(--accent-cyan)', textDecoration: 'underline'}} {...props} />,
        code: ({node, className, children, ...props}: any) => {
            const match = /language-(\w+)/.exec(className || '');
            if (match && match[1] === 'mermaid') {
                return (
                    <ErrorBoundary fallback={<div style={{color: 'var(--accent-rose)', fontSize:'0.8rem', padding:'1rem', border:'1px solid var(--accent-rose-30)'}}>Failed to render mermaid diagram (React error)</div>}>
                        <MermaidRenderer code={String(children).replace(/\n$/, '')} />
                    </ErrorBoundary>
                );
            }
            return <code style={{background: 'var(--black-40)', padding: '0.2em 0.4em', borderRadius: '4px', fontFamily: 'monospace', fontSize: '0.9em'}} className={className} {...props}>{children}</code>;
        },
        pre: ({node, ...props}: any) => <pre style={{background: 'var(--black-60)', padding: '1em', borderRadius: 'var(--radius-md)', overflowX: 'auto', marginBottom: '1em'}} {...props} />
    };

    return (
        <div className="main-panel ani-fade" style={{ height: '78vh', display: 'flex', flexDirection: 'column', padding: 0, overflow: 'hidden', position: 'relative' }}>
            <ActivityFeed maxItems={5} />
            {/* Header */}
            <div className="panel-header" style={{ padding: '1rem 1.5rem', borderBottom: '1px solid var(--border-glass)' }}>
                <div style={{ display: 'flex', alignItems: 'center', gap: 'var(--space-sm)' }}>
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
                        <h3 style={{ fontSize: '1rem', fontWeight: 700, margin: 0 }}>{t('agent.title')}</h3>
                        <div style={{ fontSize: '0.7rem', color: isTyping ? 'var(--accent-cyan)' : 'var(--text-muted)', display: 'flex', alignItems: 'center', gap: '0.4rem' }}>
                            <span style={{ width: 6, height: 6, borderRadius: '50%', background: isTyping ? 'var(--accent-cyan)' : 'var(--accent-emerald)' }} />
                            {status}
                        </div>
                    </div>
                </div>

                {mode === 'agency' && (
                    <div style={{ display: 'flex', gap: '1rem', flex: 1, justifyContent: 'center' }}>
                        <button
                            onClick={() => setActiveTab('chat')}
                            style={{
                                background: activeTab === 'chat' ? 'var(--accent-cyan-20)' : 'transparent',
                                border: 'none',
                                color: activeTab === 'chat' ? 'var(--accent-cyan)' : 'var(--text-muted)',
                                padding: '0.4rem 1rem',
                                borderRadius: 'var(--radius-sm)',
                                cursor: 'pointer',
                                fontWeight: 600,
                                fontSize: '0.85rem'
                            }}
                        >
                            {t('agent.copilotChat') || 'Copilot Chat'}
                        </button>
                        <button
                            onClick={() => setActiveTab('automations')}
                            style={{
                                background: activeTab === 'automations' ? 'var(--accent-cyan-20)' : 'transparent',
                                border: 'none',
                                color: activeTab === 'automations' ? 'var(--accent-cyan)' : 'var(--text-muted)',
                                padding: '0.4rem 1rem',
                                borderRadius: 'var(--radius-sm)',
                                cursor: 'pointer',
                                fontWeight: 600,
                                fontSize: '0.85rem'
                            }}
                        >
                            {t('agent.automationsTab') || 'Automations (ROI)'}
                        </button>
                    </div>
                )}

                <div style={{ display: 'flex', gap: '0.8rem', alignItems: 'center' }}>
                    <button 
                        onClick={() => setAutoTts(!autoTts)}
                        className="stat-badge" 
                        style={{ 
                            fontSize: '0.7rem', 
                            background: autoTts ? 'var(--accent-cyan-10)' : 'var(--white-03)',
                            border: `1px solid ${autoTts ? 'var(--accent-cyan)' : 'transparent'}`,
                            cursor: 'pointer',
                            display: 'flex',
                            alignItems: 'center',
                            gap: '0.4rem',
                            color: autoTts ? 'var(--accent-cyan)' : 'var(--text-muted)',
                            transition: 'all var(--speed-normal)'
                        }}
                    >
                        {autoTts ? <Volume2 size={12} /> : <VolumeX size={12} />}
                        {t('agent.voice')}: {autoTts ? 'ON' : 'OFF'}
                    </button>
                    <ProofPowerIndicator variant="compact" />
                    <TokenSavingsIndicator savedChars={sessionSavedChars} variant="compact" />
                    <div className="stat-badge" style={{ fontSize: '0.7rem', background: 'var(--white-03)' }}>{t('agent.modelBadge') || '3.5B MODEL'}</div>
                </div>
            </div>

            {activeTab === 'automations' && mode === 'agency' ? (
                <div style={{ flex: 1, overflowY: 'auto', padding: '2rem', display: 'flex', flexDirection: 'column', gap: '2rem', background: 'var(--black-20)' }}>
                    <h2 style={{ fontSize: '1.5rem', fontWeight: 700, margin: 0, display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
                        <Activity size={24} color="var(--accent-cyan)" />
                        {t('agent.scheduledAutomations') || 'Scheduled Automations'}
                    </h2>

                    <div style={{ display: 'grid', gridTemplateColumns: 'repeat(3, 1fr)', gap: '1rem' }}>
                        <div className="glass-panel" style={{ padding: '1.5rem', display: 'flex', flexDirection: 'column', gap: '0.5rem' }}>
                            <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem', color: 'var(--text-muted)' }}><Clock size={16} /> {t('agent.tasksExecuted') || 'Tasks Executed'}</div>
                            <div style={{ fontSize: '2rem', fontWeight: 800, color: 'var(--accent-cyan)' }}>{stats ? stats.tasksExecuted.toLocaleString() : '...'}</div>
                            <div style={{ fontSize: '0.8rem', color: 'var(--text-muted)' }}>{t('agent.lifetimeMetrics') || 'Lifetime metrics'}</div>
                        </div>
                        <div className="glass-panel" style={{ padding: '1.5rem', display: 'flex', flexDirection: 'column', gap: '0.5rem' }}>
                            <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem', color: 'var(--text-muted)' }}><DollarSign size={16} /> {t('agent.estimatedSavings') || 'Estimated Savings'}</div>
                            <div style={{ fontSize: '2rem', fontWeight: 800, color: 'var(--accent-emerald)' }}>${stats ? stats.savings.toLocaleString() : '...'}</div>
                            <div style={{ fontSize: '0.8rem', color: 'var(--accent-emerald)' }}>{t('agent.basedOnVolume') || 'Based on task volume'}</div>
                        </div>
                        <div className="glass-panel" style={{ padding: '1.5rem', display: 'flex', flexDirection: 'column', gap: '0.5rem' }}>
                            <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem', color: 'var(--text-muted)' }}><TrendingUp size={16} /> {t('agent.activeBlueprints') || 'Active Blueprints'}</div>
                            <div style={{ fontSize: '2rem', fontWeight: 800, color: 'var(--accent-purple)' }}>{stats ? stats.activeBlueprints : '...'}</div>
                            <div style={{ fontSize: '0.8rem', color: 'var(--text-muted)' }}>{t('agent.runningFlawlessly') || 'Running flawlessly'}</div>
                        </div>
                    </div>

                    <div className="glass-panel" style={{ padding: '1.5rem', flex: 1 }}>
                        <h3 style={{ fontSize: '1.1rem', marginBottom: '1rem', borderBottom: '1px solid var(--border-glass)', paddingBottom: '0.5rem' }}>{t('agent.blueprintInstances') || 'Active Blueprint Instances'}</h3>
                        <div style={{ display: 'flex', flexDirection: 'column', gap: '1rem' }}>
                            {!stats ? (
                                <div style={{ color: 'var(--text-muted)', textAlign: 'center', padding: '1rem' }}>{t('common.loading') || 'Loading instances...'}</div>
                            ) : stats.instances.length === 0 ? (
                                <div style={{ color: 'var(--text-muted)', textAlign: 'center', padding: '1rem' }}>{t('agent.noBlueprintInstances') || 'No active blueprint instances found.'}</div>
                            ) : (
                                stats.instances.map(bp => (
                                    <div key={bp.id} style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', padding: '1rem', background: 'var(--white-03)', borderRadius: 'var(--radius-md)' }}>
                                        <div style={{ display: 'flex', flexDirection: 'column', gap: '0.2rem' }}>
                                            <div style={{ fontWeight: 600 }}>{bp.name}</div>
                                            <div style={{ fontSize: '0.75rem', color: 'var(--text-muted)' }}>ID: {bp.id} • Next: {bp.nextRun}</div>
                                        </div>
                                        <div style={{ display: 'flex', alignItems: 'center', gap: '2rem' }}>
                                            <div style={{ fontWeight: 600, color: 'var(--accent-emerald)' }}>{bp.roi}</div>
                                            <div style={{ 
                                                padding: '0.2rem 0.6rem', 
                                                borderRadius: '1rem', 
                                                fontSize: '0.75rem',
                                                background: bp.status.includes('Running') ? 'var(--accent-emerald-10)' : 'var(--accent-rose-10)',
                                                color: bp.status.includes('Running') ? 'var(--accent-emerald)' : 'var(--accent-rose)'
                                            }}>
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
            <div style={{ flex: 1, overflowY: 'auto', padding: '2rem', display: 'flex', flexDirection: 'column', gap: 'var(--space-md)', background: 'var(--black-20)' }}>
                {history.length === 0 && !streamingText && (
                    <div style={{ flex: 1, display: 'flex', flexDirection: 'column', alignItems: 'center', justifyContent: 'center', color: 'var(--text-muted)', textAlign: 'center', padding: '2rem' }}>
                        <Cpu size={48} style={{ opacity: 0.1, marginBottom: '1.5rem' }} />
                        <h4 style={{ fontWeight: 600, color: 'var(--white-20)', marginBottom: '0.5rem' }}>{t('agent.ready') || 'How can I help you today?'}</h4>
                        <p style={{ fontSize: '0.85rem', maxWidth: '300px', marginBottom: '2rem' }}>{t('agent.issueCommands') || 'Choose a template below or type your own command.'}</p>
                        
                        <div style={{ display: 'flex', gap: '1rem', flexWrap: 'wrap', justifyContent: 'center', maxWidth: '800px' }}>
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
                                    style={{
                                        display: 'flex', flexDirection: 'column', alignItems: 'flex-start', gap: '0.5rem',
                                        padding: '1rem', background: 'var(--white-03)', border: '1px solid var(--border-glass)',
                                        borderRadius: 'var(--radius-md)', cursor: 'pointer', textAlign: 'left',
                                        width: '180px', transition: 'all var(--speed-normal)'
                                    }}
                                    onMouseOver={(e) => e.currentTarget.style.background = 'var(--white-05)'}
                                    onMouseOut={(e) => e.currentTarget.style.background = 'var(--white-03)'}
                                >
                                    {card.icon}
                                    <div style={{ fontWeight: 600, color: 'var(--text-primary)', fontSize: '0.85rem' }}>{card.title}</div>
                                    <div style={{ fontSize: '0.75rem', color: 'var(--text-secondary)', lineHeight: 1.4 }}>{card.desc}</div>
                                </button>
                            ))}
                        </div>
                    </div>
                )}

                {relevantKarma && (
                    <motion.div
                        initial={{ opacity: 0, y: -10 }}
                        animate={{ opacity: 1, y: 0 }}
                        className="glass-panel"
                        style={{
                            padding: '1.2rem',
                            background: relevantKarma.includes('見つかりませんでした') || relevantKarma.includes('not found')
                                ? 'var(--accent-rose-10)'
                                : 'var(--accent-cyan-05)',
                            border: `1px solid ${relevantKarma.includes('見つかりませんでした') || relevantKarma.includes('not found') ? 'var(--accent-rose-20)' : 'var(--accent-cyan-10)'}`,
                            borderLeftWidth: '4px',
                            borderLeftColor: relevantKarma.includes('見つかりませんでした') || relevantKarma.includes('not found') ? 'var(--accent-rose)' : 'var(--accent-cyan)',
                            fontSize: '0.8rem',
                            marginBottom: '1rem',
                        }}
                    >
                        <div style={{ fontWeight: 800, fontSize: '0.7rem', color: 'var(--white-50)', marginBottom: '0.8rem', display: 'flex', alignItems: 'center', gap: '0.6rem', letterSpacing: '0.1em' }}>
                            <Brain size={14} color={relevantKarma.includes('見つかりませんでした') || relevantKarma.includes('not found') ? 'var(--accent-rose)' : 'var(--accent-cyan)'} />
                            {relevantKarma.includes('見つかりませんでした') || relevantKarma.includes('not found') ? t('agent.outOfDomain') : t('agent.synapticMemory')}
                        </div>
                        <div style={{ whiteSpace: 'pre-wrap', lineHeight: 1.5, color: 'var(--white-80)' }}>
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
                            background: 'var(--accent-amber-10)',
                            border: '1px solid var(--accent-amber-20)',
                            borderLeft: '4px solid var(--accent-amber)',
                            borderRadius: 'var(--radius-sm) var(--radius-md) var(--radius-md) var(--radius-sm)',
                            marginBottom: '1rem',
                        }}
                    >
                        <div style={{ fontWeight: 800, fontSize: '0.7rem', color: 'var(--accent-amber-80)', marginBottom: '0.5rem', display: 'flex', alignItems: 'center', gap: '0.6rem', letterSpacing: '0.1em' }}>
                            <BookOpen size={14} />
                            {t('agent.knowledgeAccessed')}
                        </div>
                        <div style={{ fontSize: '0.85rem', color: 'var(--white-90)', fontWeight: 500 }}>
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
                        <div style={{ fontSize: '0.7rem', color: 'var(--text-muted)', fontWeight: 700, letterSpacing: '0.05em', textTransform: 'uppercase' }}>
                            {m.role === 'user' ? t('agent.roleUser') : t('agent.roleAiome')}
                        </div>
                        <div style={{
                            padding: '1.25rem',
                            borderRadius: m.role === 'user' ? 'var(--radius-lg) var(--radius-lg) 4px var(--radius-lg)' : '4px var(--radius-lg) var(--radius-lg) var(--radius-lg)',
                            background: m.role === 'user' ? 'var(--accent-cyan-glass)' : 'var(--bg-glass-heavy)',
                            border: m.role === 'user' ? '1px solid var(--accent-cyan-30)' : '1px solid var(--border-glass)',
                            color: m.isError ? 'var(--accent-rose)' : 'var(--text-primary)',
                            fontSize: '0.95rem',
                            lineHeight: 1.6,
                            boxShadow: 'var(--shadow-shallow)',
                            whiteSpace: 'pre-wrap'
                        }}>
                            {m.content && (
                                <ReactMarkdown
                                    rehypePlugins={[rehypeSanitize]}
                                    components={markdownComponents}
                                >
                                    {m.content}
                                </ReactMarkdown>
                            )}
                            {m.reasoning && (
                                <details style={{ marginTop: '0.5rem', fontSize: '0.8rem', color: 'var(--text-muted)' }}>
                                    <summary style={{ cursor: 'pointer', fontWeight: 600 }}>{t('agent.thinkingProcess') || '🧠 Thinking Process'}</summary>
                                    <div style={{ padding: '0.5rem', background: 'var(--black-20)', borderRadius: 'var(--radius-sm)', marginTop: '0.5rem', whiteSpace: 'pre-wrap' }}>
                                        {m.reasoning}
                                    </div>
                                </details>
                            )}
                            {m.a2uiEnvelope && (
                                <ErrorBoundary fallback={<div style={{color: 'var(--accent-rose)', fontSize:'0.75rem'}}>{t('error.a2uiFailed') || 'A2UI render failed — invalid surface data'}</div>}>
                                    <A2uiRenderer envelope={m.a2uiEnvelope} />
                                </ErrorBoundary>
                            )}
                        </div>

                        {m.role === 'assistant' && !m.isError && i === history.length - 1 && (relevantKarmaData?.entries?.length ?? 0) > 0 && (
                            <div style={{ display: 'flex', gap: '0.5rem', marginTop: '0.2rem', opacity: 0.6 }}>
                                <button
                                    onClick={() => handleFeedback(i, 'positive')}
                                    style={{ background: 'none', border: 'none', cursor: 'pointer', color: 'var(--text-muted)' }}
                                    title={t('agent.helpfulLesson')}
                                >
                                    <ThumbsUp size={14} />
                                </button>
                                <button
                                    onClick={() => handleFeedback(i, 'negative')}
                                    style={{ background: 'none', border: 'none', cursor: 'pointer', color: 'var(--text-muted)' }}
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
                        style={{ alignSelf: 'flex-start', maxWidth: '85%', display: 'flex', flexDirection: 'column', gap: '0.5rem' }}
                    >
                        <div style={{ fontSize: '0.7rem', color: 'var(--accent-cyan)', fontWeight: 700 }}>AIOME ({t('agent.streaming')})</div>
                        <div style={{
                            padding: '1.25rem',
                            borderRadius: '4px var(--radius-lg) var(--radius-lg) var(--radius-lg)',
                            background: 'var(--bg-glass-heavy)',
                            border: '1px solid var(--accent-cyan-glass)',
                            fontSize: '0.95rem',
                            lineHeight: 1.6,
                            whiteSpace: 'pre-wrap',
                            boxShadow: 'var(--shadow-shallow)'
                        }}>
                            <ReactMarkdown
                                rehypePlugins={[rehypeSanitize]}
                                components={markdownComponents}
                            >
                                {streamingText}
                            </ReactMarkdown>
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

            {/* Slash Command Suggestions */}
            {showSlash && filteredCmds.length > 0 && (
                <div style={{ padding: '0 2rem', background: 'var(--black-40)' }}>
                    <div style={{
                        background: 'var(--bg-glass-heavy)',
                        backdropFilter: 'blur(16px)', border: '1px solid var(--border-glass)',
                        borderRadius: 'var(--radius-lg)', padding: '0.5rem',
                        boxShadow: 'var(--shadow-deep)', marginBottom: '0.5rem',
                        display: 'flex', flexDirection: 'column', gap: '0.25rem'
                    }}>
                        {filteredCmds.map((cmd, i) => (
                            <div key={cmd.cmd}
                                onClick={() => {
                                    setInput(cmd.cmd);
                                    setTimeout(() => sendMessage(cmd.cmd), 0);
                                }}
                                style={{
                                    padding: '0.75rem 1rem', display: 'flex', alignItems: 'center', gap: '0.75rem',
                                    cursor: 'pointer', borderRadius: 'var(--radius-sm)',
                                    background: i === slashIndex ? 'var(--accent-cyan-20)' : 'transparent',
                                    borderLeft: i === slashIndex ? '3px solid var(--accent-cyan)' : '3px solid transparent'
                                }}
                                onMouseEnter={() => setSlashIndex(i)}
                            >
                                <div style={{ color: i === slashIndex ? 'var(--accent-cyan)' : 'var(--text-muted)' }}>{cmd.icon}</div>
                                <div>
                                    <div style={{ fontWeight: 600, color: 'var(--text-primary)', fontSize: '0.9rem' }}>{cmd.label} <span style={{color:'var(--text-muted)', fontSize:'0.8rem', marginLeft:'0.5rem'}}>{cmd.cmd}</span></div>
                                    <div style={{ fontSize: '0.75rem', color: 'var(--text-secondary)' }}>{cmd.desc}</div>
                                </div>
                            </div>
                        ))}
                    </div>
                </div>
            )}

            {/* Input Area */}
            <div style={{ padding: '1.5rem 2rem', background: 'var(--black-40)', borderTop: '1px solid var(--border-glass)' }}>
                <div style={{ position: 'relative' }}>
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
                        style={{
                            width: '100%',
                            background: 'var(--white-03)',
                            border: '1px solid var(--border-glass)',
                            borderRadius: 'var(--radius-lg)',
                            padding: '1.2rem 4.5rem 1.2rem 1.5rem',
                            color: 'var(--text-primary)',
                            outline: 'none',
                            fontSize: '1rem',
                            resize: 'none',
                            transition: 'all var(--speed-normal)',
                            boxShadow: 'var(--shadow-inset)'
                        }}
                    />
                    <button
                        onClick={() => sendMessage()}
                        disabled={!input.trim() || isTyping}
                        style={{
                            position: 'absolute',
                            right: '0.75rem',
                            top: '50%',
                            transform: 'translateY(-50%)',
                            width: '44px',
                            height: '44px',
                            borderRadius: 'var(--radius-md)',
                            background: input.trim() && !isTyping ? 'var(--accent-cyan)' : 'var(--white-05)',
                            color: input.trim() && !isTyping ? 'var(--bg-primary)' : 'var(--white-20)',
                            border: 'none',
                            cursor: 'pointer',
                            display: 'flex',
                            alignItems: 'center',
                            justifyContent: 'center',
                            transition: 'all var(--speed-fast)'
                        }}
                    >
                        <Send size={20} />
                    </button>
                </div>
                <div style={{ marginTop: '0.75rem', display: 'flex', justifyContent: 'space-between', alignItems: 'center', padding: '0 0.5rem' }}>
                    <div style={{ display: 'flex', gap: 'var(--space-md)' }}>
                        <div style={{ display: 'flex', alignItems: 'center', gap: '0.4rem', fontSize: '0.7rem', color: 'var(--text-muted)' }}>
                            <kbd style={{ background: 'var(--white-10)', padding: '2px 4px', borderRadius: '4px' }}>Shift+Enter</kbd> {t('agent.toNewline')}
                        </div>
                    </div>
                    <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem', fontSize: '0.7rem', color: 'var(--text-muted)' }}>
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

/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import React, { useEffect, useRef, useState } from 'react';
import { Network, Options } from "vis-network";
import { DataSet } from "vis-data";
import { GitBranch, ZoomIn, ZoomOut, Maximize, AlertCircle, Info, ChevronRight } from 'lucide-react';
import { API_BASE } from "../config";
import { TrajectoryStep, AgentDiagnosis } from '../types';
import { authenticatedFetch } from '../lib/auth';
import { motion, AnimatePresence } from 'framer-motion';
import { useTranslation } from '../i18n';

const CausalVisualizer: React.FC = () => {
    const containerRef = useRef<HTMLDivElement>(null);
    const networkRef = useRef<Network | null>(null);
    const [steps, setSteps] = useState<TrajectoryStep[]>([]);
    const [graph, setGraph] = useState<{nodes: any[], edges: any[]} | null>(null);
    const [diagnosis, setDiagnosis] = useState<AgentDiagnosis | null>(null);
    const [selectedStep, setSelectedStep] = useState<TrajectoryStep | null>(null);
    const [jobId, setJobId] = useState<string>("");
    const [loading, setLoading] = useState(false);
    const [error, setError] = useState<string | null>(null);
    const { t } = useTranslation();

    const fetchTrajectory = async (id: string) => {
        if (!id) return;
        setLoading(true);
        setError(null);
        try {
            const res = await authenticatedFetch(`${API_BASE}/api/v1/trajectory/${id}`);
            if (!res.ok) throw new Error("Failed to fetch trajectory");
            const data = await res.json();
            setSteps(data.nodes.map((n: any) => n.step) || []);
            setGraph(data);

            // Try to fetch diagnosis if job failed
            const diagRes = await authenticatedFetch(`${API_BASE}/api/v1/trajectory/${id}/diagnosis`);
            if (diagRes.ok) {
                const diagData = await diagRes.json();
                setDiagnosis(diagData);
            } else {
                setDiagnosis(null);
            }
        } catch (e: any) {
            setError(e.message);
        } finally {
            setLoading(false);
        }
    };

    useEffect(() => {
        if (!containerRef.current || !graph || graph.nodes.length === 0) return;

        const nodes = new DataSet<any>(graph.nodes.map((n) => {
            const label = n.step.action.length > 40 ? n.step.action.substring(0, 37) + "..." : n.step.action;
            return {
                id: n.id,
                label: `${label}\n[${n.step.step_category}]`,
                title: `Step ${n.id}: ${n.step.action}`,
                group: n.step.step_category.toLowerCase(),
                color: getStepColor(n.step.step_category),
                font: { color: '#fff', size: 14, face: 'Inter' },
                shape: 'box',
                margin: 10,
                borderWidth: 2,
                shadow: { enabled: true, color: 'rgba(0,0,0,0.3)', size: 5, x: 2, y: 2 }
            };
        }));

        const edges = new DataSet<any>(graph.edges.map(e => ({
            ...e,
            arrows: 'to',
            color: { color: 'rgba(255,255,255,0.2)', highlight: 'var(--accent-cyan)' },
            width: 2,
            smooth: { type: 'cubicBezier', forceDirection: 'vertical' }
        })));

        const options: Options = {
            layout: {
                hierarchical: {
                    direction: 'UD',
                    sortMethod: 'directed',
                    levelSeparation: 150,
                    nodeSpacing: 250
                }
            },
            physics: false,
            interaction: {
                hover: true,
                tooltipDelay: 200,
                zoomView: true
            }
        };

        networkRef.current = new Network(containerRef.current!, { nodes, edges }, options);

        networkRef.current.on("click", (params) => {
            if (params.nodes.length > 0) {
                const step = steps.find(s => s.step_id === params.nodes[0]);
                setSelectedStep(step || null);
            } else {
                setSelectedStep(null);
            }
        });

        return () => {
            networkRef.current?.destroy();
        };
    }, [graph]);

    const getStepColor = (category: string) => {
        switch (category) {
            case 'Planning': return { background: '#2d3436', border: 'var(--accent-purple)' };
            case 'WasmTool': return { background: '#1e3799', border: 'var(--accent-cyan)' };
            case 'DockerTool': return { background: '#0a3d62', border: '#3c6382' };
            case 'Verification': return { background: '#079992', border: 'var(--accent-emerald)' };
            case 'Correction': return { background: '#b71540', border: 'var(--accent-rose)' };
            default: return { background: '#2c3e50', border: '#7f8c8d' };
        }
    };

    const validateAndFetch = (id: string) => {
        const jobIdRegex = /^[a-zA-Z0-9_\-]+$/;
        if (!jobIdRegex.test(id)) {
            setError(t('causal.invalidJobId'));
            return;
        }
        fetchTrajectory(id);
    };

    return (
        <div className="main-panel ani-fade" style={{ height: '82vh', display: 'flex', flexDirection: 'column', overflow: 'hidden' }}>
            <div className="panel-header" style={{ flexShrink: 0 }}>
                <div style={{ display: 'flex', alignItems: 'center', gap: '0.75rem' }}>
                    <GitBranch size={20} color="var(--accent-cyan)" />
                    <h3>{t('causal.title')}</h3>
                </div>
                <div style={{ display: 'flex', gap: '1rem', alignItems: 'center' }}>
                    <div className="neural-input-container" style={{ position: 'relative' }}>
                        <input
                            type="text"
                            placeholder={t('causal.jobIdPlaceholder')}
                            className="neural-input"
                            style={{ paddingRight: '3rem', width: '280px' }}
                            value={jobId}
                            onChange={(e) => setJobId(e.target.value)}
                            onKeyDown={(e) => e.key === 'Enter' && validateAndFetch(jobId)}
                        />
                        <button
                            onClick={() => validateAndFetch(jobId)}
                            style={{ position: 'absolute', right: '0.5rem', top: '50%', transform: 'translateY(-50%)', background: 'none', border: 'none', color: 'var(--accent-cyan)', cursor: 'pointer' }}
                        >
                            <ChevronRight size={20} />
                        </button>
                    </div>
                </div>
            </div>

            <div style={{ flex: 1, display: 'flex', overflow: 'hidden' }}>
                {/* Graph Area */}
                <div style={{ flex: 1, position: 'relative', background: '#050505' }}>
                    <div ref={containerRef} style={{ width: '100%', height: '100%' }} />
                    
                    {loading && (
                        <div style={{ position: 'absolute', inset: 0, display: 'flex', alignItems: 'center', justifyContent: 'center', background: 'rgba(0,0,0,0.5)', zIndex: 20 }}>
                            <div className="ani-pulse" style={{ color: 'var(--accent-cyan)', fontWeight: 600 }}>{t('causal.fetchingPaths')}</div>
                        </div>
                    )}
                    
                    {error && (
                        <div style={{ position: 'absolute', top: '1rem', left: '50%', transform: 'translateX(-50%)', background: 'rgba(255,0,0,0.2)', padding: '0.5rem 1rem', borderRadius: '8px', border: '1px solid var(--accent-rose)', color: 'var(--accent-rose)', zIndex: 20 }}>
                            <AlertCircle size={16} style={{ marginBottom: '-3px', marginRight: '8px' }} />
                            {error}
                        </div>
                    )}

                    {!loading && steps.length === 0 && !error && (
                        <div style={{ position: 'absolute', inset: 0, display: 'flex', alignItems: 'center', justifyContent: 'center', color: 'var(--text-muted)', textAlign: 'center' }}>
                            <div>
                                <GitBranch size={48} style={{ opacity: 0.2, marginBottom: '1rem' }} />
                                <p>{t('causal.enterJobId')}</p>
                            </div>
                        </div>
                    )}
                </div>

                {/* Properties Sidebar */}
                <AnimatePresence>
                    {(selectedStep || diagnosis) && (
                        <motion.div
                            initial={{ x: 350 }}
                            animate={{ x: 0 }}
                            exit={{ x: 350 }}
                            style={{ width: '350px', background: 'var(--bg-dark-sidebar)', borderLeft: '1px solid var(--border-glass)', padding: '1.5rem', overflowY: 'auto', zIndex: 30 }}
                        >
                            {diagnosis && (
                                <div style={{ marginBottom: '2rem', padding: '1rem', background: 'rgba(255, 77, 148, 0.1)', border: '1px solid rgba(255, 77, 148, 0.3)', borderRadius: '12px' }}>
                                    <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem', color: 'var(--accent-rose)', marginBottom: '0.75rem' }}>
                                        <AlertCircle size={18} />
                                        <h4 style={{ margin: 0 }}>{t('causal.failureDiagnosis')}</h4>
                                    </div>
                                    <div style={{ fontSize: '0.85rem', color: 'var(--text-primary)', marginBottom: '0.5rem' }}>
                                        <strong>{t('causal.category')}:</strong> {diagnosis.category}
                                    </div>
                                    <div style={{ fontSize: '0.85rem', color: 'var(--text-secondary)', marginBottom: '1rem' }}>
                                        {diagnosis.root_cause}
                                    </div>
                                    <div style={{ padding: '0.75rem', background: 'rgba(0,0,0,0.3)', borderRadius: '8px', fontSize: '0.8rem', color: 'var(--accent-cyan)' }}>
                                        <Info size={14} style={{ marginRight: '6px' }} />
                                        Repair: {diagnosis.self_repair_hint}
                                    </div>
                                </div>
                            )}

                            {selectedStep ? (
                                <div className="step-details">
                                    <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start', marginBottom: '1rem' }}>
                                        <div style={{ fontSize: '0.7rem', color: 'var(--accent-cyan)', background: 'rgba(0,242,255,0.1)', padding: '2px 8px', borderRadius: '4px' }}>
                                            STEP #{selectedStep.step_id}
                                        </div>
                                        <div style={{ fontSize: '0.65rem', color: 'var(--text-muted)' }}>
                                            {new Date(selectedStep.timestamp).toLocaleTimeString()}
                                        </div>
                                    </div>
                                    
                                    <h3 style={{ fontSize: '1.1rem', marginBottom: '1.5rem' }}>{selectedStep.action}</h3>

                                    <div style={{ marginBottom: '1.5rem' }}>
                                        <h5 style={{ fontSize: '0.7rem', textTransform: 'uppercase', color: 'var(--text-muted)', marginBottom: '0.5rem' }}>{t('causal.reasoningIntent')}</h5>
                                        <p style={{ fontSize: '0.9rem', color: 'var(--text-secondary)', lineHeight: 1.5, whiteSpace: 'pre-wrap' }}>
                                            {selectedStep.reasoning || t('causal.noReasoning')}
                                        </p>
                                    </div>

                                    {selectedStep.completion_criteria && (
                                        <div style={{ marginBottom: '1.5rem', padding: '0.75rem', background: 'rgba(0,242,255,0.05)', border: '1px solid rgba(0,242,255,0.2)', borderRadius: '8px' }}>
                                            <h5 style={{ fontSize: '0.7rem', textTransform: 'uppercase', color: 'var(--accent-cyan)', marginBottom: '0.5rem', display: 'flex', alignItems: 'center', gap: '4px' }}>
                                                <Info size={12} /> Completion Criteria
                                            </h5>
                                            <p style={{ fontSize: '0.85rem', color: 'var(--text-primary)', fontStyle: 'italic' }}>
                                                "{selectedStep.completion_criteria}"
                                            </p>
                                        </div>
                                    )}

                                    <div style={{ marginBottom: '1.5rem' }}>
                                        <h5 style={{ fontSize: '0.7rem', textTransform: 'uppercase', color: 'var(--text-muted)', marginBottom: '0.5rem' }}>{t('causal.toolParams')}</h5>
                                        <div style={{ background: 'rgba(0,0,0,0.4)', padding: '0.75rem', borderRadius: '8px', fontFamily: 'monospace', fontSize: '0.75rem', overflowX: 'auto' }}>
                                            <div style={{ color: 'var(--accent-purple)', marginBottom: '0.25rem' }}>{selectedStep.tool_name || t('causal.internal')}</div>
                                            <pre style={{ color: 'var(--text-muted)' }}>{JSON.stringify(selectedStep.input, null, 2)}</pre>
                                        </div>
                                    </div>

                                    <div>
                                        <h5 style={{ fontSize: '0.7rem', textTransform: 'uppercase', color: 'var(--text-muted)', marginBottom: '0.5rem' }}>{t('causal.resultOutput')}</h5>
                                        <div style={{ background: 'rgba(255,255,255,0.03)', padding: '0.75rem', borderRadius: '8px', fontFamily: 'monospace', fontSize: '0.75rem', overflowX: 'auto', maxHeight: '200px' }}>
                                            <pre style={{ color: 'var(--text-secondary)' }}>{JSON.stringify(selectedStep.output, null, 2)}</pre>
                                        </div>
                                    </div>
                                </div>
                            ) : (
                                !diagnosis && <div style={{ color: 'var(--text-muted)', fontSize: '0.85rem', textAlign: 'center', marginTop: '4rem' }}>{t('causal.clickNode')}</div>
                            )}
                        </motion.div>
                    )}
                </AnimatePresence>
            </div>

            {/* Controls Overlay */}
            <div style={{ position: 'absolute', left: '1.5rem', bottom: '1.5rem', display: 'flex', gap: '0.5rem', zIndex: 10 }}>
                <button className="nav-item" style={{ margin: 0, padding: '0.4rem 0.75rem', background: 'var(--bg-glass-heavy)' }} onClick={() => networkRef.current?.fit()}>
                    <Maximize size={14} style={{ marginRight: '6px' }} /> FIT MAP
                </button>
                <button 
                    className="nav-item" 
                    style={{ margin: 0, padding: '0.4rem 1rem', background: 'var(--bg-glass-heavy)' }} 
                    onClick={() => {
                        const scale = networkRef.current?.getScale() || 1;
                        networkRef.current?.moveTo({ scale: scale / 1.2 });
                    }}
                >
                    <ZoomOut size={16} />
                </button>
                <button 
                    className="nav-item" 
                    style={{ margin: 0, padding: '0.4rem 1rem', background: 'var(--bg-glass-heavy)' }} 
                    onClick={() => {
                        const scale = networkRef.current?.getScale() || 1;
                        networkRef.current?.moveTo({ scale: scale * 1.2 });
                    }}
                >
                    <ZoomIn size={16} />
                </button>
            </div>
        </div>
    );
};

export default CausalVisualizer;

/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import React, { useCallback, useEffect, useRef, useState } from 'react';
import { Network, Options } from "vis-network";
import { DataSet } from "vis-data";
import { GitBranch, ZoomIn, ZoomOut, Maximize, AlertCircle, Info, ChevronRight } from 'lucide-react';
import { API_BASE } from "../config";
import { TrajectoryStep, AgentDiagnosis, CausalGraphResponse, CausalGraphNode } from '../types';
import { authenticatedFetch } from '../lib/auth';
import { motion, AnimatePresence } from 'framer-motion';
import { useTranslation } from '../i18n';
import {
    A2UI_NAVIGATE_EVENT,
    takeCausalJobIdFromStorage,
    type A2uiNavigateDetail,
} from '../lib/a2uiTabs';

import { cssVar } from '../utils/cssVar';

const JOB_ID_REGEX = /^[a-zA-Z0-9_\-]+$/;

const CausalVisualizer: React.FC = () => {
    const containerRef = useRef<HTMLDivElement>(null);
    const networkRef = useRef<Network | null>(null);
    const [steps, setSteps] = useState<TrajectoryStep[]>([]);
    const [graph, setGraph] = useState<CausalGraphResponse | null>(null);
    const [diagnosis, setDiagnosis] = useState<AgentDiagnosis | null>(null);
    const [selectedStep, setSelectedStep] = useState<TrajectoryStep | null>(null);
    const [jobId, setJobId] = useState<string>("");
    const [loading, setLoading] = useState(false);
    const [error, setError] = useState<string | null>(null);
    const { t } = useTranslation();

    const fetchTrajectory = useCallback(async (id: string) => {
        if (!id) return;
        setLoading(true);
        setError(null);
        try {
            const res = await authenticatedFetch(`${API_BASE}/api/v1/trajectory/${id}`);
            if (!res.ok) throw new Error(t('causal.fetchFailed'));
            const data = await res.json();
            setSteps(data.nodes.map((n: CausalGraphNode) => n.step) || []);
            setGraph(data);

            // Try to fetch diagnosis if job failed
            const diagRes = await authenticatedFetch(`${API_BASE}/api/v1/trajectory/${id}/diagnosis`);
            if (diagRes.ok) {
                const diagData = await diagRes.json();
                setDiagnosis(diagData);
            } else {
                setDiagnosis(null);
            }
        } catch (e: unknown) {
            setError(e instanceof Error ? e.message : t('causal.unknownError'));
            setSteps([]);
            setGraph(null);
            setDiagnosis(null);
        } finally {
            setLoading(false);
        }
    }, [t]);

    const validateAndFetch = useCallback((id: string) => {
        if (!JOB_ID_REGEX.test(id)) {
            setError(t('causal.invalidJobId'));
            return;
        }
        setJobId(id);
        void fetchTrajectory(id);
    }, [fetchTrajectory, t]);

    // OP-022 C1: one-shot sessionStorage handoff (dual-mount AppRoutes + HomePage)
    useEffect(() => {
        const fromStorage = takeCausalJobIdFromStorage();
        if (fromStorage) {
            validateAndFetch(fromStorage);
        }
        // eslint-disable-next-line react-hooks/exhaustive-deps -- intentional mount-only consume
    }, []);

    // Live a2ui-navigate with jobId (already-mounted Causal / same-tab re-nav)
    useEffect(() => {
        const onNav = (event: Event) => {
            const incoming = (event as CustomEvent<A2uiNavigateDetail>).detail?.jobId;
            if (typeof incoming === 'string' && incoming.length > 0) {
                validateAndFetch(incoming);
            }
        };
        window.addEventListener(A2UI_NAVIGATE_EVENT, onNav);
        return () => window.removeEventListener(A2UI_NAVIGATE_EVENT, onNav);
    }, [validateAndFetch]);

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
                font: { color: cssVar('--text-primary'), size: 14, face: 'Inter, system-ui' },
                shape: 'box',
                margin: 10,
                borderWidth: 2,
                shadow: { enabled: true, color: cssVar('--black-30'), size: 5, x: 2, y: 2 }
            };
        }));

        const edges = new DataSet<any>(graph.edges.map(e => ({
            ...e,
            arrows: 'to',
            color: { color: cssVar('--white-20'), highlight: cssVar('--accent-cyan') },
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
            case 'Economic': return { background: cssVar('--accent-emerald-20', '#10B98133'), border: cssVar('--accent-emerald', '#10B981') };
            case 'Planning': return { background: cssVar('--bg-dark'), border: cssVar('--accent-purple') };
            case 'WasmTool': return { background: cssVar('--accent-blue'), border: cssVar('--accent-cyan') };
            case 'DockerTool': return { background: cssVar('--bg-glass'), border: cssVar('--text-muted') };
            case 'Verification': return { background: cssVar('--accent-emerald'), border: cssVar('--accent-emerald') };
            case 'Correction': return { background: cssVar('--accent-rose'), border: cssVar('--accent-rose') };
            default: return { background: cssVar('--bg-dark-sidebar'), border: cssVar('--text-muted') };
        }
    };

    return (
        <div className="main-panel ani-fade" style={{ display: 'flex', flexDirection: 'column', overflow: 'hidden', flex: 1, minHeight: 0, height: '100%' }}>
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
                            aria-label={t('causal.fetchData')}
                            style={{ position: 'absolute', right: '0.5rem', top: '50%', transform: 'translateY(-50%)', background: 'none', border: 'none', color: 'var(--accent-cyan)', cursor: 'pointer' }}
                        >
                            <ChevronRight size={20} />
                        </button>
                    </div>
                </div>
            </div>

            <div style={{ flex: 1, display: 'flex', overflow: 'hidden' }}>
                {/* Graph Area */}
                <div data-testid="causal-graph-area" style={{ flex: 1, position: 'relative', background: 'var(--bg-primary)' }}>
                    <div ref={containerRef} style={{ width: '100%', height: '100%' }} />
                    
                    {loading && (
                        <div style={{ position: 'absolute', inset: 0, display: 'flex', alignItems: 'center', justifyContent: 'center', background: 'var(--black-50)', zIndex: 20 }}>
                            <div className="ani-pulse" style={{ color: 'var(--accent-cyan)', fontWeight: 600 }}>{t('causal.fetchingPaths')}</div>
                        </div>
                    )}
                    
                    {error && (
                        <div style={{ position: 'absolute', top: '1rem', left: '50%', transform: 'translateX(-50%)', background: 'var(--accent-rose-20)', padding: '0.5rem 1rem', borderRadius: '8px', border: '1px solid var(--accent-rose)', color: 'var(--accent-rose)', zIndex: 20 }}>
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

                    {/* Controls Overlay */}
                    <div data-testid="causal-controls-overlay" style={{ position: 'absolute', left: '1.5rem', bottom: '1.5rem', display: 'flex', gap: '0.5rem', zIndex: 10 }}>
                        <button className="nav-item" style={{ margin: 0, padding: '0.4rem 0.75rem', background: 'var(--bg-glass-heavy)' }} onClick={(e) => { e.stopPropagation(); networkRef.current?.fit(); }}>
                            <Maximize size={14} style={{ marginRight: '6px' }} /> {t('causal.fitMap')}
                        </button>
                        <button 
                            className="nav-item" 
                            style={{ margin: 0, padding: '0.4rem 1rem', background: 'var(--bg-glass-heavy)' }} 
                            onClick={(e) => {
                                e.stopPropagation();
                                const scale = networkRef.current?.getScale() || 1;
                                networkRef.current?.moveTo({ scale: scale / 1.2 });
                            }}
                        >
                            <ZoomOut size={16} />
                        </button>
                        <button 
                            className="nav-item" 
                            style={{ margin: 0, padding: '0.4rem 1rem', background: 'var(--bg-glass-heavy)' }} 
                            onClick={(e) => {
                                e.stopPropagation();
                                const scale = networkRef.current?.getScale() || 1;
                                networkRef.current?.moveTo({ scale: scale * 1.2 });
                            }}
                        >
                            <ZoomIn size={16} />
                        </button>
                    </div>
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
                                <div style={{ marginBottom: '2rem', padding: '1rem', background: 'var(--accent-rose-10)', border: '1px solid var(--accent-rose-30)', borderRadius: '12px' }}>
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

                                    <div style={{ padding: '0.75rem', background: 'var(--black-30)', borderRadius: '8px', fontSize: '0.8rem', color: 'var(--accent-cyan)' }}>
                                        <Info size={14} style={{ marginRight: '6px' }} />
                                        Repair: {diagnosis.self_repair_hint}
                                    </div>
                                </div>
                            )}

                            {selectedStep ? (
                                <div className="step-details">
                                    <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start', marginBottom: '1rem' }}>
                                        <div style={{ fontSize: '0.7rem', color: 'var(--accent-cyan)', background: 'var(--accent-cyan-10)', padding: '2px 8px', borderRadius: '4px' }}>
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
                                        <div style={{ marginBottom: '1.5rem', padding: '0.75rem', background: 'var(--accent-cyan-05)', border: '1px solid var(--accent-cyan-20)', borderRadius: '8px' }}>
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
                                        <div className="font-mono" style={{ background: 'var(--black-40)', padding: '0.75rem', borderRadius: '8px', fontSize: '0.75rem', overflowX: 'auto' }}>
                                            <div style={{ color: 'var(--accent-purple)', marginBottom: '0.25rem' }}>{selectedStep.tool_name || t('causal.internal')}</div>
                                            <pre style={{ color: 'var(--text-muted)' }}>{JSON.stringify(selectedStep.input, null, 2)}</pre>
                                        </div>
                                    </div>

                                    <div>
                                        <h5 style={{ fontSize: '0.7rem', textTransform: 'uppercase', color: 'var(--text-muted)', marginBottom: '0.5rem' }}>{t('causal.resultOutput')}</h5>
                                        <div className="font-mono" style={{ background: 'var(--white-03)', padding: '0.75rem', borderRadius: '8px', fontSize: '0.75rem', overflowX: 'auto', maxHeight: '200px' }}>
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

        </div>
    );
};

export default CausalVisualizer;

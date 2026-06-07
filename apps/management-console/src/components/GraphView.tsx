/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import React, { useEffect, useRef, useState } from 'react';
import { Network, Node as VisNode, Edge as VisEdge } from "vis-network";
import { DataSet } from "vis-data";
import { GitMerge, ZoomIn, ZoomOut, RefreshCw, Layers } from 'lucide-react';
import { API_BASE } from "../config";
import { GraphNode, GraphEdge } from '../types';
import { authenticatedFetch } from '../lib/auth';
import { useTranslation } from '../i18n';
import { useGraphTheme } from '../hooks/useGraphTheme';

import { cssVar } from '../utils/cssVar';

interface ArtifactData {
    id: string;
    title: string;
    category: string;
    karma_refs?: string[];
}

const GraphView: React.FC = () => {
    const containerRef = useRef<HTMLDivElement>(null);
    const networkRef = useRef<Network | null>(null);
    const [nodeCount, setNodeCount] = useState(0);
    const [artifactCount, setArtifactCount] = useState(0);
    const [showArtifacts, setShowArtifacts] = useState(true);
    const { t } = useTranslation();
    const theme = useGraphTheme();

    useEffect(() => {
        const container = containerRef.current;
        if (!container) return;

        const initGraph = async () => {
            try {
                // Parallel fetch for Karma and Artifacts
                const [karmaRes, artifactRes] = await Promise.all([
                    authenticatedFetch(`${API_BASE}/api/synergy/graph`),
                    authenticatedFetch(`${API_BASE}/api/artifacts?limit=50`)
                ]);

                if (!karmaRes.ok || !artifactRes.ok) {
                    console.error("Graph API error", { karma: karmaRes.status, artifact: artifactRes.status });
                    return;
                }

                const karmaData = await karmaRes.json();
                const artifacts = await artifactRes.json();

                // Double check container existence before creating Network instance
                if (!containerRef.current) return;

                // 1. Process Memory Nodes/Edges
                const safeNodes = Array.isArray(karmaData?.nodes) ? karmaData.nodes : [];
                const nodes = new DataSet<VisNode>(safeNodes.map((n: GraphNode) => ({
                    ...n,
                    color: {
                        background: n.group === 'karma_local' ? theme.nodes.karmaLocal.background : theme.nodes.karmaForeign.background,
                        border: n.group === 'karma_local' ? theme.nodes.karmaLocal.border : theme.nodes.karmaForeign.border,
                        highlight: {
                            background: n.group === 'karma_local' ? theme.nodes.karmaLocal.highlight.background : theme.nodes.karmaForeign.highlight.background,
                            border: theme.nodes.karmaLocal.highlight.border,
                        }
                    },
                    font: { color: theme.text, size: 12, face: 'Artemis Inter, Inter' },
                    shape: 'dot',
                    size: 20 + (n.label.length / 5)
                })));

                const safeEdges = Array.isArray(karmaData?.edges) ? karmaData.edges : [];
                const edges = new DataSet<VisEdge>(safeEdges.map((e: GraphEdge) => ({
                    ...e,
                    color: { color: theme.edges.default.color, highlight: theme.edges.default.highlight },
                    width: 1,
                    smooth: { type: 'continuous' }
                })));

                // 2. Add Artifact Nodes if toggled
                const safeArtifacts = Array.isArray(artifacts) ? artifacts : [];
                setArtifactCount(safeArtifacts.length);

                if (showArtifacts) {
                    safeArtifacts.forEach((art: ArtifactData) => {
                        nodes.add({
                            id: art.id,
                            label: `📦 ${art.title}`,
                            group: 'artifact',
                            color: {
                                background: theme.nodes.artifact.background,
                                border: theme.nodes.artifact.border,
                                highlight: { background: theme.nodes.artifact.highlight.background, border: theme.nodes.artifact.highlight.border }
                            },
                            font: { color: theme.nodes.artifact.font, size: 13, bold: "bold" },
                            shape: 'diamond',
                            size: 25,
                            title: `Category: ${art.category}`
                        });

                        // Add edges from Memory refs if present
                        if (art.karma_refs) {
                            art.karma_refs.forEach((karmaId: string) => {
                                edges.add({
                                    from: karmaId,
                                    to: art.id,
                                    label: 'materialized',
                                    color: { color: theme.edges.materialized.color },
                                    dashes: true,
                                    width: 1
                                });
                            });
                        }
                    });
                }

                setNodeCount(nodes.length);

                const options = {
                    nodes: {
                        borderWidth: 2,
                        shadow: { enabled: true, color: theme.shadow, size: 10, x: 5, y: 5 }
                    },
                    edges: { arrows: 'to' },
                    physics: {
                        stabilization: true,
                        barnesHut: {
                            gravitationalConstant: -2500,
                            centralGravity: 0.3,
                            springLength: 120,
                            springConstant: 0.04,
                            damping: 0.09,
                            avoidOverlap: 0.2
                        }
                    },
                    interaction: {
                        hover: true,
                        tooltipDelay: 200,
                        zoomView: true
                    }
                };

                networkRef.current = new Network(container, { nodes, edges }, options);
            } catch (e) {
                console.error("Graph failed to load", e);
            }
        };

        initGraph();

        return () => {
            networkRef.current?.destroy();
        };
    }, [theme, showArtifacts, t]);

    const zoomIn = () => networkRef.current?.moveTo({ scale: (networkRef.current?.getScale() || 1) * 1.2 });
    const zoomOut = () => networkRef.current?.moveTo({ scale: (networkRef.current?.getScale() || 1) / 1.2 });
    const fit = () => networkRef.current?.fit();

    return (
        <div className="main-panel ani-fade" style={{ display: 'flex', flexDirection: 'column', padding: 0, position: 'relative', flex: 1, minHeight: 0 }}>
            <div className="panel-header" style={{ padding: '1rem 1.5rem', borderBottom: '1px solid var(--border-glass)', zIndex: 10 }}>
                <div style={{ display: 'flex', alignItems: 'center', gap: '1rem' }}>
                    <GitMerge size={20} color="var(--accent-cyan)" />
                    <h3>{t('graph.title')}</h3>
                </div>
                <div style={{ display: 'flex', gap: '1.5rem', alignItems: 'center' }}>
                    <div style={{ display: 'flex', gap: '1rem' }}>
                        <div style={{ fontSize: '0.75rem', color: 'var(--text-muted)' }}>{Math.max(0, nodeCount - (showArtifacts ? artifactCount : 0))} {t('graph.karma')}</div>
                        <div style={{ fontSize: '0.75rem', color: 'var(--accent-rose)' }}>{showArtifacts ? artifactCount : 0} {t('graph.artifacts')}</div>
                    </div>
                    <button className="nav-item" style={{ margin: 0, padding: '0.4rem 0.75rem' }} onClick={fit}>
                        <RefreshCw size={14} /> {t('graph.reCenter')}
                    </button>
                </div>
            </div>

            <div ref={containerRef} style={{ flex: 1, background: `radial-gradient(circle at center, ${theme.background.gradientInner} 0%, ${theme.background.gradientOuter} 100%)` }} />

            {/* Overlay Controls */}
            <div style={{ position: 'absolute', right: '1.5rem', bottom: '1.5rem', display: 'flex', flexDirection: 'column', gap: '0.5rem', zIndex: 10 }}>
                <button
                    onClick={zoomIn}
                    style={{ width: '40px', height: '40px', background: 'var(--bg-glass-heavy)', border: '1px solid var(--border-glass)', borderRadius: '8px', color: cssVar('--text-primary', 'var(--text-primary)'), cursor: 'pointer', display: 'flex', alignItems: 'center', justifyContent: 'center' }}
                >
                    <ZoomIn size={18} />
                </button>
                <button
                    onClick={zoomOut}
                    style={{ width: '40px', height: '40px', background: 'var(--bg-glass-heavy)', border: '1px solid var(--border-glass)', borderRadius: '8px', color: cssVar('--text-primary', 'var(--text-primary)'), cursor: 'pointer', display: 'flex', alignItems: 'center', justifyContent: 'center' }}
                >
                    <ZoomOut size={18} />
                </button>
                <button
                    onClick={() => setShowArtifacts(!showArtifacts)}
                    style={{
                        width: '40px',
                        height: '40px',
                        background: showArtifacts ? 'var(--accent-cyan-15)' : 'var(--bg-glass-heavy)',
                        border: showArtifacts ? '1px solid var(--accent-cyan)' : '1px solid var(--border-glass)',
                        borderRadius: '8px',
                        color: showArtifacts ? 'var(--accent-cyan)' : cssVar('--text-primary', 'var(--text-primary)'),
                        cursor: 'pointer',
                        display: 'flex',
                        alignItems: 'center',
                        justifyContent: 'center',
                        transition: 'all 0.2s ease-in-out'
                    }}
                    title={showArtifacts ? (t('graph.hideArtifacts') || "Hide Artifacts") : (t('graph.showArtifacts') || "Show Artifacts")}
                >
                    <Layers size={18} />
                </button>
            </div>

            {/* Hint */}
            <div style={{ position: 'absolute', left: '1.5rem', bottom: '1.5rem', background: theme.shadow, padding: '0.5rem 1rem', borderRadius: '8px', border: '1px solid var(--border-glass)', fontSize: '0.75rem', color: 'var(--text-muted)', zIndex: 10 }}>
                {t('graph.hint')}
            </div>
        </div>
    );
};

export default GraphView;

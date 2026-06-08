/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import React, { useState, useCallback } from 'react';
import { A2uiEnvelope, A2uiComponent, A2uiSurface, A2uiMetric, A2uiTimelineEvent } from '../types';
import { useTokenHealth } from '../hooks/useTokenHealth';
import { API_BASE } from '../config';
import { authenticatedFetch } from '../lib/auth';
import { TreasureBox } from './TreasureBox';
import VoiceStore from './VoiceStore';
import LoraTrainingView from './LoraTrainingView';
interface A2uiRendererProps {
    envelope: A2uiEnvelope;
}

/**
 * 再帰的コンポーネントレンダラー。
 * A2uiValidator が通過済みの安全なコンポーネントのみがここに到達する。
 * React JSX は文字列をデフォルトでエスケープするため、XSS 安全性を維持。
 * dangerouslySetInnerHTML は使用禁止。
 */
const ComponentRenderer: React.FC<{ component: A2uiComponent, onAction: (action: string) => void, isSubmitting?: boolean }> = ({ component, onAction, isSubmitting = false }) => {
    const content = component.props?.content;

    switch (component.type) {
        case 'text':
            return (
                <div style={{
                    fontSize: '0.875rem',
                    color: 'var(--text-secondary)',
                    lineHeight: 1.6,
                }}>
                    {typeof content === 'string' ? content : ''}
                </div>
            );
        case 'button':
            return (
                <button
                    style={{
                        padding: '0.5rem 1rem',
                        margin: '0.25rem 0',
                        fontSize: '0.875rem',
                        fontWeight: 600,
                        background: 'var(--white-05)',
                        border: '1px solid var(--border-glass-bright)',
                        borderRadius: 'var(--radius-sm)',
                        color: 'var(--text-primary)',
                        cursor: isSubmitting ? 'wait' : 'pointer',
                        transition: `all var(--speed-fast)`,
                        opacity: isSubmitting ? 0.6 : 1,
                    }}
                    disabled={isSubmitting}
                    onClick={() => {
                        if (typeof component.props?.action === 'string') {
                            onAction(component.props.action);
                        }
                    }}
                >
                    {isSubmitting ? '処理中…' : (typeof component.props?.label === 'string' ? component.props.label : 'Action')}
                </button>
            );
        case 'list':
            return (
                <ul style={{
                    listStyleType: 'disc',
                    paddingLeft: '1.25rem',
                    margin: '0.5rem 0',
                    display: 'flex',
                    flexDirection: 'column',
                    gap: '0.25rem',
                    fontSize: '0.875rem',
                    color: 'var(--text-primary)',
                }}>
                    {(component.children ?? []).map((child, i) => (
                        <li key={i}><ComponentRenderer component={child} onAction={onAction} isSubmitting={isSubmitting} /></li>
                    ))}
                </ul>
            );
        case 'form':
            return (
                <div style={{
                    display: 'flex',
                    flexDirection: 'column',
                    gap: '0.5rem',
                    margin: '0.5rem 0',
                    width: '100%',
                    maxWidth: '28rem',
                }}>
                    {(component.children ?? []).map((child, i) => (
                        <ComponentRenderer key={i} component={child} onAction={onAction} isSubmitting={isSubmitting} />
                    ))}
                </div>
            );
        case 'input':
            return (
                <input
                    type={typeof component.props?.inputType === 'string' ? component.props.inputType : 'text'}
                    placeholder={typeof component.props?.placeholder === 'string' ? component.props.placeholder : ''}
                    readOnly // Phase 0: 送信機能は未実装
                    style={{
                        padding: '0.5rem 0.75rem',
                        fontSize: '0.875rem',
                        background: 'var(--bg-secondary)',
                        border: '1px solid var(--border-glass)',
                        borderRadius: 'var(--radius-sm)',
                        color: 'var(--text-primary)',
                        outline: 'none',
                        width: '100%',
                        transition: `border-color var(--speed-fast)`,
                    }}
                />
            );
        case 'taskApproval':
            return (
                <div style={{
                    display: 'flex',
                    flexDirection: 'column',
                    gap: '0.75rem',
                    padding: '1rem',
                    background: 'var(--bg-glass-light)',
                    border: '1px solid var(--border-glass-bright)',
                    borderRadius: 'var(--radius-md)',
                    borderLeft: component.props?.riskLevel === 'high' ? '3px solid var(--accent-rose)' : '3px solid var(--accent-amber)',
                    margin: '0.5rem 0',
                }}>
                    {component.props?.title ? <h4 style={{ margin: 0, fontSize: '0.95rem', color: 'var(--text-primary)' }}>{String(component.props.title)}</h4> : null}
                    {component.props?.description ? <p style={{ margin: 0, fontSize: '0.85rem', color: 'var(--text-secondary)' }}>{String(component.props.description)}</p> : null}
                    {component.children && component.children.length > 0 && (
                        <div style={{ display: 'flex', gap: '0.5rem', marginTop: '0.5rem' }}>
                            {component.children.map((child, i) => <ComponentRenderer key={i} component={child} onAction={onAction} isSubmitting={isSubmitting} />)}
                        </div>
                    )}
                </div>
            );
        case 'taskResult':
            return (
                <div style={{
                    display: 'flex',
                    alignItems: 'center',
                    gap: '0.75rem',
                    padding: '0.75rem',
                    background: component.props?.success ? 'var(--accent-emerald-10)' : 'var(--accent-rose-10)',
                    border: '1px solid',
                    borderColor: component.props?.success ? 'var(--accent-emerald-20)' : 'var(--accent-rose-30)',
                    borderRadius: 'var(--radius-sm)',
                    margin: '0.5rem 0',
                }}>
                    <span style={{ color: component.props?.success ? 'var(--accent-emerald)' : 'var(--accent-rose)', fontSize: '1.25rem' }}>
                        {component.props?.success ? '✓' : '✗'}
                    </span>
                    <span style={{ fontSize: '0.875rem', color: 'var(--text-primary)' }}>
                        {component.props?.message ? String(component.props.message) : (component.props?.success ? 'Success' : 'Failed')}
                    </span>
                </div>
            );
        case 'treasureItem':
            return (
                <div style={{ margin: '0.5rem 0', width: '100%', maxWidth: '28rem' }}>
                    <TreasureBox />
                </div>
            );
        case 'voiceStore':
            return (
                <div style={{ margin: '0.5rem 0', width: '100%' }}>
                    <VoiceStore />
                </div>
            );
        case 'loraMarket':
            return (
                <div style={{ margin: '0.5rem 0', width: '100%' }}>
                    <LoraTrainingView />
                </div>
            );
        case 'progressBar':
            const progress = Number(component.props?.progress || 0);
            return (
                <div style={{ width: '100%', margin: '0.5rem 0' }}>
                    {component.props?.label ? <div style={{ fontSize: '0.8rem', color: 'var(--text-secondary)', marginBottom: '0.25rem' }}>{String(component.props.label)}</div> : null}
                    <div style={{ height: '8px', background: 'var(--white-10)', borderRadius: '4px', overflow: 'hidden' }}>
                        <div role="progressbar" aria-valuenow={progress} style={{ width: `${Math.max(0, Math.min(100, progress))}%`, height: '100%', background: 'var(--accent-cyan)', transition: 'width 0.3s' }} />
                    </div>
                </div>
            );
        case 'alert':
            const severity = String(component.props?.severity || 'info');
            const alertColor = severity === 'error' ? 'var(--accent-rose)' : severity === 'warning' ? 'var(--accent-amber)' : 'var(--accent-cyan)';
            const alertBg = severity === 'error' ? 'var(--accent-rose-10)' : severity === 'warning' ? 'var(--accent-amber-10)' : 'var(--accent-cyan-10)';
            return (
                <div style={{ padding: '0.75rem', background: alertBg, borderLeft: `3px solid ${alertColor}`, borderRadius: 'var(--radius-sm)', color: 'var(--text-primary)', fontSize: '0.875rem', margin: '0.5rem 0' }}>
                    {component.props?.message ? <div>{String(component.props.message)}</div> : null}
                </div>
            );
        case 'card':
            return (
                <div style={{ padding: '1rem', background: 'var(--bg-glass-light)', border: '1px solid var(--border-glass-bright)', borderRadius: 'var(--radius-md)', margin: '0.5rem 0' }}>
                    {component.props?.title ? <h4 style={{ margin: '0 0 0.5rem 0', color: 'var(--text-primary)', fontSize: '1rem' }}>{String(component.props.title)}</h4> : null}
                    {component.props?.content ? <p style={{ margin: 0, color: 'var(--text-secondary)', fontSize: '0.875rem' }}>{String(component.props.content)}</p> : null}
                </div>
            );
        case 'codeBlock':
            return (
                <pre style={{ padding: '0.75rem', background: 'var(--black-50)', borderRadius: 'var(--radius-sm)', overflowX: 'auto', fontSize: '0.8rem', fontFamily: 'var(--font-mono)', color: 'var(--accent-cyan)', margin: '0.5rem 0' }}>
                    <code>{String(component.props?.code || '')}</code>
                </pre>
            );
        case 'chart':
            const metrics: A2uiMetric[] = Array.isArray(component.props?.metrics) ? component.props.metrics : [];
            return (
                <div style={{ padding: '1rem', background: 'var(--bg-secondary)', borderRadius: 'var(--radius-md)', margin: '0.5rem 0' }}>
                    {component.props?.title ? <h4 style={{ margin: '0 0 0.75rem 0', color: 'var(--text-primary)', fontSize: '0.9rem' }}>{String(component.props.title)}</h4> : null}
                    <div style={{ display: 'flex', flexDirection: 'column', gap: '0.5rem' }}>
                        {metrics.map((m: A2uiMetric, i: number) => (
                            <div key={i} style={{ display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
                                <span style={{ width: '80px', fontSize: '0.75rem', color: 'var(--text-secondary)', textOverflow: 'ellipsis', overflow: 'hidden', whiteSpace: 'nowrap' }}>{m.label}</span>
                                <div style={{ flex: 1, height: '6px', background: 'var(--white-10)', borderRadius: '3px', overflow: 'hidden' }}>
                                    <div style={{ width: `${Math.min(100, Math.max(0, m.value || 0))}%`, height: '100%', background: 'var(--accent-purple)' }} />
                                </div>
                                <span style={{ width: '30px', fontSize: '0.75rem', color: 'var(--text-muted)', textAlign: 'right' }}>{m.value}</span>
                            </div>
                        ))}
                    </div>
                </div>
            );
        case 'dataTable':
            const cols: string[] = Array.isArray(component.props?.columns) ? component.props.columns : [];
            const rows: Record<string, unknown>[] = Array.isArray(component.props?.rows) ? component.props.rows : [];
            return (
                <div style={{ overflowX: 'auto', margin: '0.5rem 0', border: '1px solid var(--border-glass)', borderRadius: 'var(--radius-sm)' }}>
                    <table style={{ width: '100%', borderCollapse: 'collapse', fontSize: '0.8rem' }}>
                        <thead style={{ background: 'var(--white-05)' }}>
                            <tr>{cols.map((c: string, i: number) => <th key={i} style={{ padding: '0.5rem', textAlign: 'left', color: 'var(--text-secondary)', borderBottom: '1px solid var(--border-glass)' }}>{c}</th>)}</tr>
                        </thead>
                        <tbody>
                            {rows.map((row: Record<string, unknown>, i: number) => (
                                <tr key={i} style={{ borderBottom: '1px solid var(--border-glass-dim)' }}>
                                    {cols.map((c: string, j: number) => <td key={j} style={{ padding: '0.5rem', color: 'var(--text-primary)' }}>{String(row[c] ?? '')}</td>)}
                                </tr>
                            ))}
                        </tbody>
                    </table>
                </div>
            );
        case 'cellStatus':
            const statusColor = component.props?.status === 'active' ? 'var(--accent-emerald)' : component.props?.status === 'error' ? 'var(--accent-rose)' : 'var(--text-muted)';
            return (
                <span style={{ display: 'inline-flex', alignItems: 'center', gap: '0.25rem', fontSize: '0.75rem', padding: '0.2rem 0.5rem', borderRadius: '1rem', background: 'var(--white-05)', border: `1px solid ${statusColor}40` }}>
                    <span style={{ width: '6px', height: '6px', borderRadius: '50%', background: statusColor }}></span>
                    {String(component.props?.label || component.props?.status || 'unknown')}
                </span>
            );
        case 'timeline':
            const events: A2uiTimelineEvent[] = Array.isArray(component.props?.events) ? component.props.events : [];
            return (
                <div style={{ display: 'flex', flexDirection: 'column', gap: '0.5rem', margin: '0.5rem 0', paddingLeft: '0.5rem', borderLeft: '2px solid var(--border-glass)' }}>
                    {events.map((ev: A2uiTimelineEvent, i: number) => (
                        <div key={i} style={{ position: 'relative', paddingLeft: '1rem' }}>
                            <div style={{ position: 'absolute', left: '-0.6rem', top: '0.25rem', width: '8px', height: '8px', borderRadius: '50%', background: 'var(--accent-cyan)' }} />
                            <div style={{ fontSize: '0.75rem', color: 'var(--text-muted)' }}>{ev.time}</div>
                            <div style={{ fontSize: '0.85rem', color: 'var(--text-primary)' }}>{ev.title}</div>
                        </div>
                    ))}
                </div>
            );
        default:
            return (
                <div style={{
                    padding: '0.5rem',
                    border: '1px solid var(--border-glass)',
                    borderRadius: 'var(--radius-sm)',
                    background: 'var(--bg-secondary)',
                    color: 'var(--text-muted)',
                    fontSize: '0.75rem',
                    margin: '0.25rem 0',
                }}>
                    <span style={{ fontFamily: 'var(--font-mono)', color: 'var(--accent-cyan)' }}>
                        [{component.type}]
                    </span>{' '}
                    <span style={{ opacity: 0.7 }}>Component data encoded</span>
                </div>
            );
    }
};

const SurfaceRenderer: React.FC<{ surface: A2uiSurface, onAction: (action: string) => void, isSubmitting: boolean }> = ({ surface, onAction, isSubmitting }) => {
    return (
        <div style={{ display: 'flex', flexDirection: 'column', gap: '0.75rem' }}>
            <div style={{ display: 'flex', flexDirection: 'column', gap: '0.5rem' }}>
                {surface.components.map((comp, i) => (
                    <ComponentRenderer key={i} component={comp} onAction={onAction} isSubmitting={isSubmitting} />
                ))}
            </div>
        </div>
    );
};

export const A2uiRenderer: React.FC<A2uiRendererProps> = ({ envelope }) => {
    const { checkHealth } = useTokenHealth();
    const [submittingData, setSubmittingData] = useState(false);

    const handleAction = useCallback(async (action: string, surfaceId: string) => {
        if (submittingData) return;
        setSubmittingData(true);
        try {
            const res = await authenticatedFetch(`${API_BASE}/api/v1/a2ui/action`, {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ surface_id: surfaceId, action })
            });

            if (res.status === 401) {
                checkHealth();
            }

            if (!res.ok) {
                console.error('[A2UI] Action failed:', await res.text());
                return;
            }

            console.log('[A2UI] Action Success:', action);
        } catch (e) {
            console.error('[A2UI] Network error during action:', e);
        } finally {
            setSubmittingData(false);
        }
    }, [submittingData, checkHealth]);

    if (envelope.type === 'createSurface') {
        // Runtime guard: discriminated unions don't guarantee shape at runtime
        const surface = envelope.surface;
        if (!surface || !Array.isArray(surface.components)) {
            return null;
        }
        const surfaceId = surface.id ?? 'unknown';
        return (
            <div style={{
                width: '100%',
                padding: 'var(--space-md)',
                borderRadius: 'var(--radius-md)',
                background: 'var(--bg-glass-heavy)',
                border: '1px solid var(--border-glass-bright)',
                boxShadow: 'var(--shadow-deep)',
                marginTop: '1rem',
                marginBottom: '1rem',
                overflow: 'hidden',
                position: 'relative',
            }}>
                {/* Accent bar */}
                <div style={{
                    position: 'absolute',
                    top: 0,
                    left: 0,
                    width: '3px',
                    height: '100%',
                    background: 'linear-gradient(to bottom, var(--accent-cyan), var(--accent-purple))',
                    opacity: 0.8,
                }} />
                {/* Header */}
                <div style={{
                    display: 'flex',
                    justifyContent: 'space-between',
                    alignItems: 'center',
                    marginBottom: 'var(--space-sm)',
                    paddingBottom: '0.75rem',
                    borderBottom: '1px solid var(--border-glass)',
                }}>
                    <span style={{
                        fontSize: '0.7rem',
                        fontFamily: 'var(--font-mono)',
                        fontWeight: 700,
                        color: 'var(--accent-cyan)',
                        textTransform: 'uppercase',
                        letterSpacing: '0.1em',
                        display: 'flex',
                        alignItems: 'center',
                        gap: '0.5rem',
                    }}>
                        <span style={{
                            width: '6px',
                            height: '6px',
                            borderRadius: '50%',
                            background: 'var(--accent-cyan)',
                            boxShadow: 'var(--glow-cyan)',
                        }} />
                        A2UI Surface
                    </span>
                    <span style={{
                        fontSize: '0.6rem',
                        color: 'var(--text-muted)',
                        fontFamily: 'var(--font-mono)',
                        opacity: 0.6,
                    }}>
                        {surfaceId.length > 8
                            ? `ID: ${surfaceId.slice(0, 8)}…`
                            : `ID: ${surfaceId}`}
                    </span>
                </div>
                <SurfaceRenderer surface={surface} onAction={(action) => handleAction(action, surfaceId)} isSubmitting={submittingData} />
            </div>
        );
    }

    // updateComponents / deleteSurface — Phase 0 では情報表示のみ
    return (
        <div style={{
            fontSize: '0.75rem',
            color: 'var(--accent-amber)',
            fontStyle: 'italic',
            padding: '0.75rem',
            border: '1px dashed var(--accent-amber-30)',
            borderRadius: 'var(--radius-sm)',
            background: 'var(--accent-amber-05)',
            margin: '0.5rem 0',
        }}>
            A2UI Operation: {envelope.type} (Phase 0 — dynamic update not yet supported)
        </div>
    );
};

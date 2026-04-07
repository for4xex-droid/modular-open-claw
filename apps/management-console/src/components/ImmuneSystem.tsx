/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import React, { useEffect, useState } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { Shield, AlertTriangle, CheckCircle, Search, Filter, Lock } from 'lucide-react';
import { API_BASE } from "../config";

import { ImmuneRule } from "../types";
import { authenticatedFetch } from "../lib/auth";
import { useTranslation } from '../i18n';

interface QuarantinedAsset {
    id: string;
    asset_name: string;
    image_hash: string;
    reason: string;
    status: string;
    uploaded_at?: string;
}

const ImmuneSystem: React.FC = () => {
    const { t } = useTranslation();
    const [rules, setRules] = useState<ImmuneRule[]>([]);
    const [quarantinedAssets, setQuarantinedAssets] = useState<QuarantinedAsset[]>([]);
    const [activeTab, setActiveTab] = useState<'RULES' | 'QUARANTINE'>('RULES');
    const [isAdding, setIsAdding] = useState(false);
    const [newRule, setNewRule] = useState({ pattern: '', severity: 50, action: 'BLOCK' });

    const fetchRules = async () => {
        try {
            const res = await authenticatedFetch(`${API_BASE}/api/synergy/rules`);
            if (res.ok) {
                const data: ImmuneRule[] = await res.json();

                const mapped = data.map(r => ({
                    ...r,
                    risk: r.severity > 80 ? "CRITICAL" : r.severity > 50 ? "HIGH" : "MEDIUM",
                    active: r.approval_status === "Approved" // Reflect actual status
                }));
                setRules(mapped);
            }
        } catch (e) {
            console.error("Failed to fetch immune rules", e);
        }
    };

    const fetchQuarantined = async () => {
        try {
            const res = await authenticatedFetch(`${API_BASE}/api/v1/audit/quarantine`);
            if (res.ok) {
                const data = await res.json();
                setQuarantinedAssets(data);
            }
        } catch (e) {
            console.error("Failed to fetch quarantined assets", e);
        }
    };

    useEffect(() => {
        fetchRules();
        fetchQuarantined();
    }, []);

    const handleAddRule = async () => {
        try {
            const res = await authenticatedFetch(`${API_BASE}/api/synergy/rules`, {
                method: 'POST',
                body: JSON.stringify({
                    id: '',
                    pattern: newRule.pattern,
                    severity: newRule.severity,
                    action: newRule.action,
                    created_at: '',
                })
            });
            if (res.ok) {
                setIsAdding(false);
                setNewRule({ pattern: '', severity: 50, action: 'BLOCK' });
                fetchRules();
            }
        } catch (e) {
            console.error("Failed to add rule", e);
        }
    };

    const [editingId, setEditingId] = useState<string | null>(null);

    const handleEditRule = (rule: ImmuneRule) => {
        setEditingId(rule.id);
        setNewRule({ pattern: rule.pattern, severity: rule.severity, action: rule.action });
        setIsAdding(true);
    };

    const handleUpdateRule = async () => {
        if (!editingId) return;
        try {
            const res = await authenticatedFetch(`${API_BASE}/api/synergy/rules`, {
                method: 'PUT',
                body: JSON.stringify({
                    id: editingId,
                    pattern: newRule.pattern,
                    severity: newRule.severity,
                    action: newRule.action,
                    created_at: rules.find(r => r.id === editingId)?.created_at || '',
                })
            });
            if (res.ok) {
                setIsAdding(false);
                setEditingId(null);
                setNewRule({ pattern: '', severity: 50, action: 'BLOCK' });
                fetchRules();
            }
        } catch (e) {
            console.error("Failed to update rule", e);
        }
    };

    const handleDeleteRule = async (id: string) => {
        if (!confirm("Are you sure you want to delete this immune rule?")) return;
        try {
            const res = await authenticatedFetch(`${API_BASE}/api/synergy/rules/${id}`, {
                method: 'DELETE'
            });
            if (res.ok) {
                fetchRules();
            }
        } catch (e) {
            console.error("Failed to delete rule", e);
        }
    };

    const handleReleaseQuarantine = async (id: string) => {
        if (!confirm("Are you sure you want to release this asset from quarantine?")) return;
        try {
            const res = await authenticatedFetch(`${API_BASE}/api/v1/audit/quarantine/${id}/release`, {
                method: 'POST'
            });
            if (res.ok) {
                fetchQuarantined();
            } else {
                alert("Failed to release asset: " + (await res.text()));
            }
        } catch (e) {
            console.error("Failed to release asset", e);
        }
    };

    return (
        <div className="main-panel ani-fade">
            <div className="panel-header">
                <div style={{ display: 'flex', alignItems: 'center', gap: '1rem' }}>
                    <Shield size={20} color="var(--accent-rose)" />
                    <h3>{t('immune.title')}</h3>
                </div>
                <div style={{ display: 'flex', gap: '1rem' }}>
                    <div style={{ display: 'flex', background: 'rgba(0,0,0,0.3)', borderRadius: 'var(--radius-md)', padding: '0.25rem' }}>
                        <button
                            onClick={() => setActiveTab('RULES')}
                            style={{
                                background: activeTab === 'RULES' ? 'var(--accent-cyan)' : 'transparent',
                                color: activeTab === 'RULES' ? 'var(--bg-primary)' : 'var(--text-muted)',
                                border: 'none', borderRadius: '4px', padding: '0.5rem 1rem', fontWeight: 700, cursor: 'pointer', fontSize: '0.8rem'
                            }}
                        >
                            RULES
                        </button>
                        <button
                            onClick={() => setActiveTab('QUARANTINE')}
                            style={{
                                background: activeTab === 'QUARANTINE' ? 'var(--accent-rose)' : 'transparent',
                                color: activeTab === 'QUARANTINE' ? 'var(--bg-primary)' : 'var(--text-muted)',
                                border: 'none', borderRadius: '4px', padding: '0.5rem 1rem', fontWeight: 700, cursor: 'pointer', fontSize: '0.8rem'
                            }}
                        >
                            QUARANTINE
                        </button>
                    </div>
                    {activeTab === 'RULES' && (
                        <button
                            onClick={() => {
                                setIsAdding(!isAdding);
                                if (isAdding) {
                                    setEditingId(null);
                                    setNewRule({ pattern: '', severity: 50, action: 'BLOCK' });
                                }
                            }}
                            className="nav-item"
                            style={{ margin: 0, padding: '0 1rem', background: isAdding ? 'var(--accent-rose)' : 'var(--accent-cyan)', color: 'var(--bg-primary)', fontWeight: 700 }}
                        >
                            {isAdding ? 'CANCEL' : 'FORGE NEW RULE'}
                        </button>
                    )}
                    <div className="status-badge">
                        <CheckCircle size={14} /> ACTIVE PROTECTIONS: {rules.length}
                    </div>
                </div>
            </div>

            <div style={{ padding: '2rem' }}>
                <AnimatePresence>
                    {isAdding && activeTab === 'RULES' && (
                        <motion.div
                            initial={{ height: 0, opacity: 0 }}
                            animate={{ height: 'auto', opacity: 1 }}
                            exit={{ height: 0, opacity: 0 }}
                            style={{ overflow: 'hidden', marginBottom: '2rem' }}
                        >
                            <div style={{ background: 'rgba(255,255,255,0.03)', border: `1px solid ${editingId ? 'var(--accent-amber)' : 'var(--accent-cyan)'}`, borderRadius: 'var(--radius-lg)', padding: '1.5rem', display: 'flex', flexWrap: 'wrap', gap: '1rem', alignItems: 'flex-end' }}>
                                <div style={{ flex: 2 }}>
                                    <label style={{ fontSize: '0.7rem', color: editingId ? 'var(--accent-amber)' : 'var(--accent-cyan)', display: 'block', marginBottom: '0.5rem' }}>PATTERN (REGEX OR TEXT)</label>
                                    <input
                                        value={newRule.pattern}
                                        onChange={e => setNewRule({ ...newRule, pattern: e.target.value })}
                                        placeholder="e.g. /etc/passwd"
                                        style={{ background: 'rgba(0,0,0,0.3)', border: '1px solid var(--border-glass)', borderRadius: 'var(--radius-md)', padding: '0.75rem', color: 'var(--text-primary)', width: '100%', outline: 'none' }}
                                    />
                                </div>
                                <div style={{ flex: 1 }}>
                                    <label style={{ fontSize: '0.7rem', color: editingId ? 'var(--accent-amber)' : 'var(--accent-cyan)', display: 'block', marginBottom: '0.5rem' }}>SEVERITY (0-100)</label>
                                    <input
                                        type="number"
                                        value={newRule.severity}
                                        onChange={e => setNewRule({ ...newRule, severity: parseInt(e.target.value) })}
                                        style={{ background: 'rgba(0,0,0,0.3)', border: '1px solid var(--border-glass)', borderRadius: 'var(--radius-md)', padding: '0.75rem', color: 'var(--text-primary)', width: '100%', outline: 'none' }}
                                    />
                                </div>
                                <div style={{ flex: 1 }}>
                                    <label style={{ fontSize: '0.7rem', color: editingId ? 'var(--accent-amber)' : 'var(--accent-cyan)', display: 'block', marginBottom: '0.5rem' }}>ACTION</label>
                                    <select
                                        value={newRule.action}
                                        onChange={e => setNewRule({ ...newRule, action: e.target.value })}
                                        style={{ background: 'rgba(0,0,0,0.3)', border: '1px solid var(--border-glass)', borderRadius: 'var(--radius-md)', padding: '0.75rem', color: 'var(--text-primary)', width: '100%', outline: 'none' }}
                                    >
                                        <option value="BLOCK">BLOCK</option>
                                        <option value="QUARANTINE">QUARANTINE</option>
                                        <option value="WARN">WARN</option>
                                    </select>
                                </div>
                                <button
                                    onClick={editingId ? handleUpdateRule : handleAddRule}
                                    style={{ background: editingId ? 'var(--accent-amber)' : 'var(--accent-cyan)', color: 'var(--bg-primary)', border: 'none', borderRadius: 'var(--radius-md)', padding: '0.75rem 1.5rem', fontWeight: 700, cursor: 'pointer' }}
                                >
                                    {editingId ? 'UPDATE RULE' : 'ACTIVATE RULE'}
                                </button>
                            </div>
                        </motion.div>
                    )}
                </AnimatePresence>

                <div style={{ display: 'flex', gap: '1rem', marginBottom: '2rem' }}>
                    <div style={{
                        flex: 1,
                        background: 'rgba(255,255,255,0.03)',
                        border: '1px solid var(--border-glass)',
                        borderRadius: 'var(--radius-md)',
                        padding: '0.75rem 1rem',
                        display: 'flex',
                        alignItems: 'center',
                        gap: '0.75rem'
                    }}>
                        <Search size={18} color="var(--text-muted)" />
                        <input
                            placeholder="Search active patterns..."
                            style={{ background: 'none', border: 'none', color: 'var(--text-primary)', outline: 'none', width: '100%', fontSize: '0.9rem' }}
                        />
                    </div>
                    <button className="nav-item" style={{ margin: 0, padding: '0 1rem' }}>
                        <Filter size={18} />
                    </button>
                </div>

                <div style={{ display: 'flex', flexDirection: 'column', gap: '1rem' }}>
                    <AnimatePresence>
                        {activeTab === 'RULES' ? rules.map((rule, i) => (
                            <motion.div
                                key={rule.id}
                                initial={{ opacity: 0, y: 10 }}
                                animate={{ opacity: 1, y: 0 }}
                                transition={{ delay: i * 0.1 }}
                                style={{
                                    background: 'var(--bg-glass-heavy)',
                                    border: editingId === rule.id ? '1px solid var(--accent-amber)' : '1px solid var(--border-glass)',
                                    borderRadius: 'var(--radius-lg)',
                                    padding: '1.5rem',
                                    display: 'flex',
                                    justifyContent: 'space-between',
                                    alignItems: 'center',
                                    boxShadow: '0 4px 15px rgba(0,0,0,0.2)'
                                }}
                            >
                                <div style={{ display: 'flex', gap: '1.5rem', alignItems: 'center' }}>
                                    <div style={{
                                        width: '48px',
                                        height: '48px',
                                        borderRadius: '12px',
                                        background: rule.risk === 'CRITICAL' ? 'rgba(255, 77, 148, 0.1)' : 'rgba(245, 158, 11, 0.1)',
                                        display: 'flex',
                                        alignItems: 'center',
                                        justifyContent: 'center',
                                        color: rule.risk === 'CRITICAL' ? 'var(--accent-rose)' : 'var(--accent-amber)'
                                    }}>
                                        <AlertTriangle size={24} />
                                    </div>
                                    <div>
                                        <div style={{ display: 'flex', gap: '0.75rem', alignItems: 'center', marginBottom: '0.4rem' }}>
                                            <code style={{
                                                fontSize: '1rem',
                                                fontWeight: 700,
                                                color: 'var(--text-primary)',
                                                background: 'rgba(0,0,0,0.3)',
                                                padding: '0.2rem 0.5rem',
                                                borderRadius: '4px'
                                            }}>
                                                {rule.pattern}
                                            </code>
                                            <span style={{
                                                fontSize: '0.7rem',
                                                fontWeight: 800,
                                                color: rule.risk === 'CRITICAL' ? 'var(--accent-rose)' : 'var(--accent-amber)',
                                                border: `1px solid currentColor`,
                                                padding: '1px 6px',
                                                borderRadius: '4px'
                                            }}>
                                                {rule.risk}
                                            </span>
                                        </div>
                                        <div style={{ fontSize: '0.85rem', color: 'var(--text-secondary)' }}>
                                            Action: <span style={{ color: 'var(--text-primary)', fontWeight: 600 }}>{rule.action}</span> • Status: <span style={{ color: rule.active ? 'var(--accent-emerald)' : 'var(--accent-amber)' }}>{rule.approval_status}</span>
                                        </div>
                                    </div>
                                </div>

                                <div style={{ display: 'flex', gap: '0.5rem' }}>
                                    <button
                                        onClick={() => handleEditRule(rule)}
                                        style={{
                                            background: 'rgba(255,255,255,0.05)',
                                            border: '1px solid var(--border-glass)',
                                            color: 'var(--text-primary)',
                                            padding: '0.5rem 1rem',
                                            borderRadius: '8px',
                                            fontSize: '0.8rem',
                                            cursor: 'pointer',
                                            fontWeight: 600
                                        }}
                                    >
                                        EDIT
                                    </button>
                                    <button
                                        onClick={() => handleDeleteRule(rule.id)}
                                        style={{
                                            background: 'rgba(255, 77, 148, 0.1)',
                                            border: '1px solid rgba(255, 77, 148, 0.2)',
                                            color: 'var(--accent-rose)',
                                            padding: '0.5rem 1rem',
                                            borderRadius: '8px',
                                            fontSize: '0.8rem',
                                            cursor: 'pointer',
                                            fontWeight: 600
                                        }}
                                    >
                                        DELETE
                                    </button>
                                </div>
                            </motion.div>
                        )) : quarantinedAssets.map((asset, i) => (
                            <motion.div
                                key={asset.id}
                                initial={{ opacity: 0, y: 10 }}
                                animate={{ opacity: 1, y: 0 }}
                                transition={{ delay: i * 0.1 }}
                                style={{
                                    background: 'var(--bg-glass-heavy)',
                                    border: '1px solid var(--accent-rose)',
                                    borderRadius: 'var(--radius-lg)',
                                    padding: '1.5rem',
                                    display: 'flex',
                                    justifyContent: 'space-between',
                                    alignItems: 'center',
                                    boxShadow: '0 4px 15px rgba(255, 77, 148, 0.1)'
                                }}
                            >
                                <div style={{ display: 'flex', gap: '1.5rem', alignItems: 'center' }}>
                                    <div style={{
                                        width: '48px',
                                        height: '48px',
                                        borderRadius: '12px',
                                        background: 'rgba(255, 77, 148, 0.1)',
                                        display: 'flex',
                                        alignItems: 'center',
                                        justifyContent: 'center',
                                        color: 'var(--accent-rose)'
                                    }}>
                                        <Lock size={24} />
                                    </div>
                                    <div>
                                        <div style={{ display: 'flex', gap: '0.75rem', alignItems: 'center', marginBottom: '0.4rem' }}>
                                            <span style={{ fontSize: '1rem', fontWeight: 700, color: 'var(--text-primary)' }}>
                                                {asset.asset_name}
                                            </span>
                                            <span style={{
                                                fontSize: '0.7rem',
                                                fontWeight: 800,
                                                color: 'var(--accent-rose)',
                                                border: `1px solid currentColor`,
                                                padding: '1px 6px',
                                                borderRadius: '4px'
                                            }}>
                                                QUARANTINED
                                            </span>
                                        </div>
                                        <div style={{ fontSize: '0.85rem', color: 'var(--text-secondary)' }}>
                                            Reason: <span style={{ color: 'var(--accent-amber)', fontWeight: 600 }}>{asset.reason}</span> • Hash: <span className="font-mono">{asset.image_hash}</span>
                                        </div>
                                    </div>
                                </div>

                                <div style={{ display: 'flex', gap: '0.5rem' }}>
                                    <button
                                        onClick={() => handleReleaseQuarantine(asset.id)}
                                        style={{
                                            background: 'var(--accent-cyan)',
                                            border: 'none',
                                            color: 'var(--bg-primary)',
                                            padding: '0.5rem 1rem',
                                            borderRadius: '8px',
                                            fontSize: '0.8rem',
                                            cursor: 'pointer',
                                            fontWeight: 700
                                        }}
                                    >
                                        RELEASE EXCEPTION
                                    </button>
                                </div>
                            </motion.div>
                        ))}
                    </AnimatePresence>
                </div>

                <div style={{ marginTop: '3rem', padding: '2rem', border: '1px dashed var(--border-glass)', borderRadius: 'var(--radius-xl)', textAlign: 'center' }}>
                    <Shield size={32} style={{ opacity: 0.2, marginBottom: '1rem' }} />
                    <h4 style={{ color: 'var(--text-secondary)' }}>{t('immune.heuristicsActive')}</h4>
                    <p style={{ fontSize: '0.85rem', color: 'var(--text-muted)', marginTop: '0.5rem' }}>
                        The Abyss Vault enforces these rules at the memory-page level. <br />
                        Unauthorized modifications to the sentinel state are physically impossible.
                    </p>
                </div>
            </div>
        </div>
    );
};

export default ImmuneSystem;

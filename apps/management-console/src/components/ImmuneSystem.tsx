/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import React, { useEffect, useState } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { Shield, AlertTriangle, Search, Filter, Lock, Plus, X } from 'lucide-react';
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
    const [editingId, setEditingId] = useState<string | null>(null);

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

    const inputStyle = {
        background: 'var(--black-30)',
        border: '1px solid var(--border-glass)',
        borderRadius: 'var(--radius-md)',
        padding: '0.75rem',
        color: 'var(--text-primary)',
        width: '100%',
        outline: 'none',
        fontSize: '0.9rem',
        transition: 'border-color var(--speed-normal)'
    };

    return (
        <div className="main-panel ani-fade" style={{ display: 'flex', flexDirection: 'column', height: '100%', overflow: 'hidden' }}>
            <div className="panel-header" style={{ padding: 'var(--space-md)', borderBottom: '1px solid var(--border-glass)', display: 'flex', justifyContent: 'space-between', alignItems: 'center', background: 'var(--bg-glass-light)' }}>
                <div style={{ display: 'flex', alignItems: 'center', gap: 'var(--space-sm)' }}>
                    <Shield size={20} color="var(--accent-rose)" />
                    <h3 style={{ margin: 0 }}>{t('immune.title')}</h3>
                </div>
                <div style={{ display: 'flex', gap: 'var(--space-md)', alignItems: 'center' }}>
                    <div style={{ display: 'flex', background: 'var(--black-30)', borderRadius: 'var(--radius-md)', padding: '0.2rem' }}>
                        <button
                            onClick={() => setActiveTab('RULES')}
                            style={{
                                background: activeTab === 'RULES' ? 'var(--accent-cyan)' : 'transparent',
                                color: activeTab === 'RULES' ? 'var(--bg-primary)' : 'var(--text-muted)',
                                border: 'none', borderRadius: '4px', padding: '0.4rem 1rem', fontWeight: 700, cursor: 'pointer', fontSize: '0.75rem', transition: 'all var(--speed-normal)'
                            }}
                        >
                            RULES
                        </button>
                        <button
                            onClick={() => setActiveTab('QUARANTINE')}
                            style={{
                                background: activeTab === 'QUARANTINE' ? 'var(--accent-rose)' : 'transparent',
                                color: activeTab === 'QUARANTINE' ? 'var(--bg-primary)' : 'var(--text-muted)',
                                border: 'none', borderRadius: '4px', padding: '0.4rem 1rem', fontWeight: 700, cursor: 'pointer', fontSize: '0.75rem', transition: 'all var(--speed-normal)'
                            }}
                        >
                            QUARANTINE
                        </button>
                    </div>
                </div>
            </div>

            <div style={{ flex: 1, overflowY: 'auto', padding: 'var(--space-lg)' }}>
                <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 'var(--space-lg)' }}>
                    <div style={{ display: 'flex', gap: 'var(--space-sm)', flex: 1, maxWidth: '600px' }}>
                        <div style={{
                            flex: 1,
                            background: 'var(--white-03)',
                            border: '1px solid var(--border-glass)',
                            borderRadius: 'var(--radius-md)',
                            padding: '0.6rem 1rem',
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
                        <button className="secondary-button" style={{ padding: '0.6rem' }}>
                            <Filter size={18} />
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
                            className="primary-button"
                            style={{ 
                                background: isAdding ? 'var(--accent-rose)' : 'var(--accent-cyan)', 
                                display: 'flex', alignItems: 'center', gap: '0.5rem'
                            }}
                        >
                            {isAdding ? <X size={18} /> : <Plus size={18} />}
                            {isAdding ? 'CANCEL' : 'FORGE NEW RULE'}
                        </button>
                    )}
                </div>

                <AnimatePresence>
                    {isAdding && activeTab === 'RULES' && (
                        <motion.div
                            initial={{ height: 0, opacity: 0 }}
                            animate={{ height: 'auto', opacity: 1 }}
                            exit={{ height: 0, opacity: 0 }}
                            style={{ overflow: 'hidden', marginBottom: '2rem' }}
                        >
                            <div style={{ background: 'var(--bg-glass-light)', border: `1px solid ${editingId ? 'var(--accent-amber)' : 'var(--accent-cyan)'}`, borderRadius: 'var(--radius-lg)', padding: '1.5rem', display: 'grid', gridTemplateColumns: '2fr 1fr 1fr auto', gap: '1rem', alignItems: 'flex-end' }}>
                                <div className="input-group">
                                    <label style={{ fontSize: '0.7rem', color: editingId ? 'var(--accent-amber)' : 'var(--accent-cyan)', display: 'block', marginBottom: '0.4rem', fontWeight: 700 }}>PATTERN (REGEX / TEXT)</label>
                                    <input
                                        value={newRule.pattern}
                                        onChange={e => setNewRule({ ...newRule, pattern: e.target.value })}
                                        placeholder="e.g. /etc/passwd"
                                        style={inputStyle}
                                    />
                                </div>
                                <div className="input-group">
                                    <label style={{ fontSize: '0.7rem', color: editingId ? 'var(--accent-amber)' : 'var(--accent-cyan)', display: 'block', marginBottom: '0.4rem', fontWeight: 700 }}>SEVERITY (0-100)</label>
                                    <input
                                        type="number"
                                        value={newRule.severity}
                                        onChange={e => setNewRule({ ...newRule, severity: parseInt(e.target.value) })}
                                        style={inputStyle}
                                    />
                                </div>
                                <div className="input-group">
                                    <label style={{ fontSize: '0.7rem', color: editingId ? 'var(--accent-amber)' : 'var(--accent-cyan)', display: 'block', marginBottom: '0.4rem', fontWeight: 700 }}>ACTION</label>
                                    <select
                                        value={newRule.action}
                                        onChange={e => setNewRule({ ...newRule, action: e.target.value })}
                                        style={inputStyle}
                                    >
                                        <option value="BLOCK" style={{ background: 'var(--bg-primary)' }}>BLOCK</option>
                                        <option value="QUARANTINE" style={{ background: 'var(--bg-primary)' }}>QUARANTINE</option>
                                        <option value="WARN" style={{ background: 'var(--bg-primary)' }}>WARN</option>
                                    </select>
                                </div>
                                <button
                                    onClick={editingId ? handleUpdateRule : handleAddRule}
                                    className="primary-button"
                                    style={{ background: editingId ? 'var(--accent-amber)' : 'var(--accent-cyan)', height: '44px' }}
                                >
                                    {editingId ? 'UPDATE RULE' : 'ACTIVATE RULE'}
                                </button>
                            </div>
                        </motion.div>
                    )}
                </AnimatePresence>

                <div style={{ display: 'flex', flexDirection: 'column', gap: 'var(--space-md)' }}>
                    <AnimatePresence>
                        {activeTab === 'RULES' ? (
                            rules.length > 0 ? rules.map((rule, i) => (
                            <motion.div
                                key={rule.id}
                                initial={{ opacity: 0, y: 10 }}
                                animate={{ opacity: 1, y: 0 }}
                                transition={{ delay: i * 0.05 }}
                                className="card-hover"
                                style={{
                                    background: 'var(--bg-glass-heavy)',
                                    border: editingId === rule.id ? '1px solid var(--accent-amber)' : '1px solid var(--border-glass)',
                                    borderRadius: 'var(--radius-md)',
                                    padding: 'var(--space-md)',
                                    display: 'flex',
                                    justifyContent: 'space-between',
                                    alignItems: 'center',
                                    boxShadow: 'var(--shadow-deep)',
                                    position: 'relative'
                                }}
                            >
                                <div style={{ display: 'flex', gap: 'var(--space-md)', alignItems: 'center' }}>
                                    <div style={{
                                        width: '42px',
                                        height: '42px',
                                        borderRadius: 'var(--radius-sm)',
                                        background: rule.risk === 'CRITICAL' ? 'var(--accent-rose-10)' : 'var(--accent-amber-10)',
                                        display: 'flex',
                                        alignItems: 'center',
                                        justifyContent: 'center',
                                        color: rule.risk === 'CRITICAL' ? 'var(--accent-rose)' : 'var(--accent-amber)'
                                    }}>
                                        <AlertTriangle size={20} />
                                    </div>
                                    <div>
                                        <div style={{ display: 'flex', gap: 'var(--space-sm)', alignItems: 'center', marginBottom: '0.4rem' }}>
                                            <code className="font-mono" style={{
                                                fontSize: '0.9rem',
                                                fontWeight: 700,
                                                color: 'var(--text-primary)',
                                                background: 'var(--black-20)',
                                                padding: '0.1rem 0.4rem',
                                                borderRadius: '4px'
                                            }}>
                                                {rule.pattern}
                                            </code>
                                            <span style={{
                                                fontSize: '0.65rem',
                                                fontWeight: 800,
                                                color: rule.risk === 'CRITICAL' ? 'var(--accent-rose)' : 'var(--accent-amber)',
                                                border: `1px solid currentColor`,
                                                padding: '1px 6px',
                                                borderRadius: '4px'
                                            }}>
                                                {rule.risk}
                                            </span>
                                        </div>
                                        <div style={{ fontSize: '0.75rem', color: 'var(--text-secondary)' }}>
                                            {t('immune.activeShields')}: <span style={{ color: 'var(--text-primary)', fontWeight: 600 }}>{rule.action}</span> • Status: <span style={{ color: rule.active ? 'var(--accent-emerald)' : 'var(--accent-amber)' }}>{rule.approval_status}</span>
                                        </div>
                                    </div>
                                </div>

                                <div style={{ display: 'flex', gap: 'var(--space-xs)' }}>
                                    <button
                                        onClick={() => handleEditRule(rule)}
                                        className="secondary-button"
                                        style={{ padding: '0.4rem 0.8rem', fontSize: '0.75rem' }}
                                    >
                                        EDIT
                                    </button>
                                    <button
                                        onClick={() => handleDeleteRule(rule.id)}
                                        className="card-hover"
                                        style={{ 
                                            background: 'var(--accent-rose-10)', 
                                            border: '1px solid var(--accent-rose-20)', 
                                            color: 'var(--accent-rose)',
                                            padding: '0.4rem 0.8rem',
                                            borderRadius: 'var(--radius-sm)',
                                            fontSize: '0.75rem',
                                            cursor: 'pointer',
                                            fontWeight: 700
                                        }}
                                    >
                                        DELETE
                                    </button>
                                </div>
                            </motion.div>
                        )) : (
                            <motion.div initial={{ opacity: 0 }} animate={{ opacity: 1 }} style={{ padding: 'var(--space-2xl)', textAlign: 'center', color: 'var(--text-muted)', background: 'var(--bg-glass)', borderRadius: 'var(--radius-lg)', border: '1px solid var(--border-glass)' }}>
                                <Shield size={48} style={{ opacity: 0.2, margin: '0 auto var(--space-md) auto', display: 'block' }} color="var(--accent-cyan)" />
                                <div style={{ fontWeight: 700, fontSize: '1.2rem', color: 'var(--text-primary)', marginBottom: 'var(--space-xs)' }}>{t('immune.noActiveRules')}</div>
                                <div style={{ fontSize: '0.9rem' }}>{t('immune.noActiveRulesDesc')}</div>
                            </motion.div>
                        )) : (
                            quarantinedAssets.length > 0 ? quarantinedAssets.map((asset, i) => (
                            <motion.div
                                key={asset.id}
                                initial={{ opacity: 0, y: 10 }}
                                animate={{ opacity: 1, y: 0 }}
                                transition={{ delay: i * 0.05 }}
                                className="card-hover"
                                style={{
                                    background: 'var(--bg-glass-heavy)',
                                    border: '1px solid var(--accent-rose-30)',
                                    borderRadius: 'var(--radius-md)',
                                    padding: 'var(--space-md)',
                                    display: 'flex',
                                    justifyContent: 'space-between',
                                    alignItems: 'center',
                                    boxShadow: 'var(--glow-rose)',
                                    position: 'relative'
                                }}
                            >
                                <div style={{ display: 'flex', gap: 'var(--space-md)', alignItems: 'center' }}>
                                    <div style={{
                                        width: '42px',
                                        height: '42px',
                                        borderRadius: 'var(--radius-sm)',
                                        background: 'var(--accent-rose-10)',
                                        display: 'flex',
                                        alignItems: 'center',
                                        justifyContent: 'center',
                                        color: 'var(--accent-rose)'
                                    }}>
                                        <Lock size={20} />
                                    </div>
                                    <div>
                                        <div style={{ display: 'flex', gap: 'var(--space-sm)', alignItems: 'center', marginBottom: '0.4rem' }}>
                                            <span style={{ fontSize: '1rem', fontWeight: 700, color: 'var(--text-primary)' }}>
                                                {asset.asset_name}
                                            </span>
                                            <span style={{
                                                fontSize: '0.65rem',
                                                fontWeight: 800,
                                                color: 'var(--accent-rose)',
                                                border: `1px solid currentColor`,
                                                padding: '1px 6px',
                                                borderRadius: '4px'
                                            }}>
                                                QUARANTINED
                                            </span>
                                        </div>
                                        <div style={{ fontSize: '0.75rem', color: 'var(--text-secondary)' }}>
                                            {t('immune.quarantine')}: <span style={{ color: 'var(--accent-amber)', fontWeight: 600 }}>{asset.reason}</span> • Hash: <span className="font-mono" style={{ fontSize: '0.7rem' }}>{asset.image_hash.substring(0, 16)}...</span>
                                        </div>
                                    </div>
                                </div>

                                <button
                                    onClick={() => handleReleaseQuarantine(asset.id)}
                                    className="primary-button"
                                    style={{ background: 'var(--accent-emerald)', padding: '0.5rem 1rem', fontSize: '0.75rem' }}
                                >
                                    RELEASE EXCEPTION
                                </button>
                            </motion.div>
                        )) : (
                            <motion.div initial={{ opacity: 0 }} animate={{ opacity: 1 }} style={{ padding: 'var(--space-2xl)', textAlign: 'center', color: 'var(--text-muted)', background: 'var(--bg-glass)', borderRadius: 'var(--radius-lg)', border: '1px solid var(--border-glass)' }}>
                                <Lock size={48} style={{ opacity: 0.2, margin: '0 auto var(--space-md) auto', display: 'block' }} color="var(--accent-rose)" />
                                <div style={{ fontWeight: 700, fontSize: '1.2rem', color: 'var(--text-primary)', marginBottom: 'var(--space-xs)' }}>{t('immune.quarantineClean')}</div>
                                <div style={{ fontSize: '0.9rem' }}>{t('immune.quarantineCleanDesc')}</div>
                            </motion.div>
                        ))}
                    </AnimatePresence>
                </div>

                <div className="info-box-glass" style={{ marginTop: '3rem', padding: '2rem', textAlign: 'center' }}>
                    <Shield size={32} style={{ opacity: 0.2, marginBottom: '1rem' }} />
                    <h4 style={{ color: 'var(--text-secondary)', margin: 0 }}>{t('immune.heuristicsActive')}</h4>
                    <p style={{ fontSize: '0.8rem', color: 'var(--text-muted)', marginTop: '0.5rem', lineHeight: 1.6 }}>
                        The Abyss Vault enforces these rules at the memory-page level. <br />
                        Unauthorized modifications to the sentinel state are physically impossible.
                    </p>
                </div>
            </div>

            <style>{`
                @media (max-width: 1024px) {
                    .panel-header {
                        flex-direction: column;
                        align-items: flex-start !important;
                        gap: 1rem;
                    }
                    .lora-view-container {
                        grid-template-columns: 1fr;
                    }
                }
            `}</style>
        </div>
    );
};

export default ImmuneSystem;

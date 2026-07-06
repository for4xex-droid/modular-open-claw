/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import React, { useEffect, useState, useCallback } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { Shield, AlertTriangle, Search, Filter, Lock, Plus, X, Activity } from 'lucide-react';
import { API_BASE } from "../config";

import { ImmuneRule, AegisStatusResponse } from "../types";
import { authenticatedFetch } from "../lib/auth";
import { useTranslation } from '../i18n';
import ConfirmModal from './common/ConfirmModal';
import { useToast } from './common/Toast';
import { LoadingState } from './ui/LoadingState';

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
    const [aegisStatus, setAegisStatus] = useState<AegisStatusResponse | null>(null);
    const [activeTab, setActiveTab] = useState<'RULES' | 'QUARANTINE' | 'AEGIS'>('RULES');
    const [isAdding, setIsAdding] = useState(false);
    const [newRule, setNewRule] = useState({ pattern: '', severity: 50, action: 'BLOCK' });
    const [editingId, setEditingId] = useState<string | null>(null);
    const { showToast } = useToast();
    const [searchTerm, setSearchTerm] = useState('');
    const [loading, setLoading] = useState(true);
    
    // Confirm Modals state
    const [deletingRuleId, setDeletingRuleId] = useState<string | null>(null);
    const [releasingAssetId, setReleasingAssetId] = useState<string | null>(null);

    const fetchRules = useCallback(async () => {
        try {
            const res = await authenticatedFetch(`${API_BASE}/api/synergy/rules`);
            if (res.ok) {
                const raw = await res.json();
                const data: ImmuneRule[] = Array.isArray(raw) ? raw : [];

                const mapped = data.map(r => ({
                    ...r,
                    risk: r.severity > 80 ? "CRITICAL" : r.severity > 50 ? "HIGH" : "MEDIUM",
                    active: r.approval_status === "Approved" // Reflect actual status
                }));
                setRules(mapped);
            } else {
                showToast('error', t('immune.fetchRulesFailed') || 'Failed to fetch immune rules');
            }
        } catch (e) {
            console.error("Failed to fetch immune rules", e);
            showToast('error', t('common.networkError') || 'Network error');
        }
    }, [showToast, t]);

    const fetchQuarantined = useCallback(async () => {
        try {
            const res = await authenticatedFetch(`${API_BASE}/api/v1/audit/quarantine`);
            if (res.ok) {
                const raw = await res.json();
                setQuarantinedAssets(Array.isArray(raw) ? raw : []);
            } else {
                showToast('error', t('immune.fetchQuarantineFailed') || 'Failed to fetch quarantined assets');
            }
        } catch (e) {
            console.error("Failed to fetch quarantined assets", e);
            showToast('error', t('common.networkError') || 'Network error');
        }
    }, [showToast, t]);

    const fetchAegisStatus = useCallback(async () => {
        try {
            const res = await authenticatedFetch(`${API_BASE}/api/v1/watchtower`);
            if (res.ok) {
                const data = await res.json();
                setAegisStatus(data);
            } else {
                showToast('error', t('immune.fetchAegisFailed') || 'Failed to fetch Aegis status');
            }
        } catch (e) {
            console.error("Failed to fetch aegis status", e);
            showToast('error', t('common.networkError') || 'Network error');
        }
    }, [showToast, t]);

    useEffect(() => {
        const loadAll = async () => {
            setLoading(true);
            try {
                await Promise.all([fetchRules(), fetchQuarantined(), fetchAegisStatus()]);
            } finally {
                setLoading(false);
            }
        };
        loadAll();
    }, [fetchRules, fetchQuarantined, fetchAegisStatus]);

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
                showToast('success', t('immune.ruleAdded') || 'Rule added successfully');
                fetchRules();
            } else {
                showToast('error', t('immune.ruleAddFailed') || 'Failed to add rule');
            }
        } catch (e) {
            console.error("Failed to add rule", e);
            showToast('error', t('common.networkError') || 'Network error');
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
                showToast('success', t('immune.ruleUpdated') || 'Rule updated successfully');
                fetchRules();
            } else {
                showToast('error', t('immune.ruleUpdateFailed') || 'Failed to update rule');
            }
        } catch (e) {
            console.error("Failed to update rule", e);
            showToast('error', t('common.networkError') || 'Network error');
        }
    };

    const executeDeleteRule = async () => {
        if (!deletingRuleId) return;
        try {
            const res = await authenticatedFetch(`${API_BASE}/api/synergy/rules/${deletingRuleId}`, {
                method: 'DELETE'
            });
            if (res.ok) {
                showToast('success', t('immune.ruleDeleted'));
                fetchRules();
            } else {
                showToast('error', t('immune.ruleDeleteFailed'));
            }
        } catch (e) {
            console.error("Failed to delete rule", e);
            showToast('error', t('common.networkError'));
        } finally {
            setDeletingRuleId(null);
        }
    };

    const handleDeleteRule = (id: string) => {
        setDeletingRuleId(id);
    };

    const executeReleaseQuarantine = async () => {
        if (!releasingAssetId) return;
        try {
            const res = await authenticatedFetch(`${API_BASE}/api/v1/audit/quarantine/${releasingAssetId}/release`, {
                method: 'POST'
            });
            if (res.ok) {
                showToast('success', t('immune.assetReleased'));
                fetchQuarantined();
            } else {
                showToast('error', t('immune.releaseFailed') + ': ' + (await res.text()));
            }
        } catch (e) {
            console.error("Failed to release asset", e);
            showToast('error', t('common.networkError'));
        } finally {
            setReleasingAssetId(null);
        }
    };

    const handleReleaseQuarantine = (id: string) => {
        setReleasingAssetId(id);
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
            <ConfirmModal
                isOpen={!!deletingRuleId}
                type="danger"
                title={t('immune.deleteRuleTitle')}
                message={t('immune.deleteRuleMessage')}
                details={t('immune.deleteRuleDetails')}
                confirmText={t('immune.confirmDelete')}
                onConfirm={executeDeleteRule}
                onCancel={() => setDeletingRuleId(null)}
            />
            <ConfirmModal
                isOpen={!!releasingAssetId}
                type="warning"
                title={t('immune.releaseAssetTitle')}
                message={t('immune.releaseAssetMessage')}
                details={t('immune.releaseAssetDetails')}
                confirmText={t('immune.confirmRelease')}
                onConfirm={executeReleaseQuarantine}
                onCancel={() => setReleasingAssetId(null)}
            />

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
                            {t('immune.tabRules')}
                        </button>
                        <button
                            onClick={() => setActiveTab('QUARANTINE')}
                            style={{
                                background: activeTab === 'QUARANTINE' ? 'var(--accent-rose)' : 'transparent',
                                color: activeTab === 'QUARANTINE' ? 'var(--bg-primary)' : 'var(--text-muted)',
                                border: 'none', borderRadius: '4px', padding: '0.4rem 1rem', fontWeight: 700, cursor: 'pointer', fontSize: '0.75rem', transition: 'all var(--speed-normal)'
                            }}
                        >
                            {t('immune.tabQuarantine')}
                        </button>
                        <button
                            onClick={() => setActiveTab('AEGIS')}
                            style={{
                                background: activeTab === 'AEGIS' ? 'var(--accent-amber)' : 'transparent',
                                color: activeTab === 'AEGIS' ? 'var(--bg-primary)' : 'var(--text-muted)',
                                border: 'none', borderRadius: '4px', padding: '0.4rem 1rem', fontWeight: 700, cursor: 'pointer', fontSize: '0.75rem', transition: 'all var(--speed-normal)'
                            }}
                        >
                            {t('immune.tabAegis')}
                        </button>
                    </div>
                </div>
            </div>

            <div style={{ flex: 1, overflowY: 'auto', padding: 'var(--space-lg)' }}>
                {loading ? (
                    <LoadingState messageKey="loading" />
                ) : (
                <>
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
                                placeholder={t('immune.searchPlaceholder')}
                                value={searchTerm}
                                onChange={e => setSearchTerm(e.target.value)}
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
                            {isAdding ? t('immune.cancel') : t('immune.forgeNewRule')}
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
                                    <label style={{ fontSize: '0.7rem', color: editingId ? 'var(--accent-amber)' : 'var(--accent-cyan)', display: 'block', marginBottom: '0.4rem', fontWeight: 700 }}>{t('immune.patternLabel')}</label>
                                    <input
                                        value={newRule.pattern}
                                        onChange={e => setNewRule({ ...newRule, pattern: e.target.value })}
                                        placeholder="e.g. /etc/passwd"
                                        style={inputStyle}
                                    />
                                </div>
                                <div className="input-group">
                                    <label style={{ fontSize: '0.7rem', color: editingId ? 'var(--accent-amber)' : 'var(--accent-cyan)', display: 'block', marginBottom: '0.4rem', fontWeight: 700 }}>{t('immune.severityLabel')}</label>
                                    <input
                                        type="number"
                                        value={newRule.severity}
                                        onChange={e => setNewRule({ ...newRule, severity: Math.max(1, Math.min(100, parseInt(e.target.value) || 1)) })}
                                        style={inputStyle}
                                    />
                                </div>
                                <div className="input-group">
                                    <label style={{ fontSize: '0.7rem', color: editingId ? 'var(--accent-amber)' : 'var(--accent-cyan)', display: 'block', marginBottom: '0.4rem', fontWeight: 700 }}>{t('immune.actionLabel')}</label>
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
                                    {editingId ? t('immune.updateRule') : t('immune.activateRule')}
                                </button>
                            </div>
                        </motion.div>
                    )}
                </AnimatePresence>

                <div style={{ display: 'flex', flexDirection: 'column', gap: 'var(--space-md)' }}>
                    <AnimatePresence>
                        {activeTab === 'RULES' ? (
                            rules.length > 0 ? rules.filter(r => !searchTerm || r.pattern.toLowerCase().includes(searchTerm.toLowerCase()) || r.action.toLowerCase().includes(searchTerm.toLowerCase())).map((rule, i) => (
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
                                        {t('immune.editButton')}
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
                                        {t('immune.deleteButton')}
                                    </button>
                                </div>
                            </motion.div>
                        )) : (
                            <motion.div initial={{ opacity: 0 }} animate={{ opacity: 1 }} style={{ padding: 'var(--space-2xl)', textAlign: 'center', color: 'var(--text-muted)', background: 'var(--bg-glass)', borderRadius: 'var(--radius-lg)', border: '1px solid var(--border-glass)' }}>
                                <Shield size={48} style={{ opacity: 0.2, margin: '0 auto var(--space-md) auto', display: 'block' }} color="var(--accent-cyan)" />
                                <div style={{ fontWeight: 700, fontSize: '1.2rem', color: 'var(--text-primary)', marginBottom: 'var(--space-xs)' }}>{t('immune.noActiveRules')}</div>
                                <div style={{ fontSize: '0.9rem' }}>{t('immune.noActiveRulesDesc')}</div>
                            </motion.div>
                        )) : activeTab === 'QUARANTINE' ? (
                            quarantinedAssets.length > 0 ? quarantinedAssets.filter(a => !searchTerm || a.asset_name.toLowerCase().includes(searchTerm.toLowerCase()) || a.reason.toLowerCase().includes(searchTerm.toLowerCase())).map((asset, i) => (
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
                                    {t('immune.releaseException')}
                                </button>
                            </motion.div>
                        )) : (
                            <motion.div initial={{ opacity: 0 }} animate={{ opacity: 1 }} style={{ padding: 'var(--space-2xl)', textAlign: 'center', color: 'var(--text-muted)', background: 'var(--bg-glass)', borderRadius: 'var(--radius-lg)', border: '1px solid var(--border-glass)' }}>
                                <Lock size={48} style={{ opacity: 0.2, margin: '0 auto var(--space-md) auto', display: 'block' }} color="var(--accent-rose)" />
                                <div style={{ fontWeight: 700, fontSize: '1.2rem', color: 'var(--text-primary)', marginBottom: 'var(--space-xs)' }}>{t('immune.quarantineClean')}</div>
                                <div style={{ fontSize: '0.9rem' }}>{t('immune.quarantineCleanDesc')}</div>
                            </motion.div>
                        )) : (
                            <div style={{ display: 'flex', flexDirection: 'column', gap: 'var(--space-lg)', width: '100%' }}>
                                <div style={{
                                    background: 'var(--black-20)',
                                    borderRadius: 'var(--radius-lg)',
                                    border: '1px solid var(--border-glass)',
                                    padding: 'var(--space-lg)',
                                    display: 'grid',
                                    gridTemplateColumns: 'repeat(4, 1fr)',
                                    gap: 'var(--space-md)'
                                }}>
                                    <div style={{ textAlign: 'center' }}>
                                        <div style={{ fontSize: '0.75rem', color: 'var(--text-muted)', fontWeight: 700, marginBottom: '0.5rem' }}>{t('immune.totalIncidents7d')}</div>
                                        <div style={{ fontSize: '2rem', fontWeight: 800, color: 'var(--accent-cyan)' }}>{aegisStatus?.stats?.total_incidents_7d || 0}</div>
                                    </div>
                                    <div style={{ textAlign: 'center', borderLeft: '1px solid var(--border-glass)' }}>
                                        <div style={{ fontSize: '0.75rem', color: 'var(--text-muted)', fontWeight: 700, marginBottom: '0.5rem' }}>{t('immune.unresolved')}</div>
                                        <div style={{ fontSize: '2rem', fontWeight: 800, color: aegisStatus?.stats?.unresolved && aegisStatus.stats.unresolved > 0 ? 'var(--accent-rose)' : 'var(--accent-emerald)' }}>
                                            {aegisStatus?.stats?.unresolved || 0}
                                        </div>
                                    </div>
                                    <div style={{ textAlign: 'center', borderLeft: '1px solid var(--border-glass)' }}>
                                        <div style={{ fontSize: '0.75rem', color: 'var(--text-muted)', fontWeight: 700, marginBottom: '0.5rem' }}>{t('immune.affectedSkills')}</div>
                                        <div style={{ fontSize: '2rem', fontWeight: 800, color: 'var(--accent-amber)' }}>{aegisStatus?.stats?.distinct_skills || 0}</div>
                                    </div>
                                    <div style={{ textAlign: 'center', borderLeft: '1px solid var(--border-glass)' }}>
                                        <div style={{ fontSize: '0.75rem', color: 'var(--text-muted)', fontWeight: 700, marginBottom: '0.5rem' }}>{t('immune.topFailingSkill')}</div>
                                        <div style={{ fontSize: '1rem', fontWeight: 700, color: 'var(--text-primary)', marginTop: '0.8rem' }}>{aegisStatus?.stats?.top_failing_skill || 'None'}</div>
                                    </div>
                                </div>

                                <h4 style={{ margin: 'var(--space-md) 0 0 0', color: 'var(--text-primary)', borderBottom: '1px solid var(--border-glass)', paddingBottom: '0.5rem' }}>{t('immune.openIncidents')}</h4>
                                
                                {aegisStatus && Array.isArray(aegisStatus.open_incidents) && aegisStatus.open_incidents.length > 0 ? aegisStatus.open_incidents.map((incident, i) => (
                                    <motion.div
                                        key={incident.id}
                                        initial={{ opacity: 0, y: 10 }}
                                        animate={{ opacity: 1, y: 0 }}
                                        transition={{ delay: i * 0.05 }}
                                        className="card-hover"
                                        style={{
                                            background: 'var(--bg-glass-heavy)',
                                            border: '1px solid var(--accent-amber-30)',
                                            borderRadius: 'var(--radius-md)',
                                            padding: 'var(--space-md)',
                                            display: 'flex',
                                            justifyContent: 'space-between',
                                            alignItems: 'center',
                                            boxShadow: 'var(--glow-amber)',
                                            position: 'relative'
                                        }}
                                    >
                                        <div style={{ display: 'flex', gap: 'var(--space-md)', alignItems: 'center' }}>
                                            <div style={{
                                                width: '42px',
                                                height: '42px',
                                                borderRadius: 'var(--radius-sm)',
                                                background: 'var(--accent-amber-10)',
                                                display: 'flex',
                                                alignItems: 'center',
                                                justifyContent: 'center',
                                                color: 'var(--accent-amber)'
                                            }}>
                                                <Activity size={20} />
                                            </div>
                                            <div>
                                                <div style={{ display: 'flex', gap: 'var(--space-sm)', alignItems: 'center', marginBottom: '0.4rem' }}>
                                                    <span style={{ fontSize: '1rem', fontWeight: 700, color: 'var(--text-primary)' }}>
                                                        {incident.skill_name}
                                                    </span>
                                                    <span style={{
                                                        fontSize: '0.65rem',
                                                        fontWeight: 800,
                                                        color: 'var(--bg-primary)',
                                                        background: incident.status === 'Open' ? 'var(--accent-rose)' : 'var(--accent-amber)',
                                                        padding: '2px 6px',
                                                        borderRadius: '4px'
                                                    }}>
                                                        {incident.status.toUpperCase()}
                                                    </span>
                                                </div>
                                                <div style={{ fontSize: '0.75rem', color: 'var(--text-secondary)' }}>
                                                    <span className="font-mono" style={{ background: 'var(--black-30)', padding: '2px 4px', borderRadius: '4px' }}>
                                                        {(incident.input_payload || '').substring(0, 40)}{(incident.input_payload || '').length > 40 ? '...' : ''}
                                                    </span>
                                                    <span style={{ marginLeft: '1rem', opacity: 0.6 }}>Reported: {new Date(incident.created_at).toLocaleString()}</span>
                                                </div>
                                            </div>
                                        </div>
                                    </motion.div>
                                )) : (
                                    <motion.div initial={{ opacity: 0 }} animate={{ opacity: 1 }} style={{ padding: 'var(--space-2xl)', textAlign: 'center', color: 'var(--text-muted)', background: 'var(--bg-glass)', borderRadius: 'var(--radius-lg)', border: '1px solid var(--border-glass)' }}>
                                        <Shield size={48} style={{ opacity: 0.2, margin: '0 auto var(--space-md) auto', display: 'block' }} color="var(--accent-emerald)" />
                                        <div style={{ fontWeight: 700, fontSize: '1.2rem', color: 'var(--text-primary)', marginBottom: 'var(--space-xs)' }}>{t('immune.zeroIncidents')}</div>
                                        <div style={{ fontSize: '0.9rem' }}>{t('immune.zeroIncidentsDesc')}</div>
                                    </motion.div>
                                )}
                            </div>
                        )}
                    </AnimatePresence>
                </div>

                <div className="info-box-glass" style={{ marginTop: '3rem', padding: '2rem', textAlign: 'center' }}>
                    <Shield size={32} style={{ opacity: 0.2, marginBottom: '1rem' }} />
                    <h4 style={{ color: 'var(--text-secondary)', margin: 0 }}>{t('immune.abyssVaultTitle')}</h4>
                    <p style={{ fontSize: '0.8rem', color: 'var(--text-muted)', marginTop: '0.5rem', lineHeight: 1.6 }}>
                        {t('immune.abyssVaultDesc')}
                    </p>
                </div>
                </>
                )}
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

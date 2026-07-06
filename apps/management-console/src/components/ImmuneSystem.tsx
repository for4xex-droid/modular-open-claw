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
import { StatCard } from './ui/StatCard';
import { SectionHeader } from './ui/SectionHeader';

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

    const unresolvedCount = aegisStatus?.stats?.unresolved || 0;

    return (
        <div className="main-panel ani-fade immune-panel">
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

            <div className="panel-header">
                <div className="immune-header-title">
                    <Shield size={20} color="var(--accent-rose)" />
                    <h3>{t('immune.title')}</h3>
                </div>
                <div className="immune-header-actions">
                    <div className="immune-tab-bar">
                        <button
                            onClick={() => setActiveTab('RULES')}
                            className={`immune-tab-btn immune-tab-btn--rules${activeTab === 'RULES' ? ' immune-tab-btn--active' : ''}`}
                        >
                            {t('immune.tabRules')}
                        </button>
                        <button
                            onClick={() => setActiveTab('QUARANTINE')}
                            className={`immune-tab-btn immune-tab-btn--quarantine${activeTab === 'QUARANTINE' ? ' immune-tab-btn--active' : ''}`}
                        >
                            {t('immune.tabQuarantine')}
                        </button>
                        <button
                            onClick={() => setActiveTab('AEGIS')}
                            className={`immune-tab-btn immune-tab-btn--aegis${activeTab === 'AEGIS' ? ' immune-tab-btn--active' : ''}`}
                        >
                            {t('immune.tabAegis')}
                        </button>
                    </div>
                </div>
            </div>

            <div className="immune-content">
                {loading ? (
                    <LoadingState messageKey="loading" />
                ) : (
                <>
                <div className="immune-toolbar">
                    <div className="immune-search-row">
                        <div className="immune-search-box">
                            <Search size={18} color="var(--text-muted)" />
                            <input
                                className="immune-search-input"
                                placeholder={t('immune.searchPlaceholder')}
                                value={searchTerm}
                                onChange={e => setSearchTerm(e.target.value)}
                            />
                        </div>
                        <button className="secondary-button immune-filter-btn">
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
                            className={`primary-button immune-forge-btn${isAdding ? ' immune-forge-btn--cancel' : ''}`}
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
                            className="immune-form-collapse"
                        >
                            <div className={`immune-rule-form${editingId ? ' immune-rule-form--edit' : ''}`}>
                                <div className="input-group">
                                    <label className={`immune-form-label${editingId ? ' immune-form-label--edit' : ''}`}>{t('immune.patternLabel')}</label>
                                    <input
                                        className="ui-input"
                                        value={newRule.pattern}
                                        onChange={e => setNewRule({ ...newRule, pattern: e.target.value })}
                                        placeholder="e.g. /etc/passwd"
                                    />
                                </div>
                                <div className="input-group">
                                    <label className={`immune-form-label${editingId ? ' immune-form-label--edit' : ''}`}>{t('immune.severityLabel')}</label>
                                    <input
                                        className="ui-input"
                                        type="number"
                                        value={newRule.severity}
                                        onChange={e => setNewRule({ ...newRule, severity: Math.max(1, Math.min(100, parseInt(e.target.value) || 1)) })}
                                    />
                                </div>
                                <div className="input-group">
                                    <label className={`immune-form-label${editingId ? ' immune-form-label--edit' : ''}`}>{t('immune.actionLabel')}</label>
                                    <select
                                        className="ui-select"
                                        value={newRule.action}
                                        onChange={e => setNewRule({ ...newRule, action: e.target.value })}
                                    >
                                        <option value="BLOCK">BLOCK</option>
                                        <option value="QUARANTINE">QUARANTINE</option>
                                        <option value="WARN">WARN</option>
                                    </select>
                                </div>
                                <button
                                    onClick={editingId ? handleUpdateRule : handleAddRule}
                                    className={`primary-button immune-submit-btn${editingId ? ' immune-submit-btn--edit' : ''}`}
                                >
                                    {editingId ? t('immune.updateRule') : t('immune.activateRule')}
                                </button>
                            </div>
                        </motion.div>
                    )}
                </AnimatePresence>

                <div className="immune-list">
                    <AnimatePresence>
                        {activeTab === 'RULES' ? (
                            rules.length > 0 ? rules.filter(r => !searchTerm || r.pattern.toLowerCase().includes(searchTerm.toLowerCase()) || r.action.toLowerCase().includes(searchTerm.toLowerCase())).map((rule, i) => (
                            <motion.div
                                key={rule.id}
                                initial={{ opacity: 0, y: 10 }}
                                animate={{ opacity: 1, y: 0 }}
                                transition={{ delay: i * 0.05 }}
                                className={`card-hover immune-threat-card${editingId === rule.id ? ' immune-threat-card--editing' : ''}`}
                            >
                                <div className="immune-threat-row">
                                    <div className={`immune-threat-icon ${rule.risk === 'CRITICAL' ? 'immune-threat-icon--critical' : 'immune-threat-icon--high'}`}>
                                        <AlertTriangle size={20} />
                                    </div>
                                    <div>
                                        <div className="immune-threat-title-row">
                                            <code className="font-mono immune-pattern-code">
                                                {rule.pattern}
                                            </code>
                                            <span className={`immune-risk-badge ${rule.risk === 'CRITICAL' ? 'immune-risk-badge--critical' : 'immune-risk-badge--high'}`}>
                                                {rule.risk}
                                            </span>
                                        </div>
                                        <div className="immune-meta-text">
                                            {t('immune.activeShields')}: <span className="immune-meta-emphasis">{rule.action}</span> • Status: <span className={rule.active ? 'immune-status--active' : 'immune-status--inactive'}>{rule.approval_status}</span>
                                        </div>
                                    </div>
                                </div>

                                <div className="immune-actions">
                                    <button
                                        onClick={() => handleEditRule(rule)}
                                        className="secondary-button immune-edit-btn"
                                    >
                                        {t('immune.editButton')}
                                    </button>
                                    <button
                                        onClick={() => handleDeleteRule(rule.id)}
                                        className="card-hover immune-delete-btn"
                                    >
                                        {t('immune.deleteButton')}
                                    </button>
                                </div>
                            </motion.div>
                        )) : (
                            <motion.div initial={{ opacity: 0 }} animate={{ opacity: 1 }} className="immune-empty-state">
                                <Shield size={48} className="immune-empty-icon" color="var(--accent-cyan)" />
                                <div className="immune-empty-title">{t('immune.noActiveRules')}</div>
                                <div className="immune-empty-desc">{t('immune.noActiveRulesDesc')}</div>
                            </motion.div>
                        )) : activeTab === 'QUARANTINE' ? (
                            quarantinedAssets.length > 0 ? quarantinedAssets.filter(a => !searchTerm || a.asset_name.toLowerCase().includes(searchTerm.toLowerCase()) || a.reason.toLowerCase().includes(searchTerm.toLowerCase())).map((asset, i) => (
                            <motion.div
                                key={asset.id}
                                initial={{ opacity: 0, y: 10 }}
                                animate={{ opacity: 1, y: 0 }}
                                transition={{ delay: i * 0.05 }}
                                className="card-hover immune-threat-card immune-threat-card--quarantine"
                            >
                                <div className="immune-threat-row">
                                    <div className="immune-threat-icon immune-threat-icon--rose">
                                        <Lock size={20} />
                                    </div>
                                    <div>
                                        <div className="immune-threat-title-row">
                                            <span className="immune-item-title">
                                                {asset.asset_name}
                                            </span>
                                            <span className="immune-risk-badge immune-risk-badge--quarantine">
                                                QUARANTINED
                                            </span>
                                        </div>
                                        <div className="immune-meta-text">
                                            {t('immune.quarantine')}: <span className="immune-meta-reason">{asset.reason}</span> • Hash: <span className="font-mono immune-meta-hash">{asset.image_hash.substring(0, 16)}...</span>
                                        </div>
                                    </div>
                                </div>

                                <button
                                    onClick={() => handleReleaseQuarantine(asset.id)}
                                    className="primary-button immune-release-btn"
                                >
                                    {t('immune.releaseException')}
                                </button>
                            </motion.div>
                        )) : (
                            <motion.div initial={{ opacity: 0 }} animate={{ opacity: 1 }} className="immune-empty-state">
                                <Lock size={48} className="immune-empty-icon" color="var(--accent-rose)" />
                                <div className="immune-empty-title">{t('immune.quarantineClean')}</div>
                                <div className="immune-empty-desc">{t('immune.quarantineCleanDesc')}</div>
                            </motion.div>
                        )) : (
                            <div className="immune-aegis-section">
                                <div className="grid-stats immune-aegis-stats">
                                    <StatCard
                                        label={t('immune.totalIncidents7d')}
                                        value={<span className="immune-stat-value--cyan">{aegisStatus?.stats?.total_incidents_7d || 0}</span>}
                                    />
                                    <StatCard
                                        label={t('immune.unresolved')}
                                        value={
                                            <span className={unresolvedCount > 0 ? 'immune-stat-value--rose' : 'immune-stat-value--emerald'}>
                                                {unresolvedCount}
                                            </span>
                                        }
                                    />
                                    <StatCard
                                        label={t('immune.affectedSkills')}
                                        value={<span className="immune-stat-value--amber">{aegisStatus?.stats?.distinct_skills || 0}</span>}
                                    />
                                    <StatCard
                                        label={t('immune.topFailingSkill')}
                                        className="immune-stat-card--compact"
                                        value={aegisStatus?.stats?.top_failing_skill || 'None'}
                                    />
                                </div>

                                <SectionHeader title={t('immune.openIncidents')} className="immune-section-title" />
                                
                                {aegisStatus && Array.isArray(aegisStatus.open_incidents) && aegisStatus.open_incidents.length > 0 ? aegisStatus.open_incidents.map((incident, i) => (
                                    <motion.div
                                        key={incident.id}
                                        initial={{ opacity: 0, y: 10 }}
                                        animate={{ opacity: 1, y: 0 }}
                                        transition={{ delay: i * 0.05 }}
                                        className="card-hover immune-threat-card immune-threat-card--incident"
                                    >
                                        <div className="immune-threat-row">
                                            <div className="immune-threat-icon immune-threat-icon--amber">
                                                <Activity size={20} />
                                            </div>
                                            <div>
                                                <div className="immune-threat-title-row">
                                                    <span className="immune-item-title">
                                                        {incident.skill_name}
                                                    </span>
                                                    <span className={`immune-risk-badge immune-risk-badge--status ${incident.status === 'Open' ? 'immune-risk-badge--open' : 'immune-risk-badge--closed'}`}>
                                                        {incident.status.toUpperCase()}
                                                    </span>
                                                </div>
                                                <div className="immune-meta-text">
                                                    <span className="font-mono immune-payload-chip">
                                                        {(incident.input_payload || '').substring(0, 40)}{(incident.input_payload || '').length > 40 ? '...' : ''}
                                                    </span>
                                                    <span className="immune-meta-timestamp">Reported: {new Date(incident.created_at).toLocaleString()}</span>
                                                </div>
                                            </div>
                                        </div>
                                    </motion.div>
                                )) : (
                                    <motion.div initial={{ opacity: 0 }} animate={{ opacity: 1 }} className="immune-empty-state">
                                        <Shield size={48} className="immune-empty-icon" color="var(--accent-emerald)" />
                                        <div className="immune-empty-title">{t('immune.zeroIncidents')}</div>
                                        <div className="immune-empty-desc">{t('immune.zeroIncidentsDesc')}</div>
                                    </motion.div>
                                )}
                            </div>
                        )}
                    </AnimatePresence>
                </div>

                <div className="info-box-glass immune-vault-box">
                    <Shield size={32} className="immune-vault-icon" />
                    <h4 className="immune-vault-title">{t('immune.abyssVaultTitle')}</h4>
                    <p className="immune-vault-desc">
                        {t('immune.abyssVaultDesc')}
                    </p>
                </div>
                </>
                )}
            </div>
        </div>
    );
};

export default ImmuneSystem;

/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import React, { useState, useEffect } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import {
    Box,
    Search,
    Download,
    Play,
    Settings,
    ShieldCheck,
    Cpu,
    Cloud,
    Lock,
    Terminal
} from 'lucide-react';
import { API_BASE } from '../config';
import { authenticatedFetch } from '../lib/auth';
import { useTranslation } from '../i18n';
import { useToast } from './common/Toast';

interface Skill {
    name: string;
    description: string;
    source: 'wasm' | 'mcp' | 'marketplace';
    status: 'Active' | 'Installed' | 'Available';
    layer: number;
    tools: string[];
}

const SkillVault: React.FC = () => {
    const { t } = useTranslation();
    const { showToast } = useToast();
    const [skills, setSkills] = useState<Skill[]>([]);
    const [loading, setLoading] = useState(true);
    const [filter, setFilter] = useState<'all' | 'my' | 'market'>('all');
    const [searchTerm, setSearchTerm] = useState('');

    useEffect(() => {
        fetchSkills();
    }, []);

    const fetchSkills = async () => {
        setLoading(true);
        try {
            const res = await authenticatedFetch(`${API_BASE}/api/skills`);
            if (res.ok) {
                const data = await res.json();
                if (Array.isArray(data)) {
                    setSkills(data);
                } else {
                    console.error("Unexpected skills response format:", typeof data);
                    setSkills([]);
                }
            } else {
                showToast('error', t('skill.fetchFailed', { defaultValue: 'Failed to load skills.' }));
            }
        } catch (error) {
            console.error("Failed to fetch skills:", error);
            showToast('error', t('skill.fetchNetworkError', { defaultValue: 'Network error while loading skills.' }));
        } finally {
            setLoading(false);
        }
    };

    const filteredSkills = skills.filter(skill => {
        const matchesSearch = skill.name.toLowerCase().includes(searchTerm.toLowerCase()) ||
            skill.description.toLowerCase().includes(searchTerm.toLowerCase());

        if (filter === 'my') return matchesSearch && (skill.source === 'wasm' || skill.source === 'mcp');
        if (filter === 'market') return matchesSearch && skill.source === 'marketplace';
        return matchesSearch;
    });

    return (
        <div className="skill-vault ani-fade" style={{ display: 'grid', gridTemplateColumns: '240px 1fr', gap: 'var(--space-md)', height: 'calc(100vh - 180px)' }}>
            {/* Sidebar Filters */}
            <div className="main-panel" style={{ padding: '1.5rem', display: 'flex', flexDirection: 'column', gap: '1.5rem' }}>
                <div>
                    <h4 style={{ fontSize: '0.75rem', color: 'var(--text-muted)', marginBottom: '1rem', letterSpacing: '0.1em' }}>{t('skill.categories', { defaultValue: 'LIBRARY CATEGORIES' })}</h4>
                    <div style={{ display: 'flex', flexDirection: 'column', gap: '0.5rem' }}>
                        <FilterButton active={filter === 'all'} onClick={() => setFilter('all')} icon={<Box size={18} />} label={t('skill.all')} />
                        <FilterButton active={filter === 'my'} onClick={() => setFilter('my')} icon={<Cpu size={18} />} label={t('skill.active', { defaultValue: 'Active Skills' })} />
                        <FilterButton active={filter === 'market'} onClick={() => setFilter('market')} icon={<Cloud size={18} />} label={t('skill.marketplace')} />
                    </div>
                </div>

                <div style={{ marginTop: 'auto', padding: '1rem', background: 'var(--accent-cyan-05)', borderRadius: '12px', border: '1px solid var(--accent-cyan)' }}>
                    <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem', color: 'var(--accent-cyan)', marginBottom: '0.5rem' }}>
                        <ShieldCheck size={16} />
                        <span style={{ fontSize: '0.8rem', fontWeight: 700 }}>{t('skill.verified')}</span>
                    </div>
                    <p style={{ fontSize: '0.7rem', color: 'var(--text-secondary)', lineHeight: 1.4 }}>
                        {t('skill.verifiedDescription', { defaultValue: 'All WASM skills are mathematically verified for memory safety before execution.' })}
                    </p>
                </div>
            </div>

            {/* Main Listing */}
            <div style={{ display: 'flex', flexDirection: 'column', gap: 'var(--space-md)', overflow: 'hidden' }}>
                <div className="main-panel" style={{ padding: '1rem 1.5rem', display: 'flex', alignItems: 'center', gap: '1rem' }}>
                    <Search size={20} color="var(--text-muted)" />
                    <input
                        type="text"
                        placeholder={t('skill.search')}
                        value={searchTerm}
                        onChange={(e) => setSearchTerm(e.target.value)}
                        aria-label={t('skill.searchAriaLabel', { defaultValue: 'Search skills' })}
                        style={{ background: 'none', border: 'none', outline: 'none', color: 'var(--text-primary)', flex: 1, fontSize: '0.95rem' }}
                    />
                    <button onClick={fetchSkills} aria-label={t('common.refreshAriaLabel', { defaultValue: 'Refresh skill list' })} style={{ background: 'none', border: 'none', color: 'var(--accent-cyan)', cursor: 'pointer', fontSize: '0.85rem' }}>
                        {t('common.refresh', { defaultValue: 'Refresh' })}
                    </button>
                </div>

                <div style={{ flex: 1, overflowY: 'auto', display: 'grid', gridTemplateColumns: 'repeat(auto-fill, minmax(340px, 1fr))', gap: '1rem', paddingBottom: '2rem' }}>
                    {loading ? (
                        <div style={{ gridColumn: '1/-1', textAlign: 'center', padding: '4rem', color: 'var(--text-muted)' }}>
                            <motion.div animate={{ rotate: 360 }} transition={{ duration: 2, repeat: Infinity, ease: 'linear' }}>
                                <Settings size={40} />
                            </motion.div>
                            <p style={{ marginTop: '1rem' }}>{t('skill.loading')}</p>
                        </div>
                    ) : filteredSkills.length === 0 ? (
                        <div style={{ gridColumn: '1/-1', textAlign: 'center', padding: '4rem', color: 'var(--text-muted)' }}>
                            {t('skill.noResults', { defaultValue: 'No skillsets found matching your filters.' })}
                        </div>
                    ) : (
                        <AnimatePresence>
                            {filteredSkills.map((skill, i) => (
                                <SkillCard key={skill.name} skill={skill} index={i} onInstalled={(name) => {
                                    setSkills(prev => prev.map(s => s.name === name ? { ...s, status: 'Installed' } : s));
                                }} />
                            ))}
                        </AnimatePresence>
                    )}
                </div>
            </div>
        </div>
    );
};

const FilterButton: React.FC<{ active: boolean, onClick: () => void, icon: React.ReactNode, label: string }> = ({ active, onClick, icon, label }) => (
    <button
        onClick={onClick}
        style={{
            display: 'flex', alignItems: 'center', gap: '0.75rem', padding: '0.75rem 1rem', borderRadius: '10px',
            background: active ? 'var(--accent-cyan-10)' : 'transparent',
            color: active ? 'var(--accent-cyan)' : 'var(--text-secondary)',
            border: 'none', cursor: 'pointer', transition: 'all 0.2s', textAlign: 'left', width: '100%',
            fontWeight: active ? 700 : 500
        }}
    >
        {icon}
        <span style={{ fontSize: '0.85rem' }}>{label}</span>
        {active && <motion.div layoutId="filter-dot" style={{ width: '4px', height: '4px', borderRadius: '50%', background: 'currentColor', marginLeft: 'auto' }} />}
    </button>
);

const SkillCard: React.FC<{ skill: Skill, index: number, onInstalled: (name: string) => void }> = ({ skill, index, onInstalled }) => {
    const { t } = useTranslation();
    const { showToast } = useToast();
    const [isInstalling, setIsInstalling] = useState(false);

    const handleInstall = async () => {
        setIsInstalling(true);
        try {
            const res = await authenticatedFetch(`${API_BASE}/api/v1/skills/install`, {
                method: 'POST',
                body: JSON.stringify({ name: skill.name })
            });
            if (res.ok) {
                showToast('success', t('skill.installSuccess', { defaultValue: `${skill.name} installed successfully.` }));
                onInstalled(skill.name);
            } else {
                showToast('error', t('skill.installFailed', { defaultValue: `Failed to install ${skill.name}.` }));
            }
        } catch (error) {
            console.error("Install failed:", error);
            showToast('error', t('skill.installNetworkError', { defaultValue: 'Network error during installation.' }));
        } finally {
            setIsInstalling(false);
        }
    };

    return (
        <motion.div
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ delay: index * 0.05 }}
            className="main-panel card-hover"
            style={{ padding: '1.5rem', position: 'relative', height: 'fit-content' }}
        >
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start', marginBottom: '1rem' }}>
                <div style={{
                    width: '40px', height: '40px', borderRadius: '10px', background: 'var(--white-05)',
                    display: 'flex', alignItems: 'center', justifyContent: 'center'
                }}>
                    {skill.source === 'wasm' && <Terminal size={20} color="var(--accent-cyan)" />}
                    {skill.source === 'mcp' && <Cpu size={20} color="var(--accent-amber)" />}
                    {skill.source === 'marketplace' && <Box size={20} color="var(--accent-purple)" />}
                </div>
                <div style={{ display: 'flex', gap: '0.5rem' }}>
                    {skill.status === 'Active' ? (
                        <span style={{ fontSize: '0.65rem', padding: '2px 8px', borderRadius: '4px', background: 'var(--accent-emerald-10)', color: 'var(--accent-emerald)', border: '1px solid var(--accent-emerald-20)' }}>
                            {t('skill.stable', { defaultValue: 'STABLE' })}
                        </span>
                    ) : (
                        <span style={{ fontSize: '0.65rem', padding: '2px 8px', borderRadius: '4px', background: 'var(--white-05)', color: 'var(--text-muted)' }}>
                            {t('skill.idle', { defaultValue: 'IDLE' })}
                        </span>
                    )}
                    <span style={{ fontSize: '0.65rem', padding: '2px 8px', borderRadius: '4px', background: 'var(--white-05)', color: 'var(--text-muted)' }}>
                        L{skill.layer}
                    </span>
                </div>
            </div>

            <h3 style={{ fontSize: '1.1rem', fontWeight: 800, marginBottom: '0.5rem' }}>{skill.name}</h3>
            <p style={{ fontSize: '0.8rem', color: 'var(--text-secondary)', lineHeight: 1.5, marginBottom: '1.5rem', minHeight: '2.4rem' }}>
                {skill.description}
            </p>

            <div style={{ marginBottom: '1.5rem' }}>
                <div style={{ fontSize: '0.65rem', color: 'var(--text-muted)', marginBottom: '0.5rem', letterSpacing: '0.05em' }}>{t('skill.exposedTools')}</div>
                <div style={{ display: 'flex', flexWrap: 'wrap', gap: '0.4rem' }}>
                    {(skill.tools || []).map((tool, idx) => (
                        <code key={`${tool}-${idx}`} style={{ fontSize: '0.7rem', padding: '2px 6px', borderRadius: '4px', background: 'var(--black-30)', color: 'var(--accent-cyan)' }}>
                            {tool}
                        </code>
                    ))}
                    {(skill.tools || []).length === 0 && <span style={{ fontSize: '0.7rem', color: 'var(--text-muted)' }}>{t('skill.noTools')}</span>}
                </div>
            </div>

            <div style={{ display: 'flex', gap: '0.75rem', marginTop: 'auto' }}>
                {skill.source === 'marketplace' ? (
                    <button 
                        className="primary-button" 
                        onClick={handleInstall}
                        disabled={isInstalling || skill.status === 'Installed'}
                        style={{ flex: 1, padding: '0.6rem', fontSize: '0.8rem', display: 'flex', alignItems: 'center', justifyContent: 'center', gap: '0.5rem', background: skill.status === 'Installed' ? 'var(--accent-emerald)' : 'var(--accent-cyan)', color: 'var(--bg-primary)', opacity: skill.status === 'Installed' ? 0.7 : 1 }}
                    >
                        <Download size={14} /> {skill.status === 'Installed' ? t('skill.installed', { defaultValue: 'Installed' }) : isInstalling ? t('skill.installing', { defaultValue: 'Installing...' }) : t('skill.install', { defaultValue: 'Install Skill' })}
                    </button>
                ) : (
                    <>
                        <button className="primary-button" style={{ flex: 1, padding: '0.6rem', fontSize: '0.8rem', display: 'flex', alignItems: 'center', justifyContent: 'center', gap: '0.5rem', background: 'var(--accent-cyan-glass)', color: 'var(--accent-cyan)' }}>
                            <Play size={14} /> {t('skill.runTest', { defaultValue: 'Run Test' })}
                        </button>
                        <button 
                            aria-label={t('skill.settings')}
                            style={{
                            padding: '0.6rem', borderRadius: '8px', border: '1px solid var(--white-10)', background: 'transparent', color: 'var(--text-primary)', cursor: 'pointer'
                        }}>
                            <Settings size={14} />
                        </button>
                    </>
                )}
            </div>

            {skill.source === 'mcp' && (
                <div style={{ position: 'absolute', top: '1rem', right: '4rem' }}>
                    <Lock size={14} color="var(--accent-amber)" style={{ opacity: 0.5 }} />
                </div>
            )}
        </motion.div>
    );
};

export default SkillVault;

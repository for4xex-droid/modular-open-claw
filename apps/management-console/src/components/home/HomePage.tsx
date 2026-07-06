/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import React, { useState, lazy, Suspense, useMemo, useEffect } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { Home, ShoppingBag, Globe, Settings } from 'lucide-react';
import { AgentStats, VitalityUIEvent } from '../../types';
import { VitalityEvent } from '../../hooks/useSystemVitality';
import CharacterPanel from './CharacterPanel';
import StoryFlow from './StoryFlow';
import AvatarViewerModal from './AvatarViewerModal';
import { useAvatarState } from '../../hooks/useAvatarState';
import { useDisplayMode } from '../../hooks/useDisplayMode';
import { useAvatarCharacter } from '../../hooks/AvatarContext';
import { useTranslation } from '../../i18n';

// === Lazy-loaded sub-pages ===
// Home (sidebar widget)
const TreasureBox = lazy(() => import('../TreasureBox').then(m => ({ default: m.TreasureBox })));

// Shop sub-tabs
const VoiceStore = lazy(() => import('../VoiceStore'));
const ArtifactVault = lazy(() => import('../ArtifactVault'));

// World sub-tabs
const CommuneDialogueView = lazy(() => import('../CommuneDialogueView'));
const GraphView = lazy(() => import('../GraphView'));
const CausalVisualizer = lazy(() => import('../CausalVisualizer'));
const DemoView = lazy(() => import('../DemoView'));
const BiotopeView = lazy(() => import('../BiotopeView'));
const Timeline = lazy(() => import('../Timeline'));
const BiomeGame = lazy(() => import('../../lib/biome/BiomeGame').then(m => ({ default: m.BiomeGame })));


// Settings sub-tabs
const SettingsPage = lazy(() => import('../SettingsPage'));
const ImmuneSystem = lazy(() => import('../ImmuneSystem'));
const SkillVault = lazy(() => import('../SkillVault'));
const LoraTrainingView = lazy(() => import('../LoraTrainingView'));
const ExpressionPipeline = lazy(() => import('../ExpressionPipeline'));
const DiagnosticsHistory = lazy(() => import('../DiagnosticsHistory'));

// === Types ===
interface HomePageProps {
    stats: AgentStats;
    vitalityEvents?: VitalityEvent[];
    connectionStatus?: string;
    recentEvents?: VitalityUIEvent[];
    lastEvent?: any;
    sessionSavedChars?: number;
}

export type AvatarStateLiteral = 'idle' | 'thinking' | 'speaking' | 'learning' | 'meditating' | 'awakened';

type MainTab = 'home' | 'shop' | 'world' | 'settings';

// === Mini-Tab Bar Component ===
const MiniTabBar: React.FC<{
    tabs: { key: string; label: string }[];
    active: string;
    onChange: (key: string) => void;
}> = ({ tabs, active, onChange }) => (
    <div style={{
        display: 'flex',
        gap: '2px',
        marginBottom: '0.6rem',
        padding: '3px',
        background: 'var(--white-03)',
        borderRadius: '8px',
        border: '1px solid var(--border-glass)',
    }}>
        {tabs.map(tab => (
            <button
                key={tab.key}
                onClick={() => onChange(tab.key)}
                style={{
                    flex: 1,
                    padding: '0.3rem 0.4rem',
                    borderRadius: '6px',
                    border: 'none',
                    cursor: 'pointer',
                    fontSize: '0.68rem',
                    fontWeight: active === tab.key ? 700 : 400,
                    color: active === tab.key ? 'var(--bg-primary)' : 'var(--text-muted)',
                    background: active === tab.key ? 'var(--accent-purple)' : 'transparent',
                    transition: 'all 0.15s ease',
                    whiteSpace: 'nowrap',
                    letterSpacing: '0.01em',
                }}
            >
                {tab.label}
            </button>
        ))}
    </div>
);

// Configs are moved into the component to support useTranslation

// === Main Component ===
const HomePage: React.FC<HomePageProps> = ({
    stats,
    vitalityEvents = [],
    connectionStatus = 'disconnected',
    recentEvents = [],
    lastEvent = null,
    sessionSavedChars = 0,
}) => {
    const { t } = useTranslation();

    const shopSubTabs = useMemo(() => [
        { key: 'store', label: t('home.tab.store') },
        { key: 'collection', label: t('home.tab.collection') },
    ], [t]);

const DEMO_SEEN_KEY = 'aiome_demo_seen';

    const [demoSeen, setDemoSeen] = useState(() => !!localStorage.getItem(DEMO_SEEN_KEY));

    const worldSubTabs = useMemo(() => {
        const tabs = [
            { key: 'observe', label: t('home.tab.observe') },
            { key: 'biome', label: t('home.tab.biome') },
            { key: 'connections', label: t('home.tab.connections') },
        ];
        if (!demoSeen) {
            tabs.push({ key: 'demo', label: t('home.tab.demo') });
        }
        return tabs;
    }, [t, demoSeen]);

    const observeSubTabs = useMemo(() => [
        { key: 'dashboard', label: t('home.tab.dashboard') },
        { key: 'trace', label: t('home.tab.trace') },
        { key: 'chronicle', label: t('home.tab.chronicle') },
    ], [t]);

    const connectionsSubTabs = useMemo(() => [
        { key: 'p2p', label: t('home.tab.p2p') },
        { key: 'map', label: t('home.tab.map') },
    ], [t]);

    const settingsSubTabs = useMemo(() => [
        { key: 'general', label: t('home.tab.general') },
        { key: 'security', label: t('home.tab.security') },
        { key: 'skills', label: t('home.tab.skills') },
        { key: 'training', label: t('home.tab.training') },
        { key: 'expression', label: t('home.tab.expression') },
        { key: 'audit', label: t('home.tab.audit') },
    ], [t]);

    const mainTabConfig = useMemo(() => [
        { key: 'home' as MainTab, icon: <Home size={15} />, labelKey: 'home.mainTab.home', tooltip: t('home.tooltip.home') },
        { key: 'shop' as MainTab, icon: <ShoppingBag size={15} />, labelKey: 'home.mainTab.shop', tooltip: t('home.tooltip.shop') },
        { key: 'world' as MainTab, icon: <Globe size={15} />, labelKey: 'home.mainTab.world', tooltip: t('home.tooltip.world') },
        { key: 'settings' as MainTab, icon: <Settings size={15} />, labelKey: 'home.mainTab.settings', tooltip: t('home.tooltip.settings') },
    ], [t]);
    const avatarState = useAvatarState() as AvatarStateLiteral;
    const { mode } = useDisplayMode();
    const { getAssetPath } = useAvatarCharacter();
    const [isViewerOpen, setIsViewerOpen] = useState(false);

    // Main tab state
    const [activeMainTab, setActiveMainTab] = useState<MainTab>('home');

    // Sub-tab states
    const [shopSubTab, setShopSubTab] = useState('store');
    const [worldSubTab, setWorldSubTab] = useState('observe');
    const [observeSubTab, setObserveSubTab] = useState('dashboard');
    const [connectionsSubTab, setConnectionsSubTab] = useState('p2p');
    const [settingsSubTab, setSettingsSubTab] = useState('general');

    useEffect(() => {
        if (worldSubTab === 'demo') {
            localStorage.setItem(DEMO_SEEN_KEY, '1');
            setDemoSeen(true);
        }
    }, [worldSubTab]);

    const modelUrl = (mode as string) === 'vrm' ? getAssetPath('vrm') : ((mode as string) === 'inx' ? getAssetPath('inx') : '');

    // Settings/World use full width (no CharacterPanel)
    const isFullWidth = activeMainTab === 'settings' || activeMainTab === 'world';
    const isConnected = connectionStatus === 'connected';

    // === Content renderers ===
    const renderShopContent = () => (
        <div style={{ height: '100%', overflow: 'auto' }}>
            <MiniTabBar tabs={shopSubTabs} active={shopSubTab} onChange={setShopSubTab} />
            <AnimatePresence mode="wait">
                <motion.div key={shopSubTab} initial={{ opacity: 0 }} animate={{ opacity: 1 }} transition={{ duration: 0.1 }} style={{ height: 'calc(100% - 2.5rem)' }}>
                    {shopSubTab === 'store' && <VoiceStore />}
                    {shopSubTab === 'collection' && <ArtifactVault />}
                </motion.div>
            </AnimatePresence>
        </div>
    );

    const renderWorldContent = () => (
        <div style={{ height: '100%', overflow: 'auto' }}>
            <MiniTabBar tabs={worldSubTabs} active={worldSubTab} onChange={setWorldSubTab} />
            <AnimatePresence mode="wait">
                <motion.div key={worldSubTab} initial={{ opacity: 0 }} animate={{ opacity: 1 }} transition={{ duration: 0.1 }} style={{ height: 'calc(100% - 2.5rem)' }}>
                    {worldSubTab === 'observe' && (
                        <div style={{ height: '100%', display: 'flex', flexDirection: 'column' }}>
                            <MiniTabBar tabs={observeSubTabs} active={observeSubTab} onChange={setObserveSubTab} />
                            {observeSubTab === 'dashboard' && <BiotopeView stats={stats} isConnected={isConnected} recentEvents={recentEvents} sessionSavedChars={sessionSavedChars} />}
                            {observeSubTab === 'trace' && <CausalVisualizer />}
                            {observeSubTab === 'chronicle' && <Timeline />}
                        </div>
                    )}
                    {worldSubTab === 'connections' && (
                        <div style={{ height: '100%', display: 'flex', flexDirection: 'column' }}>
                            <MiniTabBar tabs={connectionsSubTabs} active={connectionsSubTab} onChange={setConnectionsSubTab} />
                            {connectionsSubTab === 'p2p' && <CommuneDialogueView />}
                            {connectionsSubTab === 'map' && <GraphView />}
                        </div>
                    )}
                    {worldSubTab === 'demo' && <DemoView stats={stats} lastEvent={lastEvent} isConnected={isConnected} />}
                    {worldSubTab === 'biome' && <BiomeGame />}
                </motion.div>
            </AnimatePresence>
        </div>
    );

    const renderSettingsContent = () => (
        <div style={{ height: '100%', overflow: 'auto' }}>
            <MiniTabBar tabs={settingsSubTabs} active={settingsSubTab} onChange={setSettingsSubTab} />
            <AnimatePresence mode="wait">
                <motion.div key={settingsSubTab} initial={{ opacity: 0 }} animate={{ opacity: 1 }} transition={{ duration: 0.1 }} style={{ height: 'calc(100% - 2.5rem)' }}>
                    {settingsSubTab === 'general' && <SettingsPage />}
                    {settingsSubTab === 'security' && <ImmuneSystem />}
                    {settingsSubTab === 'skills' && <SkillVault />}
                    {settingsSubTab === 'training' && <LoraTrainingView />}
                    {settingsSubTab === 'expression' && <ExpressionPipeline />}
                    {settingsSubTab === 'audit' && <DiagnosticsHistory />}
                </motion.div>
            </AnimatePresence>
        </div>
    );

    return (
        <div className="home-v2-container">
            {/* Left: Character Panel (hidden on full-width tabs) */}
            {!isFullWidth && (
                <div className="home-v2-left-pane">
                    <CharacterPanel
                        stats={stats}
                        onOpenViewer={() => setIsViewerOpen(true)}
                        isViewerOpen={isViewerOpen}
                        modelUrl={modelUrl}
                        avatarState={avatarState}
                        mode={mode}
                        sessionSavedChars={sessionSavedChars}
                    />

                    {/* クイック起動「バイオーム」カード */}
                    <div style={{
                        padding: '1rem',
                        background: 'var(--bg-glass-heavy)',
                        backdropFilter: 'blur(12px)',
                        border: '1px solid var(--border-glass-bright)',
                        borderRadius: '12px',
                        display: 'flex',
                        flexDirection: 'column',
                        gap: '0.75rem',
                        boxShadow: 'var(--shadow-deep)'
                    }} data-testid="biome-quick-card">
                        <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
                            <span style={{ fontSize: '1.25rem' }}>🎮</span>
                            <div style={{ display: 'flex', flexDirection: 'column' }}>
                                <span style={{ fontWeight: 'bold', fontSize: '0.85rem', color: 'var(--white-100)' }}>{t('home.biomeCard.title')}</span>
                                <span style={{ fontSize: '0.7rem', color: 'var(--text-muted)' }}>{t('home.biomeCard.subtitle')}</span>
                            </div>
                        </div>
                        <div style={{ display: 'flex', gap: '0.5rem' }}>
                            <button
                                type="button"
                                className="home-biome-btn-primary"
                                onClick={() => {
                                    setActiveMainTab('world');
                                    setWorldSubTab('biome');
                                }}
                                data-testid="quick-start-biome"
                            >
                                {t('home.biomeCard.startGame')}
                            </button>
                            <button
                                type="button"
                                className="home-biome-btn-secondary"
                                onClick={() => window.open('/biome-popup.html', 'Biome Game', 'width=1100,height=800,menubar=no,toolbar=no,location=no,status=no')}
                                title={t('home.biomeCard.popupTitle')}
                                data-testid="quick-popup-biome"
                            >
                                {t('home.biomeCard.popupLabel')}
                            </button>
                        </div>
                    </div>

                    <Suspense fallback={null}>
                        <TreasureBox />
                    </Suspense>
                </div>
            )}

            {/* Right: Tab-navigated content */}
            <div style={{ flex: '1 1 auto', overflow: 'hidden', display: 'flex', flexDirection: 'column' }}>
                {/* Main Tab Navigation */}
                <div style={{
                    display: 'flex',
                    gap: '0.25rem',
                    marginBottom: '0.75rem',
                    background: 'var(--white-02)',
                    border: '1px solid var(--border-glass)',
                    borderRadius: '12px',
                    padding: '4px',
                }}>
                    {mainTabConfig.map(tab => {
                        const isActive = activeMainTab === tab.key;
                        return (
                            <button
                                key={tab.key}
                                onClick={() => setActiveMainTab(tab.key)}
                                data-tooltip={tab.tooltip}
                                style={{
                                    flex: 1,
                                    display: 'flex',
                                    alignItems: 'center',
                                    justifyContent: 'center',
                                    gap: '0.4rem',
                                    padding: '0.5rem 0.75rem',
                                    borderRadius: '9px',
                                    border: 'none',
                                    cursor: 'pointer',
                                    fontSize: '0.8rem',
                                    fontWeight: isActive ? 700 : 500,
                                    color: isActive ? 'var(--bg-primary)' : 'var(--text-secondary)',
                                    background: isActive ? 'var(--accent-cyan)' : 'transparent',
                                    transition: 'all 0.2s ease',
                                    position: 'relative',
                                }}
                            >
                                {tab.icon}
                                {t(tab.labelKey)}
                                {isActive && (
                                    <motion.div
                                        layoutId="subtab-active"
                                        style={{
                                            position: 'absolute',
                                            inset: 0,
                                            borderRadius: '9px',
                                            background: 'var(--accent-cyan)',
                                            zIndex: -1,
                                        }}
                                        transition={{ type: 'spring', stiffness: 400, damping: 30 }}
                                    />
                                )}
                            </button>
                        );
                    })}
                </div>

                {/* Tab Content */}
                <div style={{ flex: 1, overflow: 'hidden' }}>
                    <AnimatePresence mode="wait">
                        <motion.div
                            key={activeMainTab}
                            initial={{ opacity: 0, y: 6 }}
                            animate={{ opacity: 1, y: 0 }}
                            exit={{ opacity: 0, y: -6 }}
                            transition={{ duration: 0.15 }}
                            style={{ height: '100%' }}
                        >
                            <Suspense fallback={
                                <div style={{ height: '100%', display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
                                    <div className="ani-pulse" style={{ color: 'var(--accent-cyan)', fontWeight: 700 }}>{t('loading')}</div>
                                </div>
                            }>
                                {activeMainTab === 'home' && <StoryFlow sysEvents={vitalityEvents} connectionStatus={connectionStatus} sessionSavedChars={sessionSavedChars} />}
                                {activeMainTab === 'shop' && renderShopContent()}
                                {activeMainTab === 'world' && renderWorldContent()}
                                {activeMainTab === 'settings' && renderSettingsContent()}
                            </Suspense>
                        </motion.div>
                    </AnimatePresence>
                </div>
            </div>

            {/* Avatar Viewer Modal */}
            <AvatarViewerModal
                isOpen={isViewerOpen}
                onClose={() => setIsViewerOpen(false)}
                modelUrl={modelUrl}
                avatarState={avatarState}
                mode={mode}
            />
        </div>
    );
};

export default HomePage;

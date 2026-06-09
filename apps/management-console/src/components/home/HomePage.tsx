/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import React, { useState, lazy, Suspense } from 'react';
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

// === Sub-tab configs ===
const shopSubTabs = [
    { key: 'store', label: 'ストア' },
    { key: 'collection', label: 'コレクション' },
];

const worldSubTabs = [
    { key: 'p2p', label: 'P2P対話' },
    { key: 'dashboard', label: 'ダッシュボード' },
    { key: 'map', label: 'マップ' },
    { key: 'trace', label: 'トレース' },
    { key: 'chronicle', label: 'クロニクル' },
    { key: 'demo', label: 'デモ' },
];

const settingsSubTabs = [
    { key: 'general', label: '基本設定' },
    { key: 'security', label: 'セキュリティ' },
    { key: 'skills', label: 'スキル' },
    { key: 'training', label: 'AI学習' },
    { key: 'expression', label: '表現' },
    { key: 'audit', label: '監査' },
];

// === Main tab config ===
const mainTabConfig: { key: MainTab; icon: React.ReactNode; labelKey: string }[] = [
    { key: 'home', icon: <Home size={15} />, labelKey: 'Home' },
    { key: 'shop', icon: <ShoppingBag size={15} />, labelKey: 'Shop' },
    { key: 'world', icon: <Globe size={15} />, labelKey: 'World' },
    { key: 'settings', icon: <Settings size={15} />, labelKey: 'Settings' },
];

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
    const avatarState = useAvatarState() as AvatarStateLiteral;
    const { mode } = useDisplayMode();
    const { getAssetPath } = useAvatarCharacter();
    const [isViewerOpen, setIsViewerOpen] = useState(false);

    // Main tab state
    const [activeMainTab, setActiveMainTab] = useState<MainTab>('home');

    // Sub-tab states
    const [shopSubTab, setShopSubTab] = useState('store');
    const [worldSubTab, setWorldSubTab] = useState('p2p');
    const [settingsSubTab, setSettingsSubTab] = useState('general');

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
                    {worldSubTab === 'p2p' && <CommuneDialogueView />}
                    {worldSubTab === 'dashboard' && <BiotopeView stats={stats} isConnected={isConnected} recentEvents={recentEvents} sessionSavedChars={sessionSavedChars} />}
                    {worldSubTab === 'map' && <GraphView />}
                    {worldSubTab === 'trace' && <CausalVisualizer />}
                    {worldSubTab === 'chronicle' && <Timeline />}
                    {worldSubTab === 'demo' && <DemoView stats={stats} lastEvent={lastEvent} isConnected={isConnected} />}
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
        <div className="home-v2-container" style={{
            display: 'flex',
            height: '100%',
            gap: '1rem',
            padding: '1rem',
            overflow: 'hidden'
        }}>
            {/* Left: Character Panel (hidden on full-width tabs) */}
            {!isFullWidth && (
                <div style={{ flex: '0 0 320px', minWidth: '320px', display: 'flex', flexDirection: 'column', gap: '0.75rem', overflow: 'auto' }}>
                    <CharacterPanel
                        stats={stats}
                        onOpenViewer={() => setIsViewerOpen(true)}
                        isViewerOpen={isViewerOpen}
                        modelUrl={modelUrl}
                        avatarState={avatarState}
                        mode={mode}
                        sessionSavedChars={sessionSavedChars}
                    />
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
                                {tab.labelKey}
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

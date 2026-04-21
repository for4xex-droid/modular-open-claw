/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import React, { useState, useEffect, useMemo } from "react";
import { useTranslation, useLanguage } from "./i18n";
import { motion, AnimatePresence } from "framer-motion";
import {
  Activity,
  Shield,
  Clock,
  GitMerge,
  MessageSquare,
  BrainCircuit,
  Package,
  Box,
  Settings as SettingsIcon,
  Zap,
  Sparkles,
  Network,
  Crown,
  Play,
  Library
} from "lucide-react";
const OnboardingModal = React.lazy(() => import("./components/OnboardingModal"));
const SystemBirth = React.lazy(() => import("./components/SystemBirth"));
const HomePage = React.lazy(() => import("./components/home/HomePage"));
const BiotopeView = React.lazy(() => import("./components/BiotopeView"));
const Timeline = React.lazy(() => import("./components/Timeline"));
const ImmuneSystem = React.lazy(() => import("./components/ImmuneSystem"));
const AgentConsole = React.lazy(() => import("./components/AgentConsole"));
const SeoPulseView = React.lazy(() => import("./components/SeoPulseView"));
const SkillVault = React.lazy(() => import("./components/SkillVault"));
const ArtifactVault = React.lazy(() => import("./components/ArtifactVault"));
const DiagnosticsHistory = React.lazy(() => import("./components/DiagnosticsHistory"));
const GraphView = React.lazy(() => import("./components/GraphView"));
const PromptStatsView = React.lazy(() => import("./components/PromptStatsView"));
const SettingsPage = React.lazy(() => import("./components/SettingsPage"));
const ExpressionPipeline = React.lazy(() => import("./components/ExpressionPipeline"));
const LoraTrainingView = React.lazy(() => import("./components/LoraTrainingView"));
const BiomeDialogueView = React.lazy(() => import("./components/BiomeDialogueView"));
const VoiceStore = React.lazy(() => import("./components/VoiceStore"));
const DemoView = React.lazy(() => import("./components/DemoView"));
const CausalVisualizer = React.lazy(() => import("./components/CausalVisualizer"));
const CortexView = React.lazy(() => import("./components/cortex/CortexView"));
import DioramaView from "./components/diorama/DioramaView";
const AuthOverlay = React.lazy(() => import("./components/AuthOverlay"));
const TaskApprovalOverlay = React.lazy(() => import("./components/TaskApprovalOverlay"));
import { SoTProgressBar } from "./components/SoTProgressBar";

import { isAuthenticated } from "./lib/auth";
import { useAvatarState } from "./hooks/useAvatarState";
import { useDisplayMode } from "./hooks/useDisplayMode";
import { AgentStats, VitalityUIEvent, Karma, SoTEvent } from "./types";
import { useSystemVitality } from "./hooks/useSystemVitality";
import { useViewMode } from "./hooks/useViewMode";
import { useTokenHealth } from "./hooks/useTokenHealth";

function App() {
  const { t } = useTranslation();
  const { lang, setLang } = useLanguage();
  const [activeTab, setActiveTab] = useState("home-v2");
  const [stats, setStats] = useState<AgentStats>({ level: 1, exp: 0, resonance: 0, creativity: 0, fatigue: 0 });
  const [showOnboarding, setShowOnboarding] = useState(false);
  const [showBirth, setShowBirth] = useState(false);
  const [recentEvents, setRecentEvents] = useState<VitalityUIEvent[]>([]);
  const [isAuth, setIsAuth] = useState(isAuthenticated());
  const [sessionSavedChars, setSessionSavedChars] = useState(0);
  const seenTokenEventsRef = React.useRef(new Set<number>());

  const { events: vitalityEvents, lastEvent, connectionStatus, toggleConnection, lastPingMs } = useSystemVitality();

  const isConnected = connectionStatus === 'connected';

  const avatarState = useAvatarState();
  const { mode } = useDisplayMode();

  useEffect(() => {
    const isFirstVisit = localStorage.getItem("aiome_onboarding_done") !== "true";
    if (isFirstVisit) {
      setShowOnboarding(true);
    }
  }, []);

  // Global event processor & stats updater
  useEffect(() => {
    if (!lastEvent) return;
    const { type, data } = lastEvent;

    const addEvent = (title: string, desc: string, color: string, icon: React.ReactNode) => {
      const id = Date.now();
      setRecentEvents((prev: VitalityUIEvent[]) => [{ id, title, desc, color, icon }, ...prev].slice(0, 30));
    };

    switch (type) {
      case 'level_up': {
        const d = data as AgentStats;
        setStats(prev => ({ ...prev, level: d.level, exp: d.exp }));
        addEvent(t('event.levelUp'), t('event.ascensionLevel', { level: d.level }), 'var(--accent-cyan)', <Activity size={16} />);
        break;
      }
      case 'karma_update': {
        const d = data as Karma;
        addEvent(t('event.karmaAssimilated'), t('event.synapsesMerged', { id: d.id.substring(0, 8) }), 'var(--accent-purple)', <GitMerge size={16} />);
        break;
      }
      case 'immune_alert': {
        const d = data as any;
        addEvent(t('event.securityAlert'), d.description || t('event.anomalyDetected'), 'var(--accent-rose)', <Shield size={16} />);
        break;
      }
      case 'job_started': {
        addEvent(t('event.deliberationStarted'), typeof data === 'string' ? data : t('event.thinking'), 'var(--accent-amber)', <Activity size={16} />);
        break;
      }
      case 'skill_execution': {
        addEvent(t('event.skillActivating'), typeof data === 'string' ? data : t('event.toolExecution'), 'var(--accent-emerald)', <Zap size={16} />);
        break;
      }
      case 'inspiration': {
        const d = data as any;
        addEvent(t('event.inspiration'), d.description || t('event.creativeSpark'), 'var(--accent-rose)', <BrainCircuit size={16} />);
        break;
      }
      case 'agent_stats': {
        const d = data as AgentStats;
        setStats(d);
        break;
      }
      case 'proactive_talk': {
        const d = data as string;
        addEvent(t('event.aiomeMessage'), d, 'var(--accent-cyan)', <MessageSquare size={16} />);
        break;
      }
      case 'sot_progress': {
        const d = data as SoTEvent;
        addEvent(t('event.societyOfThought'), t('event.deliberationUpdate', { type: d.event.type }), 'var(--accent-purple)', <BrainCircuit size={16} />);
        break;
      }
      case 'token_saved': {
        const d = data as { saved_chars: number; ts: number };
        if (!seenTokenEventsRef.current.has(d.ts)) {
            seenTokenEventsRef.current.add(d.ts);
            
            // Prevent unbounded memory growth in 24/7 running dashboard tabs
            if (seenTokenEventsRef.current.size > 1000) {
               const oldestTs = seenTokenEventsRef.current.values().next().value;
               if (oldestTs) seenTokenEventsRef.current.delete(oldestTs);
            }

            setSessionSavedChars(prev => prev + d.saved_chars);
            addEvent('⚡ Token Optimized',
                `${d.saved_chars} chars saved (≈${Math.round(d.saved_chars / 4)} tokens)`,
                'var(--accent-emerald)', <Zap size={16} />);
        }
        break;
      }
      case 'quality_gate': {
        const d = data as { job_id: string; score: number; passed: boolean };
        addEvent(
          t('event.qualityGate', { defaultValue: 'Quality Gate' }) as string,
          `Score: ${d.score} - ${d.passed ? 'PASSED' : 'FAILED'} (${d.job_id.substring(0, 8)})`,
          d.passed ? 'var(--accent-emerald)' : 'var(--accent-rose)',
          <Activity size={16} />
        );
        break;
      }
      default:
        break;
    }
  }, [lastEvent]);

  // Status Badge Rendering Logic
  const renderStatusBadge = () => {
    let badgeClass = "status-badge";
    let dotClass = "status-dot";
    let text = "";

    switch (connectionStatus) {
      case "connected":
        text = lastPingMs !== null ? t('status.connectedMs', { ms: lastPingMs }) : t('status.hubConnected');
        // Default classes are fine
        break;
      case "connecting":
        badgeClass += ' disconnected'; // Using disconnected style for connecting state
        dotClass += ' offline'; // Using offline dot style for connecting state
        dotClass += ' ani-pulse';
        text = t('status.reconnecting');
        break;
      case "paused":
        badgeClass += ' paused';
        dotClass += ' offline';
        dotClass = dotClass.replace('offline', 'paused'); // Custom styling inline if needed
        text = t('status.syncPaused');
        break;
      case "disconnected":
      default:
        badgeClass += ' disconnected';
        dotClass += ' offline';
        text = t('status.connectionLost');
        break;
    }

    return (
      <button
        className={badgeClass}
        onClick={toggleConnection}
        style={{
          cursor: 'pointer', border: '1px solid var(--white-05)', background: 'var(--black-40)',
          outline: 'none', transition: 'all 0.2s', padding: '0.5rem 1rem'
        }}
        title="Click to toggle connection sync"
      >
        <div className={dotClass} style={{
          background: connectionStatus === 'paused' ? 'var(--accent-amber)' : undefined,
          boxShadow: connectionStatus === 'paused' ? 'var(--glow-amber)' : undefined
        }} />
        {text}
      </button>
    );
  };

  const { viewMode } = useViewMode();
  const { isExpired, dismiss } = useTokenHealth();

  const isVisible = (tab: string) => {
    const beginner = ['home-v2', 'dashboard', 'demo', 'karma', 'expressions', 'settings'];
    const intermediate = [...beginner, 'artifacts', 'agent', 'cortex', 'vault', 'store', 'biome', 'causal', 'lora'];
    const advanced = [...intermediate, 'graph', 'audit', 'prompt-stats', 'immune'];
    
    if (viewMode === 'beginner') return beginner.includes(tab);
    if (viewMode === 'intermediate') return intermediate.includes(tab);
    return advanced.includes(tab);
  };

  return (
    <div className="app-container">
      <AnimatePresence>
        {!isAuth && (
          <React.Suspense fallback={null}>
            <AuthOverlay onAuthenticated={() => setIsAuth(true)} />
          </React.Suspense>
        )}
      </AnimatePresence>

      <AnimatePresence>
        {isExpired && isAuth && (
          <motion.div
            initial={{ opacity: 0, y: -50 }}
            animate={{ opacity: 1, y: 20 }}
            exit={{ opacity: 0, y: -50 }}
            style={{
              position: 'fixed', top: 0, left: '50%', transform: 'translateX(-50%)', zIndex: 10000,
              background: 'var(--accent-rose-10)', border: '1px solid var(--accent-rose-30)',
              borderRadius: 'var(--radius-md)', padding: '1rem',
              display: 'flex', alignItems: 'center', gap: '1rem',
              boxShadow: '0 10px 30px var(--black-50), 0 0 20px var(--accent-rose-10)',
              backdropFilter: 'blur(10px)'
            }}
          >
            <Shield size={20} color="var(--accent-rose)" />
            <span style={{ color: 'var(--accent-rose)', fontWeight: 600, fontSize: '0.9rem' }}>
              Session expired. Please update your API secret.
            </span>
            <button
               onClick={() => { setActiveTab("settings"); dismiss(); }}
               style={{
                 background: 'var(--accent-rose)', color: 'var(--bg-primary)',
                 border: 'none', padding: '0.4rem 0.8rem', borderRadius: '6px',
                 fontWeight: 700, cursor: 'pointer', fontSize: '0.8rem'
               }}
            >
               Go to Settings
            </button>
          </motion.div>
        )}
      </AnimatePresence>

      {/* Digital Diorama — Resident Avatar */}
      <DioramaView status={avatarState} mode={mode} activeTab={activeTab} />
      
      {/* Society of Thought Visualization */}
      <SoTProgressBar />

      {/* Ambient Background Particles */}
      <div style={{ position: 'fixed', inset: 0, pointerEvents: 'none', zIndex: 0, overflow: 'hidden' }}>
        {useMemo(() => [...Array(6)].map((_, i) => (
          <motion.div
            key={i}
            animate={{
              x: [Math.random() * 100 + '%', Math.random() * 100 + '%'],
              y: [Math.random() * 100 + '%', Math.random() * 100 + '%'],
              opacity: [0.1, 0.3, 0.1],
            }}
            transition={{
              duration: 20 + Math.random() * 20,
              repeat: Infinity,
              ease: "linear"
            }}
            style={{
              position: 'absolute',
              width: 300 + Math.random() * 200,
              height: 300 + Math.random() * 200,
              background: i % 2 === 0 ? 'radial-gradient(circle, var(--accent-cyan-05) 0%, transparent 70%)' : 'radial-gradient(circle, var(--accent-purple-glass) 0%, transparent 70%)',
              borderRadius: '50%',
              filter: 'blur(50px)'
            }}
          />
        )), [])}
      </div>

      {/* Sidebar — advanced mode only */}
      {viewMode === 'advanced' && <aside className="sidebar">
        <div className="brand">
          <BrainCircuit size={28} color="var(--accent-cyan)" />
          <span>Aiome</span>
        </div>

        <nav className="nav-group">
          <h4>{t('nav.section.synergyHub')}</h4>
          {isVisible("home-v2") && (
            <NavItem
              icon={<Activity size={20} />}
              label={t('nav.homeV2')}
              active={activeTab === "home-v2"}
              onClick={() => setActiveTab("home-v2")}
            />
          )}
          {isVisible("dashboard") && (
            <NavItem
              icon={<Activity size={20} />}
              label={t('nav.biotope')}
              active={activeTab === "dashboard"}
              onClick={() => setActiveTab("dashboard")}
            />
          )}
          {isVisible("demo") && (
            <NavItem
              icon={<Play size={20} />}
              label={t('nav.demo')}
              active={activeTab === "demo"}
              onClick={() => setActiveTab("demo")}
            />
          )}
          {isVisible("karma") && (
            <NavItem
              icon={<Clock size={20} />}
              label={t('nav.chronicle')}
              active={activeTab === "karma"}
              onClick={() => setActiveTab("karma")}
            />
          )}
          {isVisible("graph") && (
            <NavItem
              icon={<GitMerge size={20} />}
              label={t('nav.resonanceMap')}
              active={activeTab === "graph"}
              onClick={() => setActiveTab("graph")}
            />
          )}
          {isVisible("causal") && (
            <NavItem
              icon={<Activity size={20} />}
              label={t('nav.causalTrace')}
              active={activeTab === "causal"}
              onClick={() => setActiveTab("causal")}
            />
          )}
          {isVisible("artifacts") && (
            <NavItem
              icon={<Box size={20} />}
              label={t('nav.artifactVault')}
              active={activeTab === "artifacts"}
              onClick={() => setActiveTab("artifacts")}
            />
          )}
          {isVisible("audit") && (
            <NavItem
              icon={<Activity size={20} />}
              label={t('nav.audit')}
              active={activeTab === "audit"}
              onClick={() => setActiveTab("audit")}
            />
          )}
          {isVisible("expressions") && (
            <NavItem
              icon={<Sparkles size={20} />}
              label={t('nav.expressions')}
              active={activeTab === "expressions"}
              onClick={() => setActiveTab("expressions")}
            />
          )}
          {isVisible("biome") && (
            <NavItem
              icon={<Network size={20} />}
              label={t('nav.biomeLab')}
              active={activeTab === "biome"}
              onClick={() => setActiveTab("biome")}
            />
          )}
          {isVisible("store") && (
            <NavItem
              icon={<Crown size={20} />}
              label={t('nav.voiceStore')}
              active={activeTab === "store"}
              onClick={() => setActiveTab("store")}
            />
          )}
        </nav>

        <nav className="nav-group">
          <h4>{t('nav.section.control')}</h4>
          {isVisible("immune") && (
            <NavItem
              icon={<Shield size={20} />}
              label={t('nav.immuneSystem')}
              active={activeTab === "immune"}
              onClick={() => setActiveTab("immune")}
            />
          )}
          {isVisible("agent") && (
            <NavItem
              icon={<MessageSquare size={20} />}
              label={t('nav.agentConsole')}
              active={activeTab === "agent"}
              onClick={() => setActiveTab("agent")}
            />
          )}
          {isVisible("cortex") && (
            <NavItem
              icon={<Library size={20} />}
              label={t('nav.cortex')}
              active={activeTab === "cortex"}
              onClick={() => setActiveTab("cortex")}
            />
          )}
          {isVisible("vault") && (
            <NavItem
              icon={<Package size={20} />}
              label={t('nav.skillVault')}
              active={activeTab === "vault"}
              onClick={() => setActiveTab("vault")}
            />
          )}
          {isVisible("prompt-stats") && (
            <NavItem
              icon={<Activity size={20} />}
              label={t('nav.promptStats')}
              active={activeTab === "prompt-stats"}
              onClick={() => setActiveTab("prompt-stats")}
            />
          )}
          {isVisible("lora") && (
            <NavItem
              icon={<BrainCircuit size={20} />}
              label={t('nav.loraAutotuner')}
              active={activeTab === "lora"}
              onClick={() => setActiveTab("lora")}
            />
          )}
          {isVisible("settings") && (
            <NavItem
              icon={<SettingsIcon size={20} />}
              label={t('nav.settings')}
              active={activeTab === "settings"}
              onClick={() => setActiveTab("settings")}
            />
          )}
        </nav>

        <div style={{ marginTop: 'auto', padding: '1rem', background: 'var(--white-03)', borderRadius: '12px', fontSize: '0.8rem' }}>
          <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: '0.5rem' }}>
            <span style={{ color: 'var(--text-secondary)' }}>{t('sidebar.samsaraTier')}</span>
            <span style={{ color: 'var(--accent-purple)' }}>{t('sidebar.level')} {stats.level}</span>
          </div>
          <div style={{ height: '4px', background: 'var(--white-10)', borderRadius: '2px', overflow: 'hidden' }}>
            <motion.div
              initial={{ width: 0 }}
              animate={{ width: `${(stats.exp % 1000) / 10}%` }}
              style={{ height: '100%', background: 'var(--accent-purple)' }}
            />
          </div>
          <div style={{ marginTop: '0.5rem', textAlign: 'center', fontSize: '0.65rem', color: 'var(--text-muted)' }}>
            AIOME {import.meta.env.VITE_APP_VERSION || "v1.0.2"}
          </div>
          <div style={{ display: 'flex', justifyContent: 'center', gap: '0.25rem', marginTop: '0.75rem' }}>
            <button
              onClick={() => setLang('en')}
              style={{
                padding: '0.3rem 0.6rem',
                borderRadius: '6px',
                border: lang === 'en' ? '1px solid var(--accent-cyan)' : '1px solid var(--white-10)',
                background: lang === 'en' ? 'var(--accent-cyan-10)' : 'transparent',
                color: lang === 'en' ? 'var(--accent-cyan)' : 'var(--text-muted)',
                cursor: 'pointer',
                fontSize: '0.7rem',
                fontWeight: 700,
                transition: 'all 0.2s'
              }}
            >
              🇺🇸 {t('language.en')}
            </button>
            <button
              onClick={() => setLang('ja')}
              style={{
                padding: '0.3rem 0.6rem',
                borderRadius: '6px',
                border: lang === 'ja' ? '1px solid var(--accent-cyan)' : '1px solid var(--white-10)',
                background: lang === 'ja' ? 'var(--accent-cyan-10)' : 'transparent',
                color: lang === 'ja' ? 'var(--accent-cyan)' : 'var(--text-muted)',
                cursor: 'pointer',
                fontSize: '0.7rem',
                fontWeight: 700,
                transition: 'all 0.2s'
              }}
            >
              🇯🇵 {t('language.ja')}
            </button>
          </div>
        </div>
      </aside>}

      {/* Main Content */}
      <main className="main-content">
        <header className="header">
          <motion.h2
            initial={{ opacity: 0, x: -20 }}
            animate={{ opacity: 1, x: 0 }}
            key={activeTab}
          >
            {activeTab === "home-v2" && t('page.homeV2')}
            {activeTab === "dashboard" && t('page.biotope')}
            {activeTab === "demo" && t('page.demo')}
            {activeTab === "karma" && t('page.chronicle')}
            {activeTab === "graph" && t('page.resonanceMap')}
            {activeTab === "immune" && t('page.immuneSystem')}
            {activeTab === "agent" && t('page.agentConsole')}
            {activeTab === "cortex" && t('page.cortex')}
            {activeTab === "vault" && t('page.skillVault')}
            {activeTab === "artifacts" && t('page.artifactVault')}
            {activeTab === "audit" && t('page.audit')}
            {activeTab === "prompt-stats" && t('page.promptStats')}
            {activeTab === "expressions" && t('page.expressions')}
            {activeTab === "biome" && t('page.biomeLab')}
            {activeTab === "store" && t('page.voiceStore')}
            {activeTab === "causal" && t('page.causalTrace')}
            {activeTab === "lora" && t('page.loraAutotuner')}
            {activeTab === "settings" && t('page.settings')}
          </motion.h2>

          <div style={{ display: 'flex', gap: '1rem', alignItems: 'center' }}>
            {renderStatusBadge()}
          </div>
        </header>

        <AnimatePresence mode="wait">
          {/* Use Suspense for lazy loaded components */}
          <React.Suspense fallback={<div style={{ height: '70vh', display: 'flex', alignItems: 'center', justifyContent: 'center' }}><div className="ani-pulse" style={{ color: 'var(--accent-cyan)', fontWeight: 700 }}>{t('loading')}</div></div>}>
            <motion.div
              key={activeTab}
              initial={{ opacity: 0, y: 10 }}
              animate={{ opacity: 1, y: 0 }}
              exit={{ opacity: 0, y: -10 }}
              transition={{ duration: 0.2 }}
            >
              {activeTab === "home-v2" && <HomePage stats={stats} vitalityEvents={vitalityEvents} connectionStatus={connectionStatus} recentEvents={recentEvents} lastEvent={lastEvent} sessionSavedChars={sessionSavedChars} />}
              {activeTab === "dashboard" && <BiotopeView stats={stats} isConnected={isConnected} recentEvents={recentEvents} sessionSavedChars={sessionSavedChars} />}
              {activeTab === "demo" && <DemoView stats={stats} lastEvent={lastEvent} isConnected={isConnected} />}
              {activeTab === "karma" && <Timeline />}
              {activeTab === "graph" && <GraphView />}
              {activeTab === "immune" && <ImmuneSystem />}
              {activeTab === "agent" && (
                <>
                  <AgentConsole sessionSavedChars={sessionSavedChars} />
                  <SeoPulseView />
                </>
              )}
              {activeTab === "cortex" && <CortexView />}
              {activeTab === "vault" && <SkillVault />}
              {activeTab === "artifacts" && <ArtifactVault />}
              {activeTab === "audit" && <DiagnosticsHistory />}
              {activeTab === "prompt-stats" && <PromptStatsView />}
              {activeTab === "expressions" && <ExpressionPipeline />}
              {activeTab === "biome" && <BiomeDialogueView />}
              {activeTab === "store" && <VoiceStore />}
              {activeTab === "causal" && <CausalVisualizer />}
              {activeTab === "lora" && <LoraTrainingView />}
              {activeTab === "settings" && <SettingsPage />}
            </motion.div>
          </React.Suspense>
        </AnimatePresence>
      </main>

      <OnboardingModal
        isOpen={showOnboarding}
        onClose={() => {
          setShowOnboarding(false);
          localStorage.setItem("aiome_onboarding_done", "true");
          setShowBirth(true);
        }}
      />

      {showBirth && (
        <SystemBirth onComplete={() => {
          setShowBirth(false);
          localStorage.setItem("aiome_birth_shown", "true");
        }} />
      )}

      <React.Suspense fallback={null}>
        <TaskApprovalOverlay />
      </React.Suspense>
    </div>
  );
}

function NavItem({ icon, label, active, onClick }: { icon: React.ReactNode, label: string, active: boolean, onClick: () => void }) {
  return (
    <div
      className={`nav-item ${active ? 'active' : ''}`}
      onClick={onClick}
    >
      {icon}
      <span>{label}</span>
      {active && <motion.div layoutId="active-pill" className="nav-active-bar" />}
    </div>
  );
}

export default App;

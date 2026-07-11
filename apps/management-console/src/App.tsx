/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import React, { useState, useEffect, useMemo, useRef } from "react";
import { useTranslation, useLanguage } from "./i18n";
import { motion, AnimatePresence } from "framer-motion";
import {
  Activity,
  Shield,
  GitMerge,
  MessageSquare,
  BrainCircuit,
  Zap,
} from "lucide-react";

const LoginScreen = React.lazy(() => import("./components/LoginScreen"));
const SetupWizard = React.lazy(() => import("./components/SetupWizard"));
const ProUpgradeModal = React.lazy(() =>
  import("./components/commerce/ProUpgradeModal").then((m) => ({ default: m.ProUpgradeModal }))
);
const DioramaView = React.lazy(() => import("./components/diorama/DioramaView"));
const TaskApprovalOverlay = React.lazy(() => import("./components/TaskApprovalOverlay"));

import { SoTProgressBar } from "./components/SoTProgressBar";
import { useWorkspacePersona } from "./hooks/useWorkspacePersona";
import { isAuthenticated, clearAuthToken, AUTH_UNAUTHORIZED_EVENT } from "./lib/auth";
import { useAvatarState } from "./hooks/useAvatarState";
import { AiomeSkeleton } from "./components/common/AiomeSkeleton";
import { useDisplayMode } from "./hooks/useDisplayMode";
import { AgentStats, VitalityUIEvent, Karma, SoTEvent, ImmuneAlertEvent, AegisSentinelEvent, InspirationEvent, BiomeEvolutionEvent, CrisisPredictionEvent } from "./types";
import { useSystemVitality } from "./hooks/useSystemVitality";
import { useViewMode } from "./hooks/useViewMode";
import { useAgentIdentity } from "./hooks/useAgentIdentity";
import { useTokenHealth } from "./hooks/useTokenHealth";
import { CheckoutSuccess } from "./components/commerce/CheckoutSuccess";
import { APP_VERSION, API_BASE, STRIPE_PRICE_ID } from "./config";
import { isValidA2uiNavTab } from "./lib/a2uiTabs";

// Split components / configurations
import { AppSidebar } from "./components/AppSidebar";
import { AppHeader } from "./components/AppHeader";
import { AppRoutes } from "./AppRoutes";

/** Valid boot mode states returned from the API normalization layer */
type BootMode = 'Normal' | 'Setup';

/** Maps lowercase backend mode strings to typed frontend values */
const BOOT_MODE_MAP: Readonly<Record<string, BootMode>> = Object.freeze({ normal: 'Normal', setup: 'Setup' });

function App() {
  const { t } = useTranslation();
  const { lang, setLang } = useLanguage();
  const [activeTab, setActiveTab] = useState("home-v2");
  const [stats, setStats] = useState<AgentStats>({ level: 1, exp: 0, resonance: 0, creativity: 0, fatigue: 0 });
  const [bootMode, setBootMode] = useState<BootMode | null>(null);
  const [recentEvents, setRecentEvents] = useState<VitalityUIEvent[]>([]);
  const [isAuth, setIsAuth] = useState(isAuthenticated());
  const [sessionSavedChars, setSessionSavedChars] = useState(0);
  const [isSidebarOpen, setIsSidebarOpen] = useState(true);
  const [isMobileNav, setIsMobileNav] = useState(false);
  const [showCheckoutSuccess, setShowCheckoutSuccess] = useState(
    () => {
      if (typeof window === 'undefined') return false;
      const path = window.location.pathname.replace(/\/$/, '');
      return path.endsWith('/checkout/success');
    }
  );
  const seenTokenEventsRef = React.useRef(new Set<number>());
  const navContainerRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const container = navContainerRef.current;
    if (!container) return;
    const activeEl = container.querySelector('.nav-item.active');
    if (activeEl) {
      activeEl.scrollIntoView({ behavior: 'smooth', block: 'nearest' });
    }
  }, [activeTab]);

  useEffect(() => {
    const mq = window.matchMedia('(max-width: 768px)');
    const syncMobileNav = () => setIsMobileNav(mq.matches);
    syncMobileNav();
    mq.addEventListener('change', syncMobileNav);
    return () => mq.removeEventListener('change', syncMobileNav);
  }, []);

  useEffect(() => {
    if (window.matchMedia('(max-width: 768px)').matches) {
      setIsSidebarOpen(false);
    }
  }, []);

  useEffect(() => {
    if (isMobileNav) {
      setIsSidebarOpen(false);
    }
  }, [activeTab, isMobileNav]);

  useEffect(() => {
    const onA2uiNavigate = (event: Event) => {
      const tab = (event as CustomEvent<{ tab?: string }>).detail?.tab;
      if (typeof tab === 'string' && isValidA2uiNavTab(tab)) {
        setActiveTab(tab);
      }
    };
    window.addEventListener('a2ui-navigate', onA2uiNavigate);
    return () => window.removeEventListener('a2ui-navigate', onA2uiNavigate);
  }, []);

  useEffect(() => {
    const onUnauthorized = () => {
      clearAuthToken();
      setIsAuth(false);
    };
    window.addEventListener(AUTH_UNAUTHORIZED_EVENT, onUnauthorized);
    return () => window.removeEventListener(AUTH_UNAUTHORIZED_EVENT, onUnauthorized);
  }, []);

  const { events: vitalityEvents, lastEvent, connectionStatus, toggleConnection, lastPingMs } = useSystemVitality();

  const isConnected = connectionStatus === 'connected';

  const avatarState = useAvatarState();
  const { mode: displayMode } = useDisplayMode();
  const workspacePersona = useWorkspacePersona();

  useEffect(() => {
    fetch(`${API_BASE}/api/v1/bootstrap/status`)
      .then(res => res.json())
      .then(data => {
        setBootMode(BOOT_MODE_MAP[data.mode] ?? 'Normal');
      })
      .catch(err => {
        console.error("Failed to fetch bootstrap mode", err);
        setBootMode('Normal');
      });
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
        const d = data as ImmuneAlertEvent;
        addEvent(t('event.securityAlert'), d.description || t('event.anomalyDetected'), 'var(--accent-rose)', <Shield size={16} />);
        break;
      }
      case 'aegis_sentinel': {
        const d = data as AegisSentinelEvent;
        const color = d.level === 'Critical' ? 'var(--accent-rose)' : 'var(--accent-amber)';
        addEvent(t('event.aegisSentinel'), d.message || t('event.aegisAlert'), color, <Shield size={16} />);
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
        const d = data as InspirationEvent;
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
        addEvent(t('event.societyOfThought'), t('event.deliberationUpdate', { type: d.type }), 'var(--accent-purple)', <BrainCircuit size={16} />);
        break;
      }
      case 'token_saved': {
        const d = data as { saved_chars: number; ts: number };
        if (!seenTokenEventsRef.current.has(d.ts)) {
            seenTokenEventsRef.current.add(d.ts);
            
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
      case 'biome_evolution': {
        const d = data as BiomeEvolutionEvent;
        addEvent(
          t('event.biomeEvolution', { defaultValue: '🧬 Biome Evolution' }) as string,
          d.message || `Generation ${d.generation} reached. Rarity: ${d.rarity}`,
          'var(--accent-cyan)',
          <Activity size={16} />
        );
        break;
      }
      case 'crisis_prediction': {
        const d = data as CrisisPredictionEvent;
        addEvent(
          t('event.crisisPrediction', { defaultValue: '⚠️ Crisis Imminent' }) as string,
          d.description || `Incoming ${d.crisis_type} in ${Math.round(d.seconds_remaining / 60)} minutes!`,
          'var(--accent-rose)',
          <Shield size={16} />
        );
        break;
      }
      default:
        break;
    }
  }, [lastEvent]);

  const { viewMode } = useViewMode();
  const { agentId } = useAgentIdentity();
  const { isExpired, dismiss } = useTokenHealth();

  const isVisible = (tab: string) => {
    const simple = ['home-v2', 'agent', 'artifacts', 'settings'];
    const cockpit = [...simple, 'dashboard', 'demo', 'biome', 'cortex', 'vault', 'store', 'nurture', 'mcp-dashboard', 'seo-pulse', 'status-page', 'workflow-builder', 'karma', 'graph', 'causal', 'commune', 'audit', 'prompt-stats', 'immune', 'lora', 'expressions', 'ban-dashboard', 'buzz-approval'];

    if (viewMode === 'simple') return simple.includes(tab);
    return cockpit.includes(tab);
  };

  const isBootComplete = bootMode === 'Normal' && isAuth;

  // Pre-compute ambient particles
  const ambientParticles = useMemo(() => [...Array(6)].map((_, i) => (
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
        background: i % 2 === 0 ? 'radial-gradient(circle, var(--fluid-obsidian-glow) 0%, transparent 70%)' : 'radial-gradient(circle, rgba(var(--fluid-deep-gold-rgb), 0.04) 0%, transparent 70%)',
        borderRadius: '50%',
        filter: 'blur(50px)'
      }}
    />
  )), []);

  return (
    <div className="app-container">
      <AnimatePresence>
        {bootMode === null ? (
          <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', height: '100vh', background: 'var(--bg-base)' }}>
            <AiomeSkeleton height="40px" width="200px" />
          </div>
        ) : bootMode === 'Setup' ? (
          <React.Suspense fallback={<AiomeSkeleton height="40px" width="200px" />}>
            <SetupWizard onComplete={() => setBootMode('Normal')} />
          </React.Suspense>
        ) : !isAuth ? (
          <React.Suspense fallback={<AiomeSkeleton height="40px" width="200px" />}>
            <LoginScreen onAuthenticated={() => setIsAuth(true)} />
          </React.Suspense>
        ) : null}
      </AnimatePresence>

      {isBootComplete && (<>

      <AnimatePresence>
        {isExpired && isAuth && (
          <motion.div
            initial={{ opacity: 0, y: -50 }}
            animate={{ opacity: 1, y: 20 }}
            exit={{ opacity: 0, y: -50 }}
            style={{
              position: 'fixed',
              top: 0,
              left: '50%',
              transform: 'translateX(-50%)',
              zIndex: 10000,
              background: 'var(--accent-rose-10)',
              border: '1px solid var(--accent-rose-30)',
              borderRadius: 'var(--radius-md)',
              padding: '1rem',
              display: 'flex',
              alignItems: 'center',
              gap: '1rem',
              boxShadow: '0 10px 30px var(--black-50), 0 0 20px var(--accent-rose-10)',
              backdropFilter: 'blur(10px)'
            }}
          >
            <Shield size={20} color="var(--accent-rose)" />
            <span style={{ color: 'var(--accent-rose)', fontWeight: 600, fontSize: '0.9rem' }}>
              {t('session.expired')}
            </span>
            <button
               onClick={() => { setActiveTab("settings"); dismiss(); }}
               style={{
                 background: 'var(--accent-rose)', color: 'var(--bg-primary)',
                 border: 'none', padding: '0.4rem 0.8rem', borderRadius: '6px',
                 fontWeight: 700, cursor: 'pointer', fontSize: '0.8rem'
               }}
            >
               {t('session.goToSettings')}
            </button>
          </motion.div>
        )}
      </AnimatePresence>

      {/* Digital Diorama — Resident Avatar */}
      <React.Suspense fallback={null}>
        <DioramaView status={avatarState} mode={displayMode} activeTab={activeTab} />
      </React.Suspense>
      
      {/* Society of Thought Visualization */}
      <SoTProgressBar />

      {/* Ambient Background Particles */}
      {isBootComplete && (
        <div style={{ position: 'fixed', inset: 0, pointerEvents: 'none', zIndex: 0, overflow: 'hidden' }}>
          {ambientParticles}
        </div>
      )}

      {/* Sidebar */}
      <AppSidebar
        viewMode={viewMode}
        isMobileNav={isMobileNav}
        isSidebarOpen={isSidebarOpen}
        setIsSidebarOpen={setIsSidebarOpen}
        workspacePersona={workspacePersona}
        isVisible={isVisible}
        activeTab={activeTab}
        setActiveTab={setActiveTab}
        t={t}
        navContainerRef={navContainerRef}
        stats={stats}
        lang={lang}
        setLang={setLang}
        APP_VERSION={APP_VERSION}
      />

      {/* Main Content */}
      <main className="main-content">
        <AppHeader
          isMobileNav={isMobileNav}
          viewMode={viewMode}
          isSidebarOpen={isSidebarOpen}
          setIsSidebarOpen={setIsSidebarOpen}
          activeTab={activeTab}
          t={t}
          connectionStatus={connectionStatus}
          lastPingMs={lastPingMs}
          toggleConnection={toggleConnection}
          workspacePersona={workspacePersona}
        />

        <AnimatePresence mode="wait">
          <React.Suspense fallback={
            <div style={{ padding: 'var(--space-lg)', display: 'grid', gap: 'var(--space-lg)', height: '100%' }}>
              <AiomeSkeleton height="40px" width="30%" />
              <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(300px, 1fr))', gap: 'var(--space-md)' }}>
                <AiomeSkeleton height="150px" />
                <AiomeSkeleton height="150px" />
                <AiomeSkeleton height="150px" />
              </div>
              <AiomeSkeleton height="300px" />
            </div>
          }>
            <motion.div
              key={activeTab}
              initial={{ opacity: 0, y: 10 }}
              animate={{ opacity: 1, y: 0 }}
              exit={{ opacity: 0, y: -10 }}
              transition={{ duration: 0.2 }}
              style={{ display: 'flex', flexDirection: 'column', flex: 1, minHeight: 0 }}
            >
              {showCheckoutSuccess ? (
                <CheckoutSuccess
                  onGoHome={() => {
                    setShowCheckoutSuccess(false);
                    window.history.replaceState({}, '', '/');
                    setActiveTab('home-v2');
                  }}
                />
              ) : (
                <AppRoutes
                  activeTab={activeTab}
                  setActiveTab={setActiveTab}
                  stats={stats}
                  vitalityEvents={vitalityEvents}
                  connectionStatus={connectionStatus}
                  recentEvents={recentEvents}
                  lastEvent={lastEvent}
                  sessionSavedChars={sessionSavedChars}
                  isConnected={isConnected}
                />
              )}
            </motion.div>
          </React.Suspense>
        </AnimatePresence>
      </main>

      <React.Suspense fallback={null}>
        <TaskApprovalOverlay />
        <ProUpgradeModal priceId={STRIPE_PRICE_ID} agentId={agentId ?? undefined} />
      </React.Suspense>
      </>)}
    </div>
  );
}

export default App;

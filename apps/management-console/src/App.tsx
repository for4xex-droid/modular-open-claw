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
  Library,
  Server,
  Briefcase,
  Home,
  LayoutDashboard,
  GitCommit,
  TrendingUp,
  ClipboardList,
  Coins,
  BarChart2,
  PanelLeftClose,
  PanelLeftOpen
} from "lucide-react";
const LoginScreen = React.lazy(() => import("./components/LoginScreen"));
const SetupWizard = React.lazy(() => import("./components/SetupWizard"));
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
const StatusPage = React.lazy(() => import("./components/StatusPage"));
const ExpressionPipeline = React.lazy(() => import("./components/ExpressionPipeline"));
const LoraTrainingView = React.lazy(() => import("./components/LoraTrainingView"));
const CommuneDialogueView = React.lazy(() => import("./components/CommuneDialogueView"));
const VoiceStore = React.lazy(() => import("./components/VoiceStore"));
const McpDashboard = React.lazy(() => import("./components/McpDashboard"));
const BanDashboard = React.lazy(() => import("./components/BanDashboard"));
const DemoView = React.lazy(() => import("./components/DemoView"));
const CausalVisualizer = React.lazy(() => import("./components/CausalVisualizer"));
const CortexView = React.lazy(() => import("./components/cortex/CortexView"));
const NurtureDashboard = React.lazy(() => import("./components/commerce/NurtureDashboard"));
const BuzzApproval = React.lazy(() => import("./components/BuzzApproval"));
const WorkflowBuilder = React.lazy(() => import("./components/WorkflowBuilder"));
import DioramaView from "./components/diorama/DioramaView";
const TaskApprovalOverlay = React.lazy(() => import("./components/TaskApprovalOverlay"));
import { SoTProgressBar } from "./components/SoTProgressBar";
import { useWorkspacePersona } from "./hooks/useWorkspacePersona";
import { AiaaOnboardingWizard } from "./components/AiaaOnboardingWizard";

import { isAuthenticated } from "./lib/auth";
import { useAvatarState } from "./hooks/useAvatarState";
import { AiomeSkeleton } from "./components/common/AiomeSkeleton";
import { useDisplayMode } from "./hooks/useDisplayMode";
import { AgentStats, VitalityUIEvent, Karma, SoTEvent, ImmuneAlertEvent, AegisSentinelEvent, InspirationEvent, BiomeEvolutionEvent, CrisisPredictionEvent } from "./types";
import { useSystemVitality } from "./hooks/useSystemVitality";
import { useViewMode } from "./hooks/useViewMode";
import { useTokenHealth } from "./hooks/useTokenHealth";
import { APP_VERSION, API_BASE } from "./config";

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
      <div style={{ display: 'flex', gap: '0.5rem' }}>
        <div className="status-item persona-toggle" onClick={() => workspacePersona.setMode(workspacePersona.mode === 'agency' ? 'consumer' : 'agency')} style={{ cursor: 'pointer', display: 'flex', alignItems: 'center', gap: '0.5rem', padding: '0.5rem', background: 'var(--black-40)', borderRadius: '6px' }} data-tooltip={workspacePersona.mode === 'agency' ? t('persona.agencyTooltip') : t('persona.consumerTooltip')}>
          <Briefcase size={14} color={workspacePersona.mode === 'agency' ? 'var(--accent-cyan)' : 'var(--text-secondary)'} />
          <span>{workspacePersona.mode === 'agency' ? t('persona.agencyMode') : t('persona.consumerMode')}</span>
        </div>
        <button
          className={badgeClass}
          onClick={toggleConnection}
          style={{
            cursor: 'pointer', border: '1px solid var(--white-05)', background: 'var(--black-40)',
            outline: 'none', transition: 'all 0.2s', padding: '0.5rem 1rem'
          }}
          data-tooltip="Click to toggle connection sync"
        >
          <div className={dotClass} style={{
            background: connectionStatus === 'paused' ? 'var(--accent-amber)' : undefined,
            boxShadow: connectionStatus === 'paused' ? 'var(--glow-amber)' : undefined
          }} />
          {text}
        </button>
      </div>
    );
  };

  const { viewMode } = useViewMode();
  const { isExpired, dismiss } = useTokenHealth();

  const isVisible = (tab: string) => {
    const beginner = ['home-v2', 'agent', 'artifacts', 'settings'];
    const intermediate = [...beginner, 'dashboard', 'demo', 'cortex', 'vault', 'store', 'nurture', 'mcp-dashboard', 'seo-pulse', 'status-page', 'workflow-builder'];
    const advanced = [...intermediate, 'karma', 'graph', 'causal', 'commune', 'audit', 'prompt-stats', 'immune', 'lora', 'expressions', 'ban-dashboard'];
    
    if (viewMode === 'beginner') return beginner.includes(tab);
    if (viewMode === 'intermediate') return intermediate.includes(tab);
    return advanced.includes(tab);
  };

  const isBootComplete = bootMode === 'Normal' && isAuth;

  // Pre-compute ambient particles (must be unconditional — Rules of Hooks)
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
          <React.Suspense fallback={<div />}>
            <SetupWizard onComplete={() => setBootMode('Normal')} />
          </React.Suspense>
        ) : !isAuth ? (
          <React.Suspense fallback={<div />}>
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
      <DioramaView status={avatarState} mode={displayMode} activeTab={activeTab} />
      
      {/* Society of Thought Visualization */}
      <SoTProgressBar />

      {/* Ambient Background Particles — 認証後のみ表示（LoginScreen は FluidBackground を使用） */}
      {isBootComplete && (
        <div style={{ position: 'fixed', inset: 0, pointerEvents: 'none', zIndex: 0, overflow: 'hidden' }}>
          {ambientParticles}
        </div>
      )}

      {/* Sidebar — advanced mode only */}
      {viewMode === 'advanced' && <aside className={`sidebar ${isSidebarOpen ? '' : 'closed'}`}>
        <div className="brand">
          <BrainCircuit size={28} color="var(--accent-cyan)" />
          <span>Aiome</span>
        </div>
        <div className="sidebar-toggle-container">
          <button 
            className="sidebar-toggle-btn"
            onClick={() => setIsSidebarOpen(!isSidebarOpen)}
            data-tooltip="Toggle Sidebar"
          >
            {isSidebarOpen ? <PanelLeftClose size={20} /> : <PanelLeftOpen size={20} />}
          </button>
        </div>

        <div className="sidebar-nav-container" ref={navContainerRef}>
          <nav className="nav-group">
          <h4>{t('nav.section.synergyHub')}</h4>
          {isVisible("home-v2") && (
            <NavItem
              icon={<Home size={18} />}
              label={t('nav.homeV2')}
              active={activeTab === "home-v2"}
              onClick={() => setActiveTab("home-v2")}
            />
          )}
          {workspacePersona.mode === 'agency' && (
            <NavItem
              icon={<Briefcase size={18} />}
              label={t('nav.agencyOnboarding')}
              active={activeTab === "agency"}
              onClick={() => setActiveTab("agency")}
            />
          )}
          {isVisible("dashboard") && (
            <NavItem
              icon={<LayoutDashboard size={20} />}
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
              icon={<GitCommit size={20} />}
              label={t('nav.causalTrace')}
              active={activeTab === "causal"}
              onClick={() => setActiveTab("causal")}
            />
          )}
          {isVisible("seo-pulse") && (
            <NavItem
              icon={<TrendingUp size={20} />}
              label={t('nav.seoPulse')}
              active={activeTab === "seo-pulse"}
              onClick={() => setActiveTab("seo-pulse")}
            />
          )}
          {isVisible("buzz-approval") && (
            <NavItem
              icon={<Zap size={20} />}
              label={t('nav.buzzApproval')}
              active={activeTab === "buzz-approval"}
              onClick={() => setActiveTab("buzz-approval")}
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
              icon={<ClipboardList size={20} />}
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
          {isVisible("commune") && (
            <NavItem
              icon={<Network size={20} />}
              label={t('nav.communeLab')}
              active={activeTab === "commune"}
              onClick={() => setActiveTab("commune")}
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
          {isVisible("nurture") && (
            <NavItem
              icon={<Coins size={20} />}
              label={t('nav.nurtureEconomy')}
              active={activeTab === "nurture"}
              onClick={() => setActiveTab("nurture")}
            />
          )}
          {isVisible("workflow-builder") && (
            <NavItem
              icon={<Network size={20} />}
              label={t('nav.workflowBuilder')}
              active={activeTab === "workflow-builder"}
              onClick={() => setActiveTab("workflow-builder")}
            />
          )}
        </nav>

        <nav className="nav-group">
          <h4>{t('nav.section.control')}</h4>
          {isVisible("status-page") && (
            <NavItem
              icon={<Shield size={20} />}
              label={t('nav.statusPage')}
              active={activeTab === "status-page"}
              onClick={() => setActiveTab("status-page")}
            />
          )}
          {isVisible("ban-dashboard") && (
            <NavItem
              icon={<Shield size={20} />}
              label={t('nav.banDashboard')}
              active={activeTab === "ban-dashboard"}
              onClick={() => setActiveTab("ban-dashboard")}
            />
          )}
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
          {isVisible("mcp-dashboard") && (
            <NavItem
              icon={<Server size={20} />}
              label={t('nav.mcpDashboard')}
              active={activeTab === "mcp-dashboard"}
              onClick={() => setActiveTab("mcp-dashboard")}
            />
          )}
          {isVisible("prompt-stats") && (
            <NavItem
              icon={<BarChart2 size={20} />}
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
        </div>

        <AnimatePresence>
          {isSidebarOpen && (
            <motion.div
              className="sidebar-footer"
              initial={{ opacity: 0, y: 10 }}
              animate={{ opacity: 1, y: 0 }}
              exit={{ opacity: 0, y: 10 }}
              transition={{ duration: 0.2 }}
            >
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
                AIOME {APP_VERSION}
              </div>
              <div style={{ display: 'flex', justifyContent: 'center', gap: '0.25rem', marginTop: '0.75rem' }}>
                <button className={`lang-btn ${lang === 'en' ? 'active' : ''}`} onClick={() => setLang('en')}>
                  🇺🇸 {t('language.en')}
                </button>
                <button className={`lang-btn ${lang === 'ja' ? 'active' : ''}`} onClick={() => setLang('ja')}>
                  🇯🇵 {t('language.ja')}
                </button>
              </div>
            </motion.div>
          )}
        </AnimatePresence>
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
            {activeTab === "seo-pulse" && t('page.seoPulse')}
            {activeTab === "cortex" && t('page.cortex')}
            {activeTab === "vault" && t('page.skillVault')}
            {activeTab === "artifacts" && t('page.artifactVault')}
            {activeTab === "audit" && t('page.audit')}
            {activeTab === "prompt-stats" && t('page.promptStats')}
            {activeTab === "mcp-dashboard" && t('page.mcpDashboard')}
            {activeTab === "expressions" && t('page.expressions')}
            {activeTab === "commune" && t('page.communeLab')}
            {activeTab === "store" && t('page.voiceStore')}
            {activeTab === "ban-dashboard" && t('page.banDashboard')}
            {activeTab === "nurture" && t('page.nurtureEconomy')}
            {activeTab === "workflow-builder" && t('page.workflowBuilder')}
            {activeTab === "causal" && t('page.causalTrace')}
            {activeTab === "lora" && t('page.loraAutotuner')}
            {activeTab === "settings" && t('page.settings')}
            {activeTab === "agency" && t('page.agencyOnboarding')}
            {activeTab === "status-page" && t('page.statusPage')}
          </motion.h2>

          <div style={{ display: 'flex', gap: '1rem', alignItems: 'center' }}>
            {renderStatusBadge()}
          </div>
        </header>

        <AnimatePresence mode="wait">
          {/* Use Suspense for lazy loaded components */}
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
              {activeTab === "home-v2" && <HomePage stats={stats} vitalityEvents={vitalityEvents} connectionStatus={connectionStatus} recentEvents={recentEvents} lastEvent={lastEvent} sessionSavedChars={sessionSavedChars} />}
              {activeTab === "dashboard" && <BiotopeView stats={stats} isConnected={isConnected} recentEvents={recentEvents} sessionSavedChars={sessionSavedChars} />}
              {activeTab === "demo" && <DemoView stats={stats} lastEvent={lastEvent} isConnected={isConnected} />}
              {activeTab === "karma" && <Timeline />}
              {activeTab === "graph" && <GraphView />}
              {activeTab === "immune" && <ImmuneSystem />}
              {activeTab === "agent" && <AgentConsole sessionSavedChars={sessionSavedChars} />}
              {activeTab === "seo-pulse" && <SeoPulseView />}
              {activeTab === "cortex" && <CortexView />}
              {activeTab === "vault" && <SkillVault />}
              {activeTab === "artifacts" && <ArtifactVault />}
              {activeTab === "audit" && <DiagnosticsHistory />}
              {activeTab === "prompt-stats" && <PromptStatsView />}
              {activeTab === "mcp-dashboard" && <McpDashboard />}
              {activeTab === "expressions" && <ExpressionPipeline />}
              {activeTab === "commune" && <CommuneDialogueView />}
              {activeTab === "store" && <VoiceStore />}
              {activeTab === "ban-dashboard" && <BanDashboard />}
              {activeTab === "nurture" && <NurtureDashboard />}
              {activeTab === "workflow-builder" && <WorkflowBuilder />}
              {activeTab === "buzz-approval" && <BuzzApproval />}
              {activeTab === "causal" && <CausalVisualizer />}
              {activeTab === "lora" && <LoraTrainingView />}
              {activeTab === "settings" && <SettingsPage />}
              {activeTab === "agency" && <AiaaOnboardingWizard />}
              {activeTab === "status-page" && <StatusPage />}
            </motion.div>
          </React.Suspense>
        </AnimatePresence>
      </main>



      <React.Suspense fallback={null}>
        <TaskApprovalOverlay />
      </React.Suspense>
      </>)}
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

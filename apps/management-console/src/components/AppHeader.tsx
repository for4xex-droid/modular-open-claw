/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import { motion } from "framer-motion";
import { PanelLeftOpen } from "lucide-react";
import { CoinChip } from "./commerce/CoinChip";
import { PlanBadge } from "./commerce/PlanBadge";
import { StatusBadge } from "./StatusBadge";
import type { WorkspacePersona } from "../hooks/useWorkspacePersona";

interface AppHeaderProps {
  isMobileNav: boolean;
  viewMode: string;
  isSidebarOpen: boolean;
  setIsSidebarOpen: (open: boolean) => void;
  activeTab: string;
  t: (key: string, options?: any) => string | any;
  connectionStatus: string;
  lastPingMs: number | null;
  toggleConnection: () => void;
  workspacePersona: WorkspacePersona;
}

export function AppHeader({
  isMobileNav,
  viewMode,
  isSidebarOpen,
  setIsSidebarOpen,
  activeTab,
  t,
  connectionStatus,
  lastPingMs,
  toggleConnection,
  workspacePersona
}: AppHeaderProps) {
  return (
    <header className="header">
      {isMobileNav && viewMode === 'cockpit' && !isSidebarOpen && (
        <button
          type="button"
          className="mobile-menu-btn"
          aria-label={t('sidebar.openMenu')}
          onClick={() => setIsSidebarOpen(true)}
        >
          <PanelLeftOpen size={20} />
        </button>
      )}
      <div className="header-title-block">
        <motion.h2
          initial={{ opacity: 0, x: -20 }}
          animate={{ opacity: 1, x: 0 }}
          key={activeTab}
        >
          {activeTab === "home-v2" && t('page.homeV2')}
          {activeTab === "dashboard" && t('page.biotope')}
          {activeTab === "demo" && t('page.demo')}
          {activeTab === "biome" && t('page.biome')}
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
          {activeTab === "buzz-approval" && t('page.buzzApproval')}
        </motion.h2>
        <motion.p
          className="page-desc"
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          key={`desc-${activeTab}`}
        >
          {t(`page.desc.${activeTab}`)}
        </motion.p>
      </div>

      <div style={{ display: 'flex', gap: '1rem', alignItems: 'center' }}>
        <CoinChip />
        <PlanBadge />
        <StatusBadge
          connectionStatus={connectionStatus}
          lastPingMs={lastPingMs}
          toggleConnection={toggleConnection}
          workspacePersona={workspacePersona}
          t={t}
        />
      </div>
    </header>
  );
}

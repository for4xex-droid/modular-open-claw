/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import React from "react";
import { AgentStats, VitalityUIEvent } from "./types";

const HomePage = React.lazy(() => import("./components/home/HomePage"));
const BiotopeView = React.lazy(() => import("./components/BiotopeView"));
const DemoView = React.lazy(() => import("./components/DemoView"));
const BiomeGame = React.lazy(() => import("./lib/biome/BiomeGame").then(m => ({ default: m.BiomeGame })));
const ActivityView = React.lazy(() => import("./components/ActivityView"));
const GraphView = React.lazy(() => import("./components/GraphView"));
const ImmuneSystem = React.lazy(() => import("./components/ImmuneSystem"));
const AgentConsole = React.lazy(() => import("./components/AgentConsole"));
const SeoPulseView = React.lazy(() => import("./components/SeoPulseView"));
const CortexView = React.lazy(() => import("./components/cortex/CortexView"));
const SkillVault = React.lazy(() => import("./components/SkillVault"));
const ArtifactVault = React.lazy(() => import("./components/ArtifactVault"));
const McpDashboard = React.lazy(() => import("./components/McpDashboard"));
const ExpressionPipeline = React.lazy(() => import("./components/ExpressionPipeline"));
const CommuneDialogueView = React.lazy(() => import("./components/CommuneDialogueView"));
const VoiceStore = React.lazy(() => import("./components/VoiceStore"));
const BanDashboard = React.lazy(() => import("./components/BanDashboard"));
const NurtureDashboard = React.lazy(() => import("./components/commerce/NurtureDashboard"));
const WorkflowBuilder = React.lazy(() => import("./components/WorkflowBuilder"));
const BuzzApproval = React.lazy(() => import("./components/BuzzApproval"));
const CausalVisualizer = React.lazy(() => import("./components/CausalVisualizer"));
const LoraTrainingView = React.lazy(() => import("./components/LoraTrainingView"));
const SettingsPage = React.lazy(() => import("./components/SettingsPage"));
const AiaaOnboardingWizard = React.lazy(() =>
  import("./components/AiaaOnboardingWizard").then((m) => ({ default: m.AiaaOnboardingWizard }))
);
const StatusPage = React.lazy(() => import("./components/StatusPage"));

interface AppRoutesProps {
  activeTab: string;
  setActiveTab: (tab: string) => void;
  stats: AgentStats;
  vitalityEvents: any[];
  connectionStatus: string;
  recentEvents: VitalityUIEvent[];
  lastEvent: any;
  sessionSavedChars: number;
  isConnected: boolean;
}

export function AppRoutes({
  activeTab,
  setActiveTab,
  stats,
  vitalityEvents,
  connectionStatus,
  recentEvents,
  lastEvent,
  sessionSavedChars,
  isConnected
}: AppRoutesProps) {
  return (
    <>
      {activeTab === "home-v2" && <HomePage stats={stats} vitalityEvents={vitalityEvents} connectionStatus={connectionStatus} recentEvents={recentEvents} lastEvent={lastEvent} sessionSavedChars={sessionSavedChars} />}
      {activeTab === "dashboard" && <BiotopeView stats={stats} isConnected={isConnected} recentEvents={recentEvents} sessionSavedChars={sessionSavedChars} />}
      {activeTab === "demo" && <DemoView stats={stats} lastEvent={lastEvent} isConnected={isConnected} />}
      {activeTab === "biome" && <BiomeGame />}
      {activeTab === "karma" && <ActivityView initialTab="timeline" />}
      {activeTab === "graph" && <GraphView />}
      {activeTab === "immune" && <ImmuneSystem />}
      {activeTab === "agent" && <AgentConsole sessionSavedChars={sessionSavedChars} />}
      {activeTab === "seo-pulse" && <SeoPulseView />}
      {activeTab === "cortex" && <CortexView />}
      {activeTab === "vault" && <SkillVault />}
      {activeTab === "artifacts" && <ArtifactVault />}
      {activeTab === "audit" && <ActivityView initialTab="audit" />}
      {activeTab === "prompt-stats" && <ActivityView initialTab="usage" />}
      {activeTab === "mcp-dashboard" && <McpDashboard />}
      {activeTab === "expressions" && <ExpressionPipeline />}
      {activeTab === "commune" && <CommuneDialogueView />}
      {activeTab === "store" && <VoiceStore />}
      {activeTab === "ban-dashboard" && <BanDashboard />}
      {activeTab === "nurture" && <NurtureDashboard onNavigateToStore={() => setActiveTab('store')} />}
      {activeTab === "workflow-builder" && <WorkflowBuilder />}
      {activeTab === "buzz-approval" && <BuzzApproval />}
      {activeTab === "causal" && <CausalVisualizer />}
      {activeTab === "lora" && <LoraTrainingView />}
      {activeTab === "settings" && <SettingsPage />}
      {activeTab === "agency" && <AiaaOnboardingWizard />}
      {activeTab === "status-page" && <StatusPage />}
    </>
  );
}

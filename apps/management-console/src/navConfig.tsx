/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import React from "react";
import {
  Home,
  MessageSquare,
  Briefcase,
  Gamepad2,
  BrainCircuit,
  Package,
  Library,
  Sparkles,
  LayoutDashboard,
  Clock,
  GitMerge,
  GitCommit,
  Shield,
  Coins,
  Crown,
  Zap,
  TrendingUp,
  Network,
  Server,
  Box,
  Settings as SettingsIcon
} from "lucide-react";

export interface NavItemDef {
  tab: string;
  labelKey: string;
  icon: React.ReactNode;
}

export interface NavGroupDef {
  sectionKey: string;
  items: NavItemDef[];
}

export const NAV_GROUPS: NavGroupDef[] = [
  {
    sectionKey: 'home',
    items: [
      { tab: 'home-v2', labelKey: 'nav.homeV2', icon: <Home size={18} /> },
      { tab: 'agent', labelKey: 'nav.agentConsole', icon: <MessageSquare size={18} /> },
      { tab: 'agency', labelKey: 'nav.agencyOnboarding', icon: <Briefcase size={18} /> },
    ],
  },
  {
    sectionKey: 'grow',
    items: [
      { tab: 'biome', labelKey: 'nav.biome', icon: <Gamepad2 size={18} /> },
      { tab: 'lora', labelKey: 'nav.loraAutotuner', icon: <BrainCircuit size={18} /> },
      { tab: 'vault', labelKey: 'nav.skillVault', icon: <Package size={18} /> },
      { tab: 'cortex', labelKey: 'nav.cortex', icon: <Library size={18} /> },
      { tab: 'expressions', labelKey: 'nav.expressions', icon: <Sparkles size={18} /> },
    ],
  },
  {
    sectionKey: 'observe',
    items: [
      { tab: 'dashboard', labelKey: 'nav.biotope', icon: <LayoutDashboard size={18} /> },
      { tab: 'karma', labelKey: 'nav.chronicle', icon: <Clock size={18} /> },
      { tab: 'graph', labelKey: 'nav.resonanceMap', icon: <GitMerge size={18} /> },
      { tab: 'causal', labelKey: 'nav.causalTrace', icon: <GitCommit size={18} /> },
      { tab: 'status-page', labelKey: 'nav.statusPage', icon: <Shield size={18} /> },
    ],
  },
  {
    sectionKey: 'expand',
    items: [
      { tab: 'nurture', labelKey: 'nav.nurtureEconomy', icon: <Coins size={18} /> },
      { tab: 'store', labelKey: 'nav.voiceStore', icon: <Crown size={18} /> },
      { tab: 'buzz-approval', labelKey: 'nav.buzzApproval', icon: <Zap size={18} /> },
      { tab: 'seo-pulse', labelKey: 'nav.seoPulse', icon: <TrendingUp size={18} /> },
      { tab: 'commune', labelKey: 'nav.communeLab', icon: <Network size={18} /> },
      { tab: 'workflow-builder', labelKey: 'nav.workflowBuilder', icon: <Network size={18} /> },
      { tab: 'mcp-dashboard', labelKey: 'nav.mcpDashboard', icon: <Server size={18} /> },
      { tab: 'artifacts', labelKey: 'nav.artifactVault', icon: <Box size={18} /> },
    ],
  },
  {
    sectionKey: 'protect',
    items: [
      { tab: 'immune', labelKey: 'nav.immuneSystem', icon: <Shield size={18} /> },
      { tab: 'ban-dashboard', labelKey: 'nav.banDashboard', icon: <Shield size={18} /> },
      { tab: 'settings', labelKey: 'nav.settings', icon: <SettingsIcon size={18} /> },
    ],
  },
];

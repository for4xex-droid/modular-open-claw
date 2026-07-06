/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import React, { createContext, useContext, ReactNode } from 'react';
import { useAgentChatState, UseAgentChatReturn } from './useAgentChat';

const AgentChatContext = createContext<UseAgentChatReturn | null>(null);

export const AgentChatProvider: React.FC<{ children: ReactNode }> = ({ children }) => {
  const value = useAgentChatState();
  return <AgentChatContext.Provider value={value}>{children}</AgentChatContext.Provider>;
};

export const useAgentChat = (): UseAgentChatReturn => {
  const ctx = useContext(AgentChatContext);
  if (!ctx) {
    throw new Error('useAgentChat must be used within AgentChatProvider');
  }
  return ctx;
};

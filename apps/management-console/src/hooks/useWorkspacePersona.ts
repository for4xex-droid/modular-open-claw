/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

import { useState, useEffect } from 'react';

export type WorkspaceMode = 'consumer' | 'agency';

export interface WorkspacePersona {
  mode: WorkspaceMode;
  setMode: (mode: WorkspaceMode) => void;
  isAgency: boolean;
}

export function useWorkspacePersona(): WorkspacePersona {
  const [mode, setModeState] = useState<WorkspaceMode>('consumer');

  useEffect(() => {
    const saved = localStorage.getItem('aiome_workspace_mode');
    if (saved === 'agency' || saved === 'consumer') {
      setModeState(saved);
    }
  }, []);

  const setMode = (newMode: WorkspaceMode) => {
    setModeState(newMode);
    localStorage.setItem('aiome_workspace_mode', newMode);
  };

  return {
    mode,
    setMode,
    isAgency: mode === 'agency',
  };
}

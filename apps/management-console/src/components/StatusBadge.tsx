/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import { Briefcase } from "lucide-react";
import type { WorkspacePersona } from "../hooks/useWorkspacePersona";

interface StatusBadgeProps {
  connectionStatus: string;
  lastPingMs: number | null;
  toggleConnection: () => void;
  workspacePersona: WorkspacePersona;
  t: (key: string, options?: any) => string | any;
}

export function StatusBadge({
  connectionStatus,
  lastPingMs,
  toggleConnection,
  workspacePersona,
  t
}: StatusBadgeProps) {
  let badgeClass = "status-badge";
  let dotClass = "status-dot";
  let text = "";

  switch (connectionStatus) {
    case "connected":
      text = lastPingMs !== null ? t('status.connectedMs', { ms: lastPingMs }) : t('status.hubConnected');
      break;
    case "connecting":
      badgeClass += ' disconnected';
      dotClass += ' offline';
      dotClass += ' ani-pulse';
      text = t('status.reconnecting');
      break;
    case "paused":
      badgeClass += ' paused';
      dotClass += ' offline';
      dotClass = dotClass.replace('offline', 'paused');
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
      <div
        className="status-item persona-toggle"
        onClick={() => workspacePersona.setMode(workspacePersona.mode === 'agency' ? 'consumer' : 'agency')}
        style={{ cursor: 'pointer', display: 'flex', alignItems: 'center', gap: '0.5rem', padding: '0.5rem', background: 'var(--black-40)', borderRadius: '6px' }}
        data-tooltip={workspacePersona.mode === 'agency' ? t('persona.agencyTooltip') : t('persona.consumerTooltip')}
      >
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
        <div
          className={dotClass}
          style={{
            background: connectionStatus === 'paused' ? 'var(--accent-amber)' : undefined,
            boxShadow: connectionStatus === 'paused' ? 'var(--glow-amber)' : undefined
          }}
        />
        {text}
      </button>
    </div>
  );
}

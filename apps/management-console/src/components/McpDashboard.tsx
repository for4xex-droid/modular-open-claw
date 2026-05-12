/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import { useState, useEffect, useCallback } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { Server, Plus, Trash2, Save, RefreshCw, Activity, Terminal, AlertTriangle, Search } from 'lucide-react';
import { useTranslation } from '../i18n';
import { API_BASE } from '../config';
import { authenticatedFetch } from '../lib/auth';
import { ActivityFeed } from './common/ActivityFeed';
import ConfirmModal from './common/ConfirmModal';

interface McpServerConfig {
  transport?: 'stdio' | 'http';
  command: string;
  args: string[];
  env?: Record<string, string>;
  url?: string;
  headers?: Record<string, string>;
  disabled?: boolean;
}

interface McpDiscoveryFile {
  mcp_servers: Record<string, McpServerConfig>;
}

type McpTransportType = 'stdio' | 'http';

/** Validates that a server ID contains only safe characters */
function isValidServerId(id: string): boolean {
  return /^[a-zA-Z0-9_-]{1,64}$/.test(id);
}

/** Validates that a URL uses an allowed scheme */
function isValidMcpUrl(url: string): boolean {
  try {
    const parsed = new URL(url);
    return parsed.protocol === 'http:' || parsed.protocol === 'https:';
  } catch {
    return false;
  }
}

const DEFAULT_SERVER_CONFIG: McpServerConfig = {
  transport: 'stdio',
  command: '',
  args: [],
  env: {},
  url: '',
  headers: {}
};

export default function McpDashboard() {
  const { t } = useTranslation();
  const [config, setConfig] = useState<McpDiscoveryFile>({ mcp_servers: {} });
  const [isLoading, setIsLoading] = useState(true);
  const [isSaving, setIsSaving] = useState(false);
  const [editingKey, setEditingKey] = useState<string | null>(null);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [validationError, setValidationError] = useState<string | null>(null);
  const [enablingServerId, setEnablingServerId] = useState<string | null>(null);
  const [activeMcpSkills, setActiveMcpSkills] = useState<Record<string, any>>({});

  // New server form state
  const [newServerId, setNewServerId] = useState('');
  const [newServerConfig, setNewServerConfig] = useState<McpServerConfig>({ ...DEFAULT_SERVER_CONFIG });
  
  // Confirm modal state
  const [removingServerId, setRemovingServerId] = useState<string | null>(null);
  
  // Search state
  const [searchTerm, setSearchTerm] = useState('');

  const loadConfig = useCallback(async () => {
    setIsLoading(true);
    setLoadError(null);
    try {
      const [resConfig, resSkills] = await Promise.all([
        authenticatedFetch(`${API_BASE}/api/skills/mcp/config`),
        authenticatedFetch(`${API_BASE}/api/skills`)
      ]);
      if (resConfig.ok) {
        const data = await resConfig.json();
        setConfig(data);
      } else {
        setLoadError(t('mcp.loadFailed', { defaultValue: 'Failed to load MCP configuration' }) as string);
      }
      if (resSkills.ok) {
        const skillsData = await resSkills.json();
        if (Array.isArray(skillsData)) {
          const activeMcp = skillsData.filter((s: any) => s.source === 'mcp').reduce((acc: any, s: any) => {
            acc[s.name] = s;
            return acc;
          }, {});
          setActiveMcpSkills(activeMcp);
        }
      }
    } catch {
      setLoadError(t('mcp.connectionError', { defaultValue: 'Connection error. Is the API server running?' }) as string);
    } finally {
      setIsLoading(false);
    }
  }, [t]);

  useEffect(() => {
    loadConfig();
  }, [loadConfig]);

  const handleSaveConfig = async (newConfig: McpDiscoveryFile): Promise<boolean> => {
    setIsSaving(true);
    try {
      const res = await authenticatedFetch(`${API_BASE}/api/skills/mcp/config`, {
        method: 'POST',
        body: JSON.stringify(newConfig)
      });
      if (res.ok) {
        setConfig(newConfig);
        setEditingKey(null);
        setNewServerId('');
        setNewServerConfig({ ...DEFAULT_SERVER_CONFIG });
        setValidationError(null);
        return true;
      } else {
        const errorText = await res.text();
        setValidationError(t('mcp.saveFailed', { defaultValue: `Failed to save: ${errorText}` }) as string);
        return false;
      }
    } catch {
      setValidationError(t('mcp.saveConnectionError', { defaultValue: 'Connection error while saving.' }) as string);
      return false;
    } finally {
      setIsSaving(false);
    }
  };

  const executeRemoveServer = async () => {
    if (!removingServerId) return;
    const newConfig = { ...config, mcp_servers: { ...config.mcp_servers } };
    delete newConfig.mcp_servers[removingServerId];
    const success = await handleSaveConfig(newConfig);
    if (success) {
      setRemovingServerId(null);
    }
  };

  const handleRemoveServer = (id: string) => {
    setRemovingServerId(id);
  };

  const handleEnableServer = async () => {
    if (!enablingServerId) return;
    const serverToEnable = config.mcp_servers[enablingServerId];
    if (!serverToEnable) return;
    
    const newConfig = {
      ...config,
      mcp_servers: {
        ...config.mcp_servers,
        [enablingServerId]: {
          ...serverToEnable,
          disabled: false
        }
      }
    };
    const success = await handleSaveConfig(newConfig);
    if (success) {
      setEnablingServerId(null);
    }
  };

  const handleAddServer = async () => {
    setValidationError(null);
    const trimmedId = newServerId.trim();

    if (!trimmedId) {
      setValidationError(t('mcp.errorIdRequired', { defaultValue: 'Server ID is required.' }) as string);
      return;
    }
    if (!isValidServerId(trimmedId)) {
      setValidationError(t('mcp.errorIdInvalid', { defaultValue: 'Server ID must contain only letters, numbers, hyphens, and underscores (max 64 chars).' }) as string);
      return;
    }
    if (config.mcp_servers[trimmedId]) {
      setValidationError(t('mcp.errorIdExists', { defaultValue: 'A server with this ID already exists.' }) as string);
      return;
    }

    if (newServerConfig.transport === 'http') {
      if (!newServerConfig.url || !isValidMcpUrl(newServerConfig.url)) {
        setValidationError(t('mcp.errorUrlInvalid', { defaultValue: 'A valid HTTP or HTTPS URL is required for HTTP transport.' }) as string);
        return;
      }
    } else {
      if (!newServerConfig.command.trim()) {
        setValidationError(t('mcp.errorCommandRequired', { defaultValue: 'Command is required for STDIO transport.' }) as string);
        return;
      }
    }

    const newConfig = {
      ...config,
      mcp_servers: {
        ...config.mcp_servers,
        [trimmedId]: newServerConfig
      }
    };
    await handleSaveConfig(newConfig);
  };

  const handleTransportChange = (value: string) => {
    setNewServerConfig({ ...newServerConfig, transport: value as McpTransportType });
    setValidationError(null);
  };

  const inputStyle = {
    background: 'var(--black-30)',
    border: '1px solid var(--border-glass)',
    borderRadius: 'var(--radius-md)',
    padding: '0.75rem',
    color: 'var(--text-primary)',
    width: '100%',
    outline: 'none',
    fontSize: '0.9rem',
    fontFamily: 'monospace'
  };

  const isStdio = (transport?: string) => !transport || transport === 'stdio';

  return (
    <div className="main-panel ani-fade" style={{ display: 'flex', flexDirection: 'column', height: '100%', overflow: 'hidden' }}>
      <div className="panel-header" style={{ padding: 'var(--space-md)', borderBottom: '1px solid var(--border-glass)', display: 'flex', justifyContent: 'space-between', alignItems: 'center', background: 'var(--bg-glass-light)' }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: 'var(--space-sm)' }}>
          <Server size={20} color="var(--accent-cyan)" />
          <h3 style={{ margin: 0 }}>{t('page.mcpDashboard')}</h3>
        </div>
        <button onClick={loadConfig} className="secondary-button" style={{ display: 'flex', gap: '0.5rem', alignItems: 'center' }}>
          <RefreshCw size={16} className={isLoading ? "ani-spin" : ""} />
          {t('common.refresh')}
        </button>
      </div>

      <div style={{ flex: 1, overflowY: 'auto', padding: 'var(--space-lg)' }}>
        {/* Error Banner */}
        <AnimatePresence>
          {loadError && (
            <motion.div
              initial={{ opacity: 0, height: 0 }}
              animate={{ opacity: 1, height: 'auto' }}
              exit={{ opacity: 0, height: 0 }}
              style={{ background: 'var(--accent-rose-10)', border: '1px solid var(--accent-rose-30)', borderRadius: 'var(--radius-md)', padding: '1rem', marginBottom: 'var(--space-lg)', display: 'flex', alignItems: 'center', gap: '0.75rem' }}
            >
              <AlertTriangle size={18} color="var(--accent-rose)" />
              <span style={{ color: 'var(--accent-rose)', fontSize: '0.9rem', fontWeight: 600 }}>{loadError}</span>
            </motion.div>
          )}
        </AnimatePresence>

        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 'var(--space-lg)' }}>
          <div>
            <h4 style={{ color: 'var(--text-primary)', margin: '0 0 0.5rem 0' }}>
              {t('mcp.registeredServers', { defaultValue: 'Registered Servers' }) as string}
            </h4>
            <p style={{ color: 'var(--text-muted)', fontSize: '0.85rem', margin: 0 }}>
              {t('mcp.configPath', { defaultValue: 'Servers configured in' }) as string}{' '}
              <code style={{ background: 'var(--black-30)', padding: '2px 6px', borderRadius: '4px' }}>~/.aiome/mcp_servers.json</code>
            </p>
          </div>
          <div style={{ display: 'flex', gap: '1rem', alignItems: 'center' }}>
            <div style={{ display: 'flex', alignItems: 'center', background: 'var(--black-30)', border: '1px solid var(--border-glass)', borderRadius: 'var(--radius-md)', padding: '0.5rem 0.75rem', width: '250px' }}>
              <Search size={16} color="var(--text-muted)" style={{ marginRight: '0.5rem' }} />
              <input
                type="text"
                value={searchTerm}
                onChange={e => setSearchTerm(e.target.value)}
                placeholder={t('mcp.searchPlaceholder', { defaultValue: 'Search servers...' }) as string}
                style={{ background: 'transparent', border: 'none', color: 'var(--text-primary)', outline: 'none', width: '100%', fontSize: '0.85rem' }}
              />
            </div>
            <button
              onClick={() => { setEditingKey('NEW'); setValidationError(null); }}
              className="primary-button"
              style={{ display: 'flex', alignItems: 'center', gap: '0.5rem', background: 'var(--accent-cyan)' }}
            >
              <Plus size={18} />
              {t('mcp.addServer', { defaultValue: 'Add Server' }) as string}
            </button>
          </div>
        </div>

        <AnimatePresence>
          {editingKey === 'NEW' && (
            <motion.div
              initial={{ height: 0, opacity: 0 }}
              animate={{ height: 'auto', opacity: 1 }}
              exit={{ height: 0, opacity: 0 }}
              style={{ overflow: 'hidden', marginBottom: 'var(--space-xl)' }}
            >
              <div style={{ background: 'var(--bg-glass-heavy)', border: '1px solid var(--accent-cyan)', borderRadius: 'var(--radius-lg)', padding: 'var(--space-lg)' }}>
                <h5 style={{ margin: '0 0 1rem 0', color: 'var(--accent-cyan)' }}>
                  {t('mcp.registerNew', { defaultValue: 'Register New MCP Server' }) as string}
                </h5>

                {/* Validation Error */}
                {validationError && (
                  <div style={{ background: 'var(--accent-rose-10)', border: '1px solid var(--accent-rose-30)', borderRadius: 'var(--radius-sm)', padding: '0.75rem', marginBottom: '1rem', display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
                    <AlertTriangle size={16} color="var(--accent-rose)" />
                    <span style={{ color: 'var(--accent-rose)', fontSize: '0.85rem' }}>{validationError}</span>
                  </div>
                )}

                <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '1rem', marginBottom: '1rem' }}>
                  <div>
                    <label style={{ display: 'block', fontSize: '0.75rem', fontWeight: 700, color: 'var(--text-secondary)', marginBottom: '0.5rem' }}>
                      {t('mcp.serverId', { defaultValue: 'Server ID' }) as string}
                    </label>
                    <input
                      style={inputStyle}
                      value={newServerId}
                      onChange={e => { setNewServerId(e.target.value); setValidationError(null); }}
                      placeholder="e.g. sqlite-mcp"
                    />
                  </div>
                  <div>
                    <label style={{ display: 'block', fontSize: '0.75rem', fontWeight: 700, color: 'var(--text-secondary)', marginBottom: '0.5rem' }}>
                      {t('mcp.transport', { defaultValue: 'Transport' }) as string}
                    </label>
                    <select style={inputStyle} value={newServerConfig.transport || 'stdio'} onChange={e => handleTransportChange(e.target.value)}>
                      <option value="stdio" style={{ background: 'var(--bg-primary)' }}>STDIO ({t('mcp.localCommand', { defaultValue: 'Local Command' }) as string})</option>
                      <option value="http" style={{ background: 'var(--bg-primary)' }}>HTTP ({t('mcp.remoteUrl', { defaultValue: 'Remote URL' }) as string})</option>
                    </select>
                  </div>
                </div>

                {isStdio(newServerConfig.transport) ? (
                  <div style={{ display: 'grid', gridTemplateColumns: '1fr 2fr', gap: '1rem', marginBottom: '1rem' }}>
                    <div>
                      <label style={{ display: 'block', fontSize: '0.75rem', fontWeight: 700, color: 'var(--text-secondary)', marginBottom: '0.5rem' }}>
                        {t('mcp.command', { defaultValue: 'Command' }) as string}
                      </label>
                      <input style={inputStyle} value={newServerConfig.command} onChange={e => setNewServerConfig({ ...newServerConfig, command: e.target.value })} placeholder="npx" />
                    </div>
                    <div>
                      <label style={{ display: 'block', fontSize: '0.75rem', fontWeight: 700, color: 'var(--text-secondary)', marginBottom: '0.5rem' }}>
                        {t('mcp.args', { defaultValue: 'Args (comma separated)' }) as string}
                      </label>
                      <input style={inputStyle} value={newServerConfig.args.join(', ')} onChange={e => setNewServerConfig({ ...newServerConfig, args: e.target.value.split(',').map(s => s.trim()).filter(s => s) })} placeholder="-y, @modelcontextprotocol/server-sqlite" />
                    </div>
                  </div>
                ) : (
                  <div style={{ marginBottom: '1rem' }}>
                    <label style={{ display: 'block', fontSize: '0.75rem', fontWeight: 700, color: 'var(--text-secondary)', marginBottom: '0.5rem' }}>URL</label>
                    <input style={inputStyle} value={newServerConfig.url || ''} onChange={e => setNewServerConfig({ ...newServerConfig, url: e.target.value })} placeholder="https://example.com/mcp" />
                  </div>
                )}

                <div style={{ display: 'flex', gap: '1rem', justifyContent: 'flex-end', marginTop: '1rem' }}>
                  <button onClick={() => { setEditingKey(null); setValidationError(null); setNewServerConfig({ ...DEFAULT_SERVER_CONFIG }); setNewServerId(''); }} className="secondary-button" style={{ padding: '0.6rem 1.2rem' }}>
                    {t('mcp.cancel', { defaultValue: 'Cancel' }) as string}
                  </button>
                  <button onClick={handleAddServer} disabled={isSaving} className="primary-button" style={{ background: 'var(--accent-cyan)', padding: '0.6rem 1.2rem', display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
                    <Save size={16} /> {isSaving ? (t('mcp.saving', { defaultValue: 'Saving...' }) as string) : (t('mcp.saveRestart', { defaultValue: 'Save & Restart' }) as string)}
                  </button>
                </div>
              </div>
            </motion.div>
          )}
        </AnimatePresence>

        <div style={{ display: 'flex', flexDirection: 'column', gap: 'var(--space-md)' }}>
          {Object.entries(config.mcp_servers).length === 0 && !isLoading && !loadError && (
            <div style={{ textAlign: 'center', padding: '3rem', background: 'var(--bg-glass)', borderRadius: 'var(--radius-lg)', border: '1px solid var(--border-glass)' }}>
              <Server size={48} style={{ opacity: 0.2, margin: '0 auto 1rem auto', display: 'block' }} color="var(--accent-cyan)" />
              <h4 style={{ color: 'var(--text-primary)', margin: '0 0 0.5rem 0' }}>
                {t('mcp.noServers', { defaultValue: 'No MCP Servers Registered' }) as string}
              </h4>
              <p style={{ color: 'var(--text-muted)' }}>
                {t('mcp.noServersDesc', { defaultValue: 'Add a server to grant Aiome new capabilities.' }) as string}
              </p>
            </div>
          )}

          {Object.entries(config.mcp_servers)
            .filter(([id, server]) => {
              if (!searchTerm) return true;
              const term = searchTerm.toLowerCase();
              return id.toLowerCase().includes(term) || 
                     (server.command && server.command.toLowerCase().includes(term)) ||
                     (server.url && server.url.toLowerCase().includes(term));
            })
            .map(([id, server]) => (
            <motion.div
              key={id}
              initial={{ opacity: 0, y: 10 }}
              animate={{ opacity: 1, y: 0 }}
              className="card-hover"
              style={{
                background: server.disabled ? 'var(--bg-glass-light)' : 'var(--bg-glass-heavy)',
                border: '1px solid var(--border-glass)',
                borderRadius: 'var(--radius-md)',
                padding: '1.5rem',
                display: 'flex',
                flexDirection: 'column',
                gap: '1rem',
                boxShadow: server.disabled ? 'none' : 'var(--shadow-medium)',
                opacity: server.disabled ? 0.7 : 1
              }}
            >
              <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start' }}>
                <div style={{ display: 'flex', gap: '1rem', alignItems: 'center' }}>
                  <div style={{
                    width: '48px', height: '48px', borderRadius: 'var(--radius-md)',
                    background: isStdio(server.transport) ? 'var(--accent-cyan-10)' : 'var(--accent-purple-10)',
                    color: isStdio(server.transport) ? 'var(--accent-cyan)' : 'var(--accent-purple)',
                    display: 'flex', alignItems: 'center', justifyContent: 'center'
                  }}>
                    {isStdio(server.transport) ? <Terminal size={24} /> : <Activity size={24} />}
                  </div>
                  <div>
                    <h4 style={{ margin: '0 0 0.25rem 0', color: 'var(--text-primary)', fontSize: '1.1rem', display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
                      {id}
                      <span style={{ fontSize: '0.65rem', fontWeight: 800, padding: '2px 6px', borderRadius: '4px', border: '1px solid currentColor', color: isStdio(server.transport) ? 'var(--accent-cyan)' : 'var(--accent-purple)' }}>
                        {isStdio(server.transport) ? 'STDIO' : 'HTTP'}
                      </span>
                    </h4>
                    <p style={{ margin: 0, fontSize: '0.85rem', color: 'var(--text-secondary)' }}>
                      {isStdio(server.transport) ? (
                        <span className="font-mono">{server.command} {server.args?.join(' ')}</span>
                      ) : (
                        <span className="font-mono">{server.url}</span>
                      )}
                    </p>
                  </div>
                </div>

                <div style={{ display: 'flex', gap: '0.5rem', alignItems: 'center' }}>
                  {!server.disabled && (
                    <span style={{ 
                      fontSize: '0.65rem', fontWeight: 800, padding: '4px 8px', borderRadius: '4px', height: 'fit-content',
                      background: activeMcpSkills[id] ? 'var(--accent-emerald-10)' : 'var(--accent-amber-10)', 
                      color: activeMcpSkills[id] ? 'var(--accent-emerald)' : 'var(--accent-amber)', 
                      border: `1px solid ${activeMcpSkills[id] ? 'var(--accent-emerald-20)' : 'var(--accent-amber-20)'}` 
                    }}>
                      {activeMcpSkills[id] ? `● ACTIVE (${activeMcpSkills[id].tools?.length || 0} TOOLS)` : '● CONNECTING...'}
                    </span>
                  )}
                  {server.disabled && (
                    <button
                      onClick={() => setEnablingServerId(id)}
                      className="primary-button"
                      style={{ background: 'var(--accent-cyan)', display: 'flex', alignItems: 'center', gap: '0.5rem', padding: '0.4rem 0.8rem', fontSize: '0.85rem' }}
                    >
                      {t('mcp.enable', { defaultValue: 'Enable' }) as string}
                    </button>
                  )}
                  <button
                    onClick={() => handleRemoveServer(id)}
                    className="secondary-button"
                    style={{ color: 'var(--accent-rose)', borderColor: 'var(--accent-rose-30)', background: 'var(--accent-rose-10)' }}
                    title={t('mcp.removeServer', { defaultValue: 'Remove Server' }) as string}
                  >
                    <Trash2 size={16} />
                  </button>
                </div>
              </div>

              {server.env && Object.keys(server.env).length > 0 && (
                <div style={{ background: 'var(--black-30)', padding: '0.75rem', borderRadius: 'var(--radius-sm)', border: '1px solid var(--white-05)' }}>
                  <div style={{ fontSize: '0.7rem', fontWeight: 700, color: 'var(--text-muted)', marginBottom: '0.5rem' }}>
                    {t('mcp.envVars', { defaultValue: 'ENVIRONMENT VARIABLES' }) as string}
                  </div>
                  <div style={{ display: 'flex', flexWrap: 'wrap', gap: '0.5rem' }}>
                    {Object.entries(server.env).map(([k, v]) => (
                      <span key={k} className="font-mono" style={{ fontSize: '0.75rem', background: 'var(--black-40)', padding: '2px 6px', borderRadius: '4px', border: '1px solid var(--white-10)' }}>
                        <span style={{ color: 'var(--accent-cyan)' }}>{k}</span>=<span style={{ color: 'var(--text-secondary)' }}>{v.startsWith('$') ? v : '***'}</span>
                      </span>
                    ))}
                  </div>
                </div>
              )}
            </motion.div>
          ))}
        </div>

        {Object.keys(config.mcp_servers).length > 0 && (
          <div style={{ marginTop: 'var(--space-xl)', position: 'relative', minHeight: '200px' }}>
            <h4 style={{ color: 'var(--text-primary)', marginBottom: 'var(--space-md)' }}>
              {t('mcp.recentActivity', { defaultValue: 'Recent Activity' }) as string}
            </h4>
            <ActivityFeed maxItems={5} />
          </div>
        )}
      </div>

      <ConfirmModal
        isOpen={!!enablingServerId}
        type="warning"
        title={t('mcp.securityWarning', { defaultValue: 'Security Warning' }) as string}
        message={t('mcp.tokenRecommendation', { defaultValue: 'You are about to enable a third-party MCP server. We strongly recommend using Read-Only or minimum privilege tokens to prevent unauthorized data access or modification.' }) as string}
        details={<><strong>Server:</strong> <span style={{ color: 'var(--text-primary)' }}>{enablingServerId}</span></>}
        confirmText={t('mcp.confirmEnable', { defaultValue: 'I understand, Enable' }) as string}
        onConfirm={handleEnableServer}
        onCancel={() => setEnablingServerId(null)}
      />

      <ConfirmModal
        isOpen={!!removingServerId}
        type="danger"
        title={t('mcp.removeServer', { defaultValue: 'Remove Server' }) as string}
        message={t('mcp.confirmRemove', { defaultValue: `Are you sure you want to remove MCP server '${removingServerId}'?` }) as string}
        confirmText={t('common.remove', { defaultValue: 'Remove' }) as string}
        onConfirm={executeRemoveServer}
        onCancel={() => setRemovingServerId(null)}
      />
    </div>
  );
}

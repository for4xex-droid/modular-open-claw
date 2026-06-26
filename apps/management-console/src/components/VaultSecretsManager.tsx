/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

import { useState, useEffect } from 'react';
import { useTranslation } from '../i18n';
import { authenticatedFetch } from '../lib/auth';
import { API_BASE } from '../config';
import { 
  Key, 
  Trash2, 
  Settings, 
  CheckCircle, 
  XCircle, 
  ChevronDown, 
  ChevronUp, 
  Loader2 
} from 'lucide-react';

export interface SecretItem {
  key: string;
  category: 'ai' | 'commerce' | 'bridge' | 'infrastructure' | string;
  is_set: boolean;
}

export function VaultSecretsManager() {
  const { t } = useTranslation();
  const [secrets, setSecrets] = useState<SecretItem[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [forbidden, setForbidden] = useState(false);
  
  // Accordion open states
  const [openSections, setOpenSections] = useState<Record<string, boolean>>({
    ai: true,
    commerce: true,
    bridge: false,
    infrastructure: false,
  });

  // Modal states
  const [editingKey, setEditingKey] = useState<string | null>(null);
  const [secretValue, setSecretValue] = useState('');
  const [saving, setSaving] = useState(false);

  const fetchStatus = async () => {
    setLoading(true);
    setError(null);
    setForbidden(false);
    try {
      const res = await authenticatedFetch(`${API_BASE}/api/v1/vault/status`);
      if (res.status === 403) {
        setForbidden(true);
        setLoading(false);
        return;
      }
      if (!res.ok) {
        throw new Error(`HTTP ${res.status}`);
      }
      const data = await res.json();
      setSecrets(data.secrets || []);
    } catch (err: any) {
      setError(err.message || 'Unknown error');
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    fetchStatus();
  }, []);

  const toggleSection = (section: string) => {
    setOpenSections(prev => ({ ...prev, [section]: !prev[section] }));
  };

  const handleSave = async () => {
    if (!editingKey || !secretValue) return;
    setSaving(true);
    try {
      const res = await authenticatedFetch(`${API_BASE}/api/v1/vault/secrets`, {
        method: 'PUT',
        body: JSON.stringify({
          key: editingKey,
          value: secretValue,
        }),
      });
      if (!res.ok) {
        throw new Error(`HTTP ${res.status}`);
      }
      setEditingKey(null);
      setSecretValue('');
      await fetchStatus();
    } catch (err: any) {
      alert(t('vault.toast.saveError', { error: err.message }));
    } finally {
      setSaving(false);
    }
  };

  const handleDelete = async (key: string) => {
    if (!window.confirm(t('vault.status.deleteConfirm'))) return;
    try {
      const res = await authenticatedFetch(`${API_BASE}/api/v1/vault/secrets/${key}`, {
        method: 'DELETE',
      });
      if (!res.ok) {
        throw new Error(`HTTP ${res.status}`);
      }
      await fetchStatus();
    } catch (err: any) {
      alert(t('vault.toast.deleteError', { error: err.message }));
    }
  };

  if (forbidden) {
    return (
      <div className="vault-forbidden" style={{ padding: 'var(--space-md)', textAlign: 'center', color: 'var(--color-danger)' }}>
        <p>{t('vault.permissionRequired')}</p>
      </div>
    );
  }

  if (loading) {
    return (
      <div className="vault-loading" style={{ display: 'flex', justifyContent: 'center', padding: 'var(--space-xl)' }}>
        <Loader2 className="animate-spin" style={{ color: 'var(--color-primary-light)' }} />
      </div>
    );
  }

  if (error) {
    return (
      <div className="vault-error" style={{ padding: 'var(--space-md)', color: 'var(--color-danger)' }}>
        <p>{t('vault.toast.loadError', { error })}</p>
        <button className="btn btn-secondary" onClick={fetchStatus}>Retry</button>
      </div>
    );
  }

  const categories = ['ai', 'commerce', 'bridge', 'infrastructure'];

  return (
    <div className="vault-secrets-manager">
      <div className="vault-header" style={{ marginBottom: 'var(--space-lg)' }}>
        <h3 style={{ display: 'flex', alignItems: 'center', gap: 'var(--space-sm)' }}>
          <Key /> {t('vault.title')}
        </h3>
        <p style={{ fontSize: 'var(--font-sm)', color: 'var(--text-secondary)' }}>
          {t('vault.description')}
        </p>
      </div>

      <div className="vault-categories-list" style={{ display: 'flex', flexDirection: 'column', gap: 'var(--space-md)' }}>
        {categories.map(cat => {
          const catSecrets = secrets.filter(s => s.category === cat);
          const isOpen = openSections[cat];
          return (
            <div key={cat} className="vault-category-card" style={{ border: '1px solid var(--border-color)', borderRadius: 'var(--radius-md)', overflow: 'hidden' }}>
              <div 
                className="vault-category-header" 
                onClick={() => toggleSection(cat)}
                style={{ 
                  display: 'flex', 
                  justifyContent: 'space-between', 
                  alignItems: 'center', 
                  padding: 'var(--space-md)', 
                  background: 'var(--bg-secondary)', 
                  cursor: 'pointer',
                  userSelect: 'none'
                }}
              >
                <span style={{ fontWeight: 'bold' }}>{t(`vault.category.${cat}`)}</span>
                {isOpen ? <ChevronUp size={16} /> : <ChevronDown size={16} />}
              </div>

              {isOpen && (
                <div className="vault-category-content" style={{ padding: 'var(--space-sm) var(--space-md)' }}>
                  {catSecrets.length === 0 ? (
                    <p style={{ color: 'var(--text-muted)', fontSize: 'var(--font-sm)' }}>No keys in this category.</p>
                  ) : (
                    <table style={{ width: '100%', borderCollapse: 'collapse' }}>
                      <thead>
                        <tr style={{ borderBottom: '1px solid var(--border-color)', textAlign: 'left', fontSize: 'var(--font-sm)', color: 'var(--text-secondary)' }}>
                          <th style={{ padding: 'var(--space-sm) 0' }}>Key</th>
                          <th style={{ padding: 'var(--space-sm) 0', width: '120px' }}>Status</th>
                          <th style={{ padding: 'var(--space-sm) 0', width: '150px', textAlign: 'right' }}>{t('vault.status.actions')}</th>
                        </tr>
                      </thead>
                      <tbody>
                        {catSecrets.map(secret => (
                          <tr key={secret.key} style={{ borderBottom: '1px dotted var(--border-color)', fontSize: 'var(--font-md)' }}>
                            <td style={{ padding: 'var(--space-md) 0', fontFamily: 'monospace', fontWeight: 'bold' }}>{secret.key}</td>
                            <td style={{ padding: 'var(--space-md) 0' }}>
                              {secret.is_set ? (
                                <span style={{ display: 'inline-flex', alignItems: 'center', gap: '4px', color: 'var(--color-success)', fontSize: 'var(--font-sm)' }}>
                                  <CheckCircle size={14} /> {t('vault.status.set')}
                                </span>
                              ) : (
                                <span style={{ display: 'inline-flex', alignItems: 'center', gap: '4px', color: 'var(--color-warning)', fontSize: 'var(--font-sm)' }}>
                                  <XCircle size={14} /> {t('vault.status.notSet')}
                                </span>
                              )}
                            </td>
                            <td style={{ padding: 'var(--space-md) 0', textAlign: 'right' }}>
                              <button 
                                className="btn btn-sm btn-primary" 
                                onClick={() => setEditingKey(secret.key)}
                                style={{ marginRight: 'var(--space-xs)' }}
                              >
                                <Settings size={12} style={{ marginRight: '4px' }} /> {t('vault.status.configure')}
                              </button>
                              {secret.is_set && (
                                <button 
                                  className="btn btn-sm btn-danger" 
                                  onClick={() => handleDelete(secret.key)}
                                >
                                  <Trash2 size={12} />
                                </button>
                              )}
                            </td>
                          </tr>
                        ))}
                      </tbody>
                    </table>
                  )}
                </div>
              )}
            </div>
          );
        })}
      </div>

      {/* Configure Modal */}
      {editingKey && (
        <div className="modal-backdrop" style={{
          position: 'fixed',
          top: 0,
          left: 0,
          right: 0,
          bottom: 0,
          background: 'rgba(0,0,0,0.5)',
          display: 'flex',
          justifyContent: 'center',
          alignItems: 'center',
          zIndex: 1000
        }}>
          <div className="glass-panel" style={{
            width: '450px',
            padding: 'var(--space-lg)',
            borderRadius: 'var(--radius-lg)',
            background: 'var(--bg-primary)'
          }}>
            <h4>{t('vault.modal.title', { key: editingKey })}</h4>
            <div style={{ margin: 'var(--space-md) 0' }}>
              <input 
                type="password" 
                value={secretValue}
                onChange={(e) => setSecretValue(e.target.value)}
                placeholder={t('vault.modal.placeholder')}
                style={{
                  width: '100%',
                  padding: 'var(--space-sm)',
                  borderRadius: 'var(--radius-sm)',
                  border: '1px solid var(--border-color)',
                  background: 'var(--bg-secondary)',
                  color: 'var(--text-primary)'
                }}
              />
            </div>
            <div style={{ display: 'flex', justifyContent: 'flex-end', gap: 'var(--space-sm)' }}>
              <button className="btn btn-secondary" onClick={() => { setEditingKey(null); setSecretValue(''); }} disabled={saving}>
                {t('vault.modal.cancel')}
              </button>
              <button className="btn btn-primary" onClick={handleSave} disabled={saving || !secretValue}>
                {saving ? <Loader2 className="animate-spin" size={14} /> : t('vault.modal.save')}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

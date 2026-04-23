import React, { useState, useEffect, useCallback } from 'react';
import { motion } from 'framer-motion';
import { Shield, Unlock, RefreshCw, AlertCircle } from 'lucide-react';
import { API_BASE } from '../config';
import { authenticatedFetch } from '../lib/auth';
import { useTranslation } from '../i18n';
import { components } from '../types/generated';

type EscrowRecord = components['schemas']['EscrowRecord'];

export interface EscrowManagementViewProps {
  agentId?: string;
}

const EscrowManagementView: React.FC<EscrowManagementViewProps> = ({ agentId }) => {
  const { t } = useTranslation();
  const [escrows, setEscrows] = useState<EscrowRecord[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [releasing, setReleasing] = useState<Record<string, boolean>>({});

  const targetAgentId = agentId || "00000000-0000-0000-0000-000000000001"; // Fallback to system agent for testing

  const fetchEscrows = useCallback(async (signal?: AbortSignal) => {
    setLoading(true);
    setError(null);
    try {
      const res = await authenticatedFetch(`${API_BASE}/api/v1/commerce/escrow/history/${targetAgentId}`, { signal });
      if (res.ok) {
        const data = await res.json();
        setEscrows(data || []);
      } else {
        setError(t('escrow.loadFailed') || 'Failed to load escrow history');
      }
    } catch (e: unknown) {
      if (e instanceof DOMException && e.name === 'AbortError') return;
      console.error('Failed to fetch escrow history', e);
      setError(t('escrow.loadFailed') || 'Failed to load escrow history');
    } finally {
      setLoading(false);
    }
  }, [t, targetAgentId]);

  useEffect(() => {
    const controller = new AbortController();
    fetchEscrows(controller.signal);
    return () => controller.abort();
  }, [fetchEscrows]);

  const handleRelease = async (escrowId: string, payeeId: string) => {
    setReleasing(prev => ({ ...prev, [escrowId]: true }));
    try {
      const res = await authenticatedFetch(`${API_BASE}/api/v1/commerce/escrow/${escrowId}/release`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ payee_id: payeeId })
      });
      if (res.ok) {
        await fetchEscrows();
      } else {
        console.error('Failed to release escrow');
      }
    } catch (e) {
      console.error('Error releasing escrow', e);
    } finally {
      setReleasing(prev => ({ ...prev, [escrowId]: false }));
    }
  };

  return (
    <div className="main-panel ani-fade">
      <div className="panel-header">
        <div style={{ display: 'flex', alignItems: 'center', gap: '1rem' }}>
          <Shield size={20} color="var(--accent-blue)" />
          <h3>{t('escrow.title') || 'Escrow Management'}</h3>
        </div>
        <div style={{ display: 'flex', gap: 'var(--space-xs)', alignItems: 'center' }}>
          <button className="btn btn-secondary btn-sm" onClick={() => fetchEscrows()} disabled={loading}>
            <RefreshCw size={14} className={loading ? 'ani-spin' : ''} />
            {t('common.refresh')}
          </button>
        </div>
      </div>
      
      <div className="panel-content scroll-v" style={{ padding: 'var(--space-md)' }}>
        {loading ? (
          <div className="loading-state">
            <p>{t('common.loading')}</p>
          </div>
        ) : error ? (
          <div className="loading-state">
             <AlertCircle size={24} color="var(--accent-rose)" style={{ marginBottom: '0.5rem' }} />
             <p style={{ color: 'var(--accent-rose)' }}>{error}</p>
          </div>
        ) : escrows.length === 0 ? (
          <div className="loading-state" style={{ flexDirection: 'column', alignItems: 'center', opacity: 0.7 }}>
             <Shield size={48} style={{ opacity: 0.2, marginBottom: '1rem' }} />
             <p>{t('escrow.noData') || 'No active or historical escrows found.'}</p>
          </div>
        ) : (
          <div className="escrow-grid">
            {escrows.map((escrow, idx) => (
              <motion.div 
                key={escrow.id}
                className="escrow-card"
                initial={{ opacity: 0, y: 10 }}
                animate={{ opacity: 1, y: 0 }}
                transition={{ delay: idx * 0.05 }}
              >
                <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '0.75rem' }}>
                  <h4 style={{ margin: 0, display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
                    <span style={{ fontSize: '0.8rem', color: 'var(--text-muted)' }}>ID:</span>
                    <span style={{ fontFamily: 'var(--font-mono)' }}>{escrow.id.substring(0, 8)}...</span>
                  </h4>
                  <div className={`status-badge ${escrow.status.toLowerCase()}`}>
                    {escrow.status}
                  </div>
                </div>
                
                <div className="escrow-row">
                  <span>Amount:</span>
                  <strong style={{ fontSize: '1.1rem', color: 'var(--accent-emerald)' }}>
                    {escrow.amount}
                  </strong>
                </div>
                <div className="escrow-row">
                  <span>Created:</span>
                  <strong>{new Date(escrow.created_at).toLocaleString()}</strong>
                </div>
                
                {escrow.status === 'Locked' && (
                  <div style={{ marginTop: '1rem', paddingTop: '1rem', borderTop: '1px solid var(--border-glass-bright)' }}>
                    <button 
                      className="btn btn-primary" 
                      style={{ width: '100%', display: 'flex', justifyContent: 'center', gap: '0.5rem' }}
                      onClick={() => handleRelease(escrow.id, targetAgentId)}
                      disabled={releasing[escrow.id]}
                    >
                      <Unlock size={14} />
                      {releasing[escrow.id] ? 'Releasing...' : 'Release Escrow'}
                    </button>
                  </div>
                )}
              </motion.div>
            ))}
          </div>
        )}
      </div>

      <style>{`
        .escrow-grid {
          display: grid;
          grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
          gap: var(--space-md);
        }
        .escrow-card {
          background: var(--bg-glass);
          border: 1px solid var(--border-glass-bright);
          border-radius: var(--radius-md);
          padding: var(--space-md);
          box-shadow: var(--shadow-shallow);
        }
        .escrow-row {
          display: flex;
          justify-content: space-between;
          align-items: center;
          font-size: 0.85rem;
          margin-bottom: var(--space-xs);
          color: var(--text-muted);
        }
        .escrow-row strong {
          color: var(--text-primary);
          font-family: var(--font-mono);
        }
        .status-badge {
          font-size: 0.7rem;
          padding: 2px 8px;
          border-radius: 12px;
          text-transform: uppercase;
          font-weight: 700;
          letter-spacing: 0.05em;
        }
        .status-badge.locked {
          background: var(--accent-amber-10);
          color: var(--accent-amber);
          border: 1px solid var(--accent-amber-30);
        }
        .status-badge.released {
          background: var(--accent-emerald-10);
          color: var(--accent-emerald);
          border: 1px solid var(--accent-emerald-30);
        }
        .status-badge.refunded {
          background: var(--accent-purple-10);
          color: var(--accent-purple);
          border: 1px solid var(--accent-purple-30);
        }
        .loading-state {
          display: flex;
          justify-content: center;
          align-items: center;
          padding: var(--space-xl);
          color: var(--text-muted);
          min-height: 200px;
        }
      `}</style>
    </div>
  );
};

export default EscrowManagementView;

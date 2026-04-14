import React, { useState, useEffect, useCallback } from 'react';
import { motion } from 'framer-motion';
import { Activity, RefreshCw } from 'lucide-react';
import { API_BASE } from '../config';
import { authenticatedFetch } from '../lib/auth';
import { useTranslation } from '../i18n';

import { components } from '../types/generated';

type ProviderStat = components['schemas']['ProviderEvalStat'];

const PromptStatsView: React.FC = () => {
  const { t } = useTranslation();
  const [stats, setStats] = useState<ProviderStat[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [period, setPeriod] = useState<string>('7d');

  const fetchStats = useCallback(async (signal?: AbortSignal) => {
    setLoading(true);
    setError(null);
    try {
      const res = await authenticatedFetch(`${API_BASE}/api/v1/audit/prompt-stats?period=${encodeURIComponent(period)}`, { signal });
      if (res.ok) {
        const data = await res.json();
        setStats(data.providers || []);
      } else {
        setError(t('promptStats.loadFailed'));
      }
    } catch (e: unknown) {
      if (e instanceof DOMException && e.name === 'AbortError') return;
      console.error('Failed to fetch prompt stats', e);
      setError(t('promptStats.loadFailed'));
    } finally {
      setLoading(false);
    }
  }, [t]);

  useEffect(() => {
    const controller = new AbortController();
    fetchStats(controller.signal);
    return () => controller.abort();
  }, [fetchStats, period]);

  return (
    <div className="main-panel ani-fade">
      <div className="panel-header">
        <div style={{ display: 'flex', alignItems: 'center', gap: '1rem' }}>
          <Activity size={20} color="var(--accent-purple)" />
          <h3>{t('promptStats.title')}</h3>
        </div>
        <div style={{ display: 'flex', gap: 'var(--space-xs)', alignItems: 'center' }}>
          <select 
            value={period} 
            onChange={(e) => setPeriod(e.target.value)}
            className="input"
            style={{ padding: 'var(--space-xs) var(--space-sm)', height: 'auto' }}
          >
            <option value="7d">Last 7 Days</option>
            <option value="30d">Last 30 Days</option>
            <option value="90d">Last 90 Days</option>
          </select>
          <button className="btn btn-secondary btn-sm" onClick={() => fetchStats()} disabled={loading}>
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
             <p style={{ color: 'var(--accent-rose)' }}>{error}</p>
          </div>
        ) : stats.length === 0 ? (
          <div className="loading-state">
             <p>{t('promptStats.noData')}</p>
          </div>
        ) : (
          <>
            <div className="chart-container">
              <h4 style={{ margin: '0 0 var(--space-sm) 0', color: 'var(--text-muted)' }}>{t('promptStats.costComparison') || 'Cost Comparison'}</h4>
              {stats.map((stat, idx) => {
                const maxCost = Math.max(...stats.map(s => s.total_cost_usd), 0.0001);
                const perc = (stat.total_cost_usd / maxCost) * 100;
                return (
                  <div key={`chart-cost-${stat.model}-${idx}`} className="chart-bar-wrap">
                    <div className="chart-label">{stat.model}</div>
                    <div className="chart-track">
                      <motion.div 
                        className="chart-fill"
                        style={{ background: `var(--chart-${(idx % 5) + 1})` }}
                        initial={{ width: 0 }}
                        animate={{ width: `${perc}%` }}
                        transition={{ delay: 0.2 + idx * 0.1 }}
                      />
                    </div>
                    <div className="chart-value">${stat.total_cost_usd.toFixed(4)}</div>
                  </div>
                );
              })}
            </div>
            
            <div className="stats-grid">
              {stats.map((stat, idx) => (
              <motion.div 
                key={`${stat.provider}-${stat.model}-${idx}`}
                className="stat-card"
                initial={{ opacity: 0, y: 10 }}
                animate={{ opacity: 1, y: 0 }}
                transition={{ delay: idx * 0.05 }}
              >
                <h4>{stat.provider} - {stat.model}</h4>
                <div className="stat-row">
                  <span>{t('promptStats.calls')}:</span>
                  <strong>{stat.total_calls}</strong>
                </div>
                <div className="stat-row">
                  <span>{t('promptStats.latency')}:</span>
                  <strong>{stat.average_latency_ms.toFixed(1)} ms</strong>
                </div>
                <div className="stat-row">
                  <span>{t('promptStats.cost')}:</span>
                  <strong>${stat.total_cost_usd.toFixed(4)}</strong>
                </div>
                <div className="stat-row">
                  <span>{t('promptStats.cache')}:</span>
                  <strong>{stat.cache_hit_rate.toFixed(1)}%</strong>
                </div>
              </motion.div>
            ))}
            </div>
          </>
        )}
      </div>

      <style>{`
        .chart-container {
          background: var(--bg-tertiary);
          border-radius: var(--radius-md);
          padding: var(--space-md);
          margin-bottom: var(--space-lg);
          border: 1px solid var(--border-glass-bright);
        }
        .chart-bar-wrap {
          display: flex;
          align-items: center;
          gap: var(--space-sm);
          margin-bottom: var(--space-xs);
          font-family: var(--font-mono);
          font-size: 0.8rem;
        }
        .chart-label {
          width: 140px;
          text-align: right;
          color: var(--text-primary);
          overflow: hidden;
          text-overflow: ellipsis;
          white-space: nowrap;
        }
        .chart-track {
          flex: 1;
          background: var(--white-05);
          height: 12px;
          border-radius: 6px;
          overflow: hidden;
        }
        .chart-fill {
          height: 100%;
          border-radius: 6px;
        }
        .chart-value {
          width: 80px;
          color: var(--text-muted);
        }
        .stats-grid {
          display: grid;
          grid-template-columns: repeat(auto-fill, minmax(240px, 1fr));
          gap: var(--space-sm);
        }
        .stat-card {
          background: var(--white-02);
          border: 1px solid var(--border-glass-bright);
          border-radius: var(--radius-md);
          padding: var(--space-sm);
        }
        .stat-card h4 {
          margin: 0 0 var(--space-sm) 0;
          color: var(--text-primary);
          font-family: var(--font-display);
        }
        .stat-row {
          display: flex;
          justify-content: space-between;
          font-size: 0.8rem;
          margin-bottom: var(--space-xs);
          color: var(--text-muted);
        }
        .stat-row strong {
          color: var(--text-primary);
        }
        .loading-state {
          display: flex;
          justify-content: center;
          padding: var(--space-xl);
          color: var(--text-muted);
        }
      `}</style>
    </div>
  );
};

export default PromptStatsView;

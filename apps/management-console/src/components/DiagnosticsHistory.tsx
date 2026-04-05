/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import React, { useState, useEffect } from "react";
import { motion } from "framer-motion";
import {
  Activity,
  History,
  AlertTriangle,
  Clock,
  Database,
  ChevronRight,
  RefreshCw,
  Hash
} from "lucide-react";
import { API_BASE } from "../config";
import { authenticatedFetch } from "../lib/auth";
import { useTranslation } from '../i18n';

interface Diagnosis {
  id: number;
  job_id: string;
  root_cause: string | null;
  self_repair_hint: string | null;
  failure_category: string | null;
  timestamp: string | null;
}

interface AuditEntry {
  id: number;
  table_name: string;
  operation: "INSERT" | "UPDATE" | "DELETE";
  record_id: string;
  current_hash: string;
  timestamp: string | null;
}

const DiagnosticsHistory: React.FC = () => {
    const { t } = useTranslation();
  const [activeTab, setActiveTab] = useState<"diagnostics" | "ledger">("diagnostics");
  const [diagnoses, setDiagnoses] = useState<Diagnosis[]>([]);
  const [ledger, setLedger] = useState<AuditEntry[]>([]);
  const [loading, setLoading] = useState(true);
  const [page, setPage] = useState(1);

  const PAGE_SIZE = 20;

  useEffect(() => {
    fetchData();
  }, [activeTab]);

  const fetchData = async () => {
    setLoading(true);
    try {
      const endpoint = activeTab === "diagnostics" ? "/api/v1/audit/diagnostics" : "/api/v1/audit/ledger";
      const res = await authenticatedFetch(`${API_BASE}${endpoint}`);
      if (res.ok) {
        const data = await res.json();
        if (activeTab === "diagnostics") setDiagnoses(data);
        else setLedger(data);
      }
    } catch (e) {
      console.error(`Failed to fetch ${activeTab}`, e);
    } finally {
      setLoading(false);
    }
  };

  const getCategoryColor = (cat: string | null) => {
    switch (cat?.toLowerCase()) {
      case "security": return "var(--accent-rose)";
      case "context": return "var(--accent-purple)";
      case "runtime": return "var(--accent-cyan)";
      default: return "var(--text-muted)";
    }
  };

  const renderDiagnostics = () => (
    <div className="audit-list">
      {diagnoses.slice(0, page * PAGE_SIZE).map((d) => (
        <motion.div
          key={d.id}
          className="audit-card"
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
        >
          <div className="card-header">
            <div className="id-badge">
              <Hash size={12} /> {d.job_id.slice(0, 8)}
            </div>
            <div className="timestamp">
              <Clock size={12} /> {d.timestamp ? new Date(d.timestamp).toLocaleString() : t('diagnostics.unknown')}
            </div>
          </div>
          
          <div className="card-body">
            <div className="status-indicator" style={{ background: getCategoryColor(d.failure_category) }} />
            <div className="issue-context">
              <h4 style={{ color: getCategoryColor(d.failure_category) }}>
                {d.failure_category?.toUpperCase() || "LOG"}
              </h4>
              <p className="root-cause">{d.root_cause || t('diagnostics.noRootCause')}</p>
            </div>
          </div>

          {d.self_repair_hint && (
            <div className="repair-hint">
              <Activity size={12} color="var(--accent-cyan)" />
              <span>{d.self_repair_hint}</span>
            </div>
          )}
        </motion.div>
      ))}
      {diagnoses.length > page * PAGE_SIZE && (
        <button className="load-more" onClick={() => setPage(p => p + 1)}>
          LOAD MORE <ChevronRight size={14} />
        </button>
      )}
    </div>
  );

  const renderLedger = () => (
    <div className="audit-list">
      {ledger.slice(0, page * PAGE_SIZE).map((e) => (
        <motion.div
            key={e.id}
            className="audit-card ledger-item"
            initial={{ opacity: 0, x: -10 }}
            animate={{ opacity: 1, x: 0 }}
        >
          <div className="ledger-op" style={{ color: e.operation === 'DELETE' ? 'var(--accent-rose)' : 'var(--accent-cyan)' }}>
            {e.operation}
          </div>
          <div className="ledger-content">
            <div className="table-info">
              <Database size={12} /> <span>{e.table_name}</span>
              <span className="record-id">ID: {e.record_id}</span>
            </div>
            <div className="hash-sig">
              SHA256: {e.current_hash.slice(0, 16)}...
            </div>
          </div>
          <div className="timestamp small">
            {e.timestamp ? new Date(e.timestamp).toLocaleTimeString() : "--:--"}
          </div>
        </motion.div>
      ))}
    </div>
  );

  return (
    <div className="main-panel ani-fade">
      <div className="panel-header">
        <div style={{ display: 'flex', alignItems: 'center', gap: '1rem' }}>
          <Activity size={20} color="var(--accent-rose)" />
          <h3>{t('diagnostics.title')}</h3>
        </div>
        <div className="tab-switcher">
          <button className={activeTab === 'diagnostics' ? 'active' : ''} onClick={() => setActiveTab('diagnostics')}>
            <AlertTriangle size={14} /> DIAGNOSTICS
          </button>
          <button className={activeTab === 'ledger' ? 'active' : ''} onClick={() => setActiveTab('ledger')}>
            <History size={14} /> GLOBAL LEDGER
          </button>
        </div>
      </div>

      <div className="panel-content scroll-v" style={{ padding: '1.5rem' }}>
        {loading && page === 1 ? (
          <div className="loading-state">
            <RefreshCw className="ani-pulse" size={48} color="var(--accent-rose)" />
            <p>{t('diagnostics.syncing')}</p>
          </div>
        ) : (
          activeTab === 'diagnostics' ? renderDiagnostics() : renderLedger()
        )}
      </div>

      <style>{`
        .tab-switcher {
          display: flex;
          background: rgba(255,255,255,0.05);
          border-radius: 12px;
          padding: 0.2rem;
          gap: 0.2rem;
        }
        .tab-switcher button {
          border: none;
          background: transparent;
          color: var(--text-muted);
          padding: 0.4rem 1rem;
          border-radius: 10px;
          cursor: pointer;
          font-size: 0.75rem;
          font-weight: 600;
          display: flex;
          align-items: center;
          gap: 0.5rem;
          transition: all 0.2s;
        }
        .tab-switcher button.active {
          background: var(--bg-glass-heavy);
          color: white;
          box-shadow: 0 4px 10px rgba(0,0,0,0.2);
        }
        .audit-list {
          display: flex;
          flex-direction: column;
          gap: 1rem;
          max-width: 900px;
          margin: 0 auto;
        }
        .audit-card {
          background: rgba(255,255,255,0.02);
          border: 1px solid rgba(255,255,255,0.05);
          border-radius: 16px;
          padding: 1.2rem;
          position: relative;
          overflow: hidden;
        }
        .audit-card:hover {
          background: rgba(255,255,255,0.04);
          border-color: rgba(255,255,255,0.1);
        }
        .id-badge {
          display: flex;
          align-items: center;
          gap: 0.3rem;
          font-size: 0.7rem;
          color: var(--text-muted);
          font-family: var(--font-mono);
          background: rgba(255,255,255,0.05);
          padding: 0.2rem 0.5rem;
          border-radius: 4px;
        }
        .timestamp {
          font-size: 0.7rem;
          color: var(--text-muted);
          display: flex;
          align-items: center;
          gap: 0.3rem;
        }
        .card-header {
          display: flex;
          justify-content: space-between;
          margin-bottom: 1rem;
        }
        .card-body {
          display: flex;
          gap: 1rem;
          align-items: flex-start;
        }
        .status-indicator {
          width: 4px;
          height: 40px;
          border-radius: 4px;
        }
        .issue-context h4 {
          margin: 0 0 0.3rem 0;
          font-size: 0.75rem;
          letter-spacing: 0.05em;
        }
        .root-cause {
          font-size: 0.9rem;
          color: white;
          margin: 0;
        }
        .repair-hint {
          margin-top: 1rem;
          background: rgba(0,242,255,0.05);
          border-radius: 8px;
          padding: 0.6rem 0.8rem;
          display: flex;
          align-items: center;
          gap: 0.8rem;
          font-size: 0.8rem;
          color: var(--text-secondary);
        }
        .load-more {
          background: transparent;
          border: 1px solid rgba(255,255,255,0.1);
          color: var(--text-muted);
          padding: 0.8rem;
          border-radius: 12px;
          cursor: pointer;
          display: flex;
          align-items: center;
          justify-content: center;
          gap: 0.5rem;
          font-size: 0.75rem;
          font-weight: 700;
          transition: all 0.2s;
        }
        .load-more:hover {
          color: white;
          border-color: rgba(255,255,255,0.3);
          background: rgba(255,255,255,0.02);
        }
        .ledger-item {
          display: flex;
          align-items: center;
          gap: 1.5rem;
          padding: 0.8rem 1.2rem;
        }
        .ledger-op {
          font-weight: 900;
          font-family: var(--font-display);
          font-size: 0.7rem;
          width: 60px;
        }
        .ledger-content {
          flex: 1;
        }
        .table-info {
          display: flex;
          align-items: center;
          gap: 0.5rem;
          font-size: 0.8rem;
          color: white;
          margin-bottom: 0.2rem;
        }
        .record-id {
          color: var(--text-muted);
          font-size: 0.7rem;
          margin-left: 0.5rem;
        }
        .hash-sig {
          font-size: 0.6rem;
          font-family: var(--font-mono);
          color: var(--text-muted);
        }
        .loading-state {
          display: flex;
          flex-direction: column;
          align-items: center;
          justify-content: center;
          padding: 5rem;
          gap: 1.5rem;
          color: var(--text-muted);
        }
      `}</style>
    </div>
  );
};

export default DiagnosticsHistory;

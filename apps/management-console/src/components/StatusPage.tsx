/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import { useState, useEffect } from "react";
import { 
  Activity, 
  Shield, 
  AlertTriangle, 
  RefreshCw, 
  Cpu, 
  Server, 
  Database,
} from "lucide-react";
import { authenticatedFetch } from "../lib/auth";
import { API_BASE } from "../config";

import { components } from "../types/generated";
import { StatCard } from './ui/StatCard';
import { SectionHeader } from './ui/SectionHeader';
import { Card } from './ui/Card';

type SystemHealth = components["schemas"]["ResourceStatus"];

export default function StatusPage() {
  const [health, setHealth] = useState<SystemHealth | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [refreshing, setRefreshing] = useState(false);

  const fetchHealth = async () => {
    try {
      setRefreshing(true);
      const response = await authenticatedFetch(`${API_BASE}/api/health`);
      if (!response.ok) {
        throw new Error(`Failed with status: ${response.status}`);
      }
      const data = await response.json();
      setHealth(data);
      setError(null);
    } catch (err: unknown) {
      console.error("Failed to fetch system status", err);
      setError("Failed to load system health metrics. Please check network connection.");
    } finally {
      setLoading(false);
      setRefreshing(false);
    }
  };

  useEffect(() => {
    let cancelled = false;
    let timerId: number | undefined;
    const poll = async () => {
      await fetchHealth();
      if (!cancelled) {
        // setTimeout ensures next poll starts only after previous completes
        timerId = window.setTimeout(poll, 30000);
      }
    };
    poll();
    return () => {
      cancelled = true;
      if (timerId !== undefined) window.clearTimeout(timerId);
    };
  }, []);

  if (loading) {
    return (
      <div className="ui-center-state">
        <RefreshCw className="animate-spin text-accent-cyan" size={40} />
        <p className="ui-help-text">Synchronizing integrity metrics...</p>
      </div>
    );
  }

  if (error) {
    return (
      <div className="ui-center-state">
        <AlertTriangle color="var(--accent-rose)" size={48} />
        <h3 className="ui-section-header__title">System Telemetry Offline</h3>
        <p className="ui-help-text">{error}</p>
        <button 
          id="btn-retry-status"
          className="primary-button"
          onClick={fetchHealth}
        >
          <RefreshCw size={16} />
          Retry Connection
        </button>
      </div>
    );
  }

  const si = health?.support_incidents;
  const cb = health?.llm_circuit_breaker;
  const lora = health?.lora_engine;

  // CPU Gauge Calculations
  const cpuVal = health?.cpu_usage_percent || 0;
  // Memory Calculations
  const memUsed = health?.memory_usage_mb || 0;
  const memTotal = health?.total_memory_mb || 1;
  const memPercent = Math.min(100, Math.round((memUsed / memTotal) * 100));
  const diskUsedPercent = health && health.total_disk_gb > 0
    ? Math.round(((health.total_disk_gb - health.disk_free_gb) / health.total_disk_gb) * 100)
    : 0;

  return (
    <div className="settings-page ui-field-stack ui-field-stack--compact">
      {/* Header Info */}
      <div className="ui-field-row ui-field-row--between">
        <div className="header-title-block">
          <h2>System Status & Integrity Hub</h2>
          <p className="page-desc">
            Real-time diagnostics and autonomous support escalation telemetry
          </p>
        </div>
        <button 
          id="btn-refresh-status"
          className="primary-button"
          onClick={fetchHealth} 
          disabled={refreshing}
        >
          <RefreshCw size={14} className={refreshing ? "animate-spin" : ""} />
          Refresh
        </button>
      </div>

      {/* Support Incidents Analytics Panel (S-5 Requirement) */}
      <section className="ui-field-stack ui-field-stack--compact">
        <SectionHeader
          icon={<Shield size={18} color="var(--accent-cyan)" />}
          title="Support Escalation Stats (Last 7 Days)"
        />
        
        <div className="grid-stats">
          <StatCard
            label="Total Support Reports"
            value={si ? si.total_incidents_7d : 0}
            trend="Accumulated across past 7 days"
          />

          <StatCard
            label="Active Unresolved Incidents"
            value={si ? si.unresolved : 0}
            trend="Requires operator or system remediation"
            trendClassName={si && si.unresolved > 0 ? '' : 'trend-up'}
          />

          <StatCard
            label="Distinct Users Supported"
            value={si ? si.distinct_users : 0}
            trend="Unique users interacting with help desk"
          />

          <StatCard
            label="Peak Severity Node"
            value={si ? si.top_severity : "None"}
            trend="Highest incident alert level registered"
          />
        </div>
      </section>

      {/* Resource Health Metrics */}
      <div className="settings-grid">
        
        {/* Memory & Disk Telemetry */}
        <Card>
          <SectionHeader
            icon={<Cpu size={16} color="var(--accent-cyan)" />}
            title="Resource Telemetry"
          />

          {/* CPU Bar */}
          <div className="ui-field-stack ui-field-stack--xs">
            <div className="ui-field-row ui-field-row--between">
              <span className="ui-field-label ui-field-label--inline">CPU Utilization</span>
              <span className="trend-up">{cpuVal}%</span>
            </div>
            <div style={{ height: '8px', background: 'var(--white-05)', borderRadius: '4px', overflow: 'hidden' }}>
              <div style={{ height: '100%', width: `${cpuVal}%`, background: 'var(--accent-cyan)', borderRadius: '4px', transition: 'width 0.5s ease-out' }} />
            </div>
          </div>

          {/* Memory Bar */}
          <div className="ui-field-stack ui-field-stack--xs">
            <div className="ui-field-row ui-field-row--between">
              <span className="ui-field-label ui-field-label--inline">Resident Memory (RAM)</span>
              <span>{memUsed} MB / {memTotal} MB</span>
            </div>
            <div style={{ height: '8px', background: 'var(--white-05)', borderRadius: '4px', overflow: 'hidden' }}>
              <div style={{ height: '100%', width: `${memPercent}%`, background: 'var(--accent-purple)', borderRadius: '4px', transition: 'width 0.5s ease-out' }} />
            </div>
          </div>

          {/* Disk Bar */}
          <div className="ui-field-stack ui-field-stack--xs">
            <div className="ui-field-row ui-field-row--between">
              <span className="ui-field-label ui-field-label--inline">Local Disk (Free Space)</span>
              <span className="trend-up">{health?.disk_free_gb || 0} GB Free / {health?.total_disk_gb || 0} GB</span>
            </div>
            {health && (
              <div style={{ height: '8px', background: 'var(--white-05)', borderRadius: '4px', overflow: 'hidden' }}>
                <div style={{ height: '100%', width: `${diskUsedPercent}%`, background: 'var(--accent-emerald)', borderRadius: '4px' }} />
              </div>
            )}
          </div>
        </Card>

        {/* Component Integrity states (LLM Circuit Breaker, LoRA Autotuner, etc.) */}
        <Card>
          <SectionHeader
            icon={<Server size={16} color="var(--accent-purple)" />}
            title="Component Integrity Status"
          />

          {/* LLM Circuit Breaker Status */}
          <div className="ui-field-row ui-field-row--between glass-panel ui-card--pad-md">
            <div className="ui-field-row">
              <Database size={14} color="var(--accent-cyan)" />
              <span className="ui-field-label ui-field-label--inline">LLM Circuit Breaker</span>
            </div>
            <div className={`status-badge${cb?.state === 'Closed' ? '' : ' disconnected'}`}>
              <span className={`status-dot${cb?.state === 'Closed' ? '' : ' offline'}`} />
              {cb ? cb.state : "Offline"}
            </div>
          </div>

          {/* LoRA Engine Autotuner Status */}
          <div className="ui-field-row ui-field-row--between glass-panel ui-card--pad-md">
            <div className="ui-field-row">
              <Activity size={14} color="var(--accent-purple)" />
              <span className="ui-field-label ui-field-label--inline">LoRA Autotuner</span>
            </div>
            <div className={`status-badge${lora?.status === 'ready' ? '' : ' paused'}`}>
              <span className={`status-dot${lora?.status === 'ready' ? '' : ' offline'}`} />
              {lora ? lora.status : "Offline"}
            </div>
          </div>
        </Card>

      </div>
    </div>
  );
}

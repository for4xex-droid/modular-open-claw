import { useState, useEffect } from "react";
import { 
  Activity, 
  Shield, 
  AlertTriangle, 
  Users, 
  CheckCircle, 
  RefreshCw, 
  Cpu, 
  Server, 
  Database,
  Layers
} from "lucide-react";
import { authenticatedFetch } from "../lib/auth";
import { API_BASE } from "../config";

// TODO: U-004 — generated.ts の ResourceStatus は support_incidents / llm_circuit_breaker /
// lora_engine を `unknown` 型で出力するため、サブオブジェクトの型安全性が不十分。
// 将来的に Rust 側で各サブオブジェクトを独立した utoipa::ToSchema 構造体に昇格させ、
// generated types への完全移行を行うこと。
interface IncidentStats {
  total_incidents_7d: number;
  distinct_users: number;
  unresolved: number;
  top_severity: string;
}

interface CircuitBreakerStatus {
  name: string;
  state: string;
}

interface LoraStatus {
  mlx_available: boolean;
  status: string;
}

interface SystemHealth {
  memory_usage_mb: number;
  total_memory_mb: number;
  cpu_usage_percent: number;
  vram_usage_mb: number | null;
  disk_free_gb: number;
  total_disk_gb: number;
  llm_circuit_breaker: CircuitBreakerStatus | null;
  lora_engine: LoraStatus | null;
  support_incidents: IncidentStats | null;
}

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
      <div className="status-container loading-state" style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', justifyContent: 'center', height: '60vh', gap: '1rem' }}>
        <RefreshCw className="animate-spin text-accent-cyan" size={40} style={{ color: 'var(--accent-cyan)' }} />
        <p style={{ color: 'var(--text-secondary)' }}>Synchronizing integrity metrics...</p>
      </div>
    );
  }

  if (error) {
    return (
      <div className="status-container error-state" style={{ padding: '2rem', background: 'var(--accent-rose-05)', border: '1px solid var(--accent-rose-20)', borderRadius: '12px', margin: '2rem', display: 'flex', flexDirection: 'column', alignItems: 'center', gap: '1rem' }}>
        <AlertTriangle color="var(--accent-rose)" size={48} />
        <h3 style={{ color: 'var(--accent-rose)', margin: 0 }}>System Telemetry Offline</h3>
        <p style={{ color: 'var(--text-secondary)', textAlign: 'center', maxWidth: '500px' }}>{error}</p>
        <button 
          id="btn-retry-status"
          onClick={fetchHealth} 
          style={{ display: 'flex', alignItems: 'center', gap: '0.5rem', background: 'var(--accent-rose)', color: 'var(--bg-primary)', border: 'none', padding: '0.75rem 1.5rem', borderRadius: '8px', fontWeight: 'bold', cursor: 'pointer', transition: 'all 0.2s' }}
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

  return (
    <div className="status-page-wrapper" style={{ padding: '1.5rem', display: 'flex', flexDirection: 'column', gap: '1.5rem', color: 'var(--text-primary)' }}>
      {/* Header Info */}
      <div className="status-header" style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', borderBottom: '1px solid var(--white-05)', paddingBottom: '1rem' }}>
        <div>
          <h1 style={{ margin: 0, fontSize: '1.8rem', fontWeight: 800, background: 'linear-gradient(to right, var(--accent-cyan), var(--accent-purple))', WebkitBackgroundClip: 'text', WebkitTextFillColor: 'transparent' }}>
            System Status & Integrity Hub
          </h1>
          <p style={{ margin: '0.25rem 0 0 0', color: 'var(--text-secondary)', fontSize: '0.9rem' }}>
            Real-time diagnostics and autonomous support escalation telemetry
          </p>
        </div>
        <button 
          id="btn-refresh-status"
          onClick={fetchHealth} 
          disabled={refreshing}
          style={{ display: 'flex', alignItems: 'center', gap: '0.5rem', padding: '0.5rem 1rem', background: 'var(--black-40)', border: '1px solid var(--white-10)', borderRadius: '6px', cursor: 'pointer', color: 'var(--text-primary)', transition: 'all 0.2s' }}
        >
          <RefreshCw size={14} className={refreshing ? "animate-spin" : ""} />
          Refresh
        </button>
      </div>

      {/* Support Incidents Analytics Panel (S-5 Requirement) */}
      <section className="dashboard-section" style={{ display: 'flex', flexDirection: 'column', gap: '1rem' }}>
        <h3 style={{ margin: 0, fontSize: '1.2rem', fontWeight: 600, display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
          <Shield size={18} color="var(--accent-cyan)" />
          Support Escalation Stats (Last 7 Days)
        </h3>
        
        <div className="incident-grid" style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(220px, 1fr))', gap: '1rem' }}>
          {/* Card 1: Total Incidents */}
          <div className="glass-card" style={{ background: 'var(--black-40)', backdropFilter: 'blur(10px)', border: '1px solid var(--white-05)', borderRadius: '12px', padding: '1.25rem', position: 'relative', overflow: 'hidden' }}>
            <div className="card-badge" style={{ position: 'absolute', top: 0, right: 0, width: '4px', height: '100%', background: 'var(--accent-cyan)' }} />
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start' }}>
              <span style={{ fontSize: '0.85rem', color: 'var(--text-secondary)', fontWeight: 600 }}>Total Support Reports</span>
              <Layers size={16} color="var(--accent-cyan)" />
            </div>
            <div style={{ fontSize: '2rem', fontWeight: 800, margin: '0.5rem 0', color: 'var(--accent-cyan)' }}>
              {si ? si.total_incidents_7d : 0}
            </div>
            <div style={{ fontSize: '0.75rem', color: 'var(--text-muted)' }}>Accumulated across past 7 days</div>
          </div>

          {/* Card 2: Unresolved Incidents */}
          <div className="glass-card" style={{ background: 'var(--black-40)', backdropFilter: 'blur(10px)', border: '1px solid var(--white-05)', borderRadius: '12px', padding: '1.25rem', position: 'relative', overflow: 'hidden' }}>
            <div className="card-badge" style={{ position: 'absolute', top: 0, right: 0, width: '4px', height: '100%', background: (si && si.unresolved > 0) ? 'var(--accent-rose)' : 'var(--accent-emerald)' }} />
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start' }}>
              <span style={{ fontSize: '0.85rem', color: 'var(--text-secondary)', fontWeight: 600 }}>Active Unresolved Incidents</span>
              <AlertTriangle size={16} color={(si && si.unresolved > 0) ? 'var(--accent-rose)' : 'var(--accent-emerald)'} />
            </div>
            <div style={{ fontSize: '2rem', fontWeight: 800, margin: '0.5rem 0', color: (si && si.unresolved > 0) ? 'var(--accent-rose)' : 'var(--accent-emerald)' }}>
              {si ? si.unresolved : 0}
            </div>
            <div style={{ fontSize: '0.75rem', color: 'var(--text-muted)' }}>Requires operator or system remediation</div>
          </div>

          {/* Card 3: Impacted Users */}
          <div className="glass-card" style={{ background: 'var(--black-40)', backdropFilter: 'blur(10px)', border: '1px solid var(--white-05)', borderRadius: '12px', padding: '1.25rem', position: 'relative', overflow: 'hidden' }}>
            <div className="card-badge" style={{ position: 'absolute', top: 0, right: 0, width: '4px', height: '100%', background: 'var(--accent-purple)' }} />
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start' }}>
              <span style={{ fontSize: '0.85rem', color: 'var(--text-secondary)', fontWeight: 600 }}>Distinct Users Supported</span>
              <Users size={16} color="var(--accent-purple)" />
            </div>
            <div style={{ fontSize: '2rem', fontWeight: 800, margin: '0.5rem 0', color: 'var(--accent-purple)' }}>
              {si ? si.distinct_users : 0}
            </div>
            <div style={{ fontSize: '0.75rem', color: 'var(--text-muted)' }}>Unique users interacting with help desk</div>
          </div>

          {/* Card 4: Top Severity */}
          <div className="glass-card" style={{ background: 'var(--black-40)', backdropFilter: 'blur(10px)', border: '1px solid var(--white-05)', borderRadius: '12px', padding: '1.25rem', position: 'relative', overflow: 'hidden' }}>
            <div className="card-badge" style={{ 
              position: 'absolute', top: 0, right: 0, width: '4px', height: '100%', 
              background: si?.top_severity === 'Critical' ? 'var(--accent-rose)' : si?.top_severity === 'High' ? 'var(--accent-amber)' : 'var(--text-secondary)'
            }} />
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start' }}>
              <span style={{ fontSize: '0.85rem', color: 'var(--text-secondary)', fontWeight: 600 }}>Peak Severity Node</span>
              <CheckCircle size={16} color="var(--text-secondary)" />
            </div>
            <div style={{ fontSize: '1.5rem', fontWeight: 800, margin: '0.85rem 0 0.5rem 0', color: 'var(--text-secondary)' }}>
              {si ? si.top_severity : "None"}
            </div>
            <div style={{ fontSize: '0.75rem', color: 'var(--text-muted)' }}>Highest incident alert level registered</div>
          </div>
        </div>
      </section>

      {/* Resource Health Metrics */}
      <div className="health-metrics-grid" style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(350px, 1fr))', gap: '1.5rem' }}>
        
        {/* Memory & Disk Telemetry */}
        <div className="panel" style={{ background: 'var(--black-40)', backdropFilter: 'blur(10px)', border: '1px solid var(--white-05)', borderRadius: '12px', padding: '1.5rem', display: 'flex', flexDirection: 'column', gap: '1.25rem' }}>
          <h4 style={{ margin: 0, fontSize: '1rem', fontWeight: 600, display: 'flex', alignItems: 'center', gap: '0.5rem', borderBottom: '1px solid var(--white-05)', paddingBottom: '0.5rem' }}>
            <Cpu size={16} color="var(--accent-cyan)" />
            Resource Telemetry
          </h4>

          {/* CPU Bar */}
          <div style={{ display: 'flex', flexDirection: 'column', gap: '0.5rem' }}>
            <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '0.85rem' }}>
              <span style={{ color: 'var(--text-secondary)' }}>CPU Utilization</span>
              <span style={{ color: 'var(--accent-cyan)', fontWeight: 600 }}>{cpuVal}%</span>
            </div>
            <div style={{ height: '8px', background: 'var(--white-05)', borderRadius: '4px', overflow: 'hidden' }}>
              <div style={{ height: '100%', width: `${cpuVal}%`, background: 'var(--accent-cyan)', borderRadius: '4px', transition: 'width 0.5s ease-out' }} />
            </div>
          </div>

          {/* Memory Bar */}
          <div style={{ display: 'flex', flexDirection: 'column', gap: '0.5rem' }}>
            <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '0.85rem' }}>
              <span style={{ color: 'var(--text-secondary)' }}>Resident Memory (RAM)</span>
              <span style={{ color: 'var(--accent-purple)', fontWeight: 600 }}>{memUsed} MB / {memTotal} MB</span>
            </div>
            <div style={{ height: '8px', background: 'var(--white-05)', borderRadius: '4px', overflow: 'hidden' }}>
              <div style={{ height: '100%', width: `${memPercent}%`, background: 'var(--accent-purple)', borderRadius: '4px', transition: 'width 0.5s ease-out' }} />
            </div>
          </div>

          {/* Disk Bar */}
          <div style={{ display: 'flex', flexDirection: 'column', gap: '0.5rem' }}>
            <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '0.85rem' }}>
              <span style={{ color: 'var(--text-secondary)' }}>Local Disk (Free Space)</span>
              <span style={{ color: 'var(--accent-emerald)', fontWeight: 600 }}>{health?.disk_free_gb || 0} GB Free / {health?.total_disk_gb || 0} GB</span>
            </div>
            {health && (
              <div style={{ height: '8px', background: 'var(--white-05)', borderRadius: '4px', overflow: 'hidden' }}>
                <div style={{ height: '100%', width: `${health.total_disk_gb > 0 ? Math.round(((health.total_disk_gb - health.disk_free_gb) / health.total_disk_gb) * 100) : 0}%`, background: 'var(--accent-emerald)', borderRadius: '4px' }} />
              </div>
            )}
          </div>
        </div>

        {/* Component Integrity states (LLM Circuit Breaker, LoRA Autotuner, etc.) */}
        <div className="panel" style={{ background: 'var(--black-40)', backdropFilter: 'blur(10px)', border: '1px solid var(--white-05)', borderRadius: '12px', padding: '1.5rem', display: 'flex', flexDirection: 'column', gap: '1.25rem' }}>
          <h4 style={{ margin: 0, fontSize: '1rem', fontWeight: 600, display: 'flex', alignItems: 'center', gap: '0.5rem', borderBottom: '1px solid var(--white-05)', paddingBottom: '0.5rem' }}>
            <Server size={16} color="var(--accent-purple)" />
            Component Integrity Status
          </h4>

          {/* LLM Circuit Breaker Status */}
          <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', padding: '0.75rem', background: 'var(--black-20)', borderRadius: '8px', border: '1px solid var(--white-05)' }}>
            <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
              <Database size={14} color="var(--accent-cyan)" />
              <span style={{ fontSize: '0.9rem', color: 'var(--text-secondary)' }}>LLM Circuit Breaker</span>
            </div>
            <div style={{ display: 'flex', alignItems: 'center', gap: '0.35rem' }}>
              <span style={{ 
                width: '8px', height: '8px', borderRadius: '50%', 
                background: cb?.state === 'Closed' ? 'var(--accent-emerald)' : 'var(--accent-rose)' 
              }} />
              <span style={{ fontSize: '0.85rem', fontWeight: 700, color: cb?.state === 'Closed' ? 'var(--accent-emerald)' : 'var(--accent-rose)' }}>
                {cb ? cb.state : "Offline"}
              </span>
            </div>
          </div>

          {/* LoRA Engine Autotuner Status */}
          <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', padding: '0.75rem', background: 'var(--black-20)', borderRadius: '8px', border: '1px solid var(--white-05)' }}>
            <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
              <Activity size={14} color="var(--accent-purple)" />
              <span style={{ fontSize: '0.9rem', color: 'var(--text-secondary)' }}>LoRA Autotuner</span>
            </div>
            <div style={{ display: 'flex', alignItems: 'center', gap: '0.35rem' }}>
              <span style={{ 
                width: '8px', height: '8px', borderRadius: '50%', 
                background: lora?.status === 'ready' ? 'var(--accent-emerald)' : 'var(--text-secondary)' 
              }} />
              <span style={{ fontSize: '0.85rem', fontWeight: 700, color: lora?.status === 'ready' ? 'var(--accent-emerald)' : 'var(--text-secondary)' }}>
                {lora ? lora.status : "Offline"}
              </span>
            </div>
          </div>
        </div>

      </div>
    </div>
  );
}

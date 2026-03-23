import { useState, useEffect } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { Play, CheckCircle, Clock, Check, BrainCircuit, Activity, Zap, Cpu, Network, ShieldCheck, AlertTriangle, WifiOff, RefreshCw } from 'lucide-react';
import { AgentStats } from '../types';
import { API_BASE } from '../config';
import { getAuthHeaders } from '../lib/auth';

interface DemoViewProps {
  stats: AgentStats;
  lastEvent: any;
  isConnected: boolean;
}

const DEMO_STEPS_META = [
  { step: 1, title: "Intent Generation", icon: <BrainCircuit size={18}/> },
  { step: 2, title: "Trend Analysis", icon: <Activity size={18}/> },
  { step: 3, title: "Gig Publishing", icon: <Network size={18}/> },
  { step: 4, title: "Bidding Simulation", icon: <Zap size={18}/> },
  { step: 5, title: "Acceptance Simulation", icon: <ShieldCheck size={18}/> },
  { step: 6, title: "Delivery Simulation", icon: <Cpu size={18}/> },
  { step: 7, title: "Settlement & Karma", icon: <Clock size={18}/> },
  { step: 8, title: "Evolution Complete", icon: <CheckCircle size={18}/> }
];

export default function DemoView({ stats, lastEvent, isConnected }: DemoViewProps) {
  const [currentStep, setCurrentStep] = useState<number>(0);
  const [messages, setMessages] = useState<Record<number, string>>({});
  const [isRunning, setIsRunning] = useState(false);
  const [fakeKarma, setFakeKarma] = useState(stats.resonance);
  const [error, setError] = useState<string | null>(null);
  const [debugLog, setDebugLog] = useState<string[]>([]);
  
  const addLog = (msg: string) => {
    const ts = new Date().toLocaleTimeString();
    setDebugLog(prev => [`[${ts}] ${msg}`, ...prev].slice(0, 20));
  };

  useEffect(() => {
    if (!lastEvent) return;
    addLog(`SSE Event: type=${lastEvent.type}`);
    
    if (lastEvent.type === 'plugin_event') {
      const data = lastEvent.data;
      addLog(`PluginEvent: plugin=${data?.plugin_name}, event=${data?.event_type}`);
      
      if (data?.plugin_name === 'AutonomousDemo') {
        const payload = data?.payload || data;
        const step = payload?.step;
        const message = payload?.message;
        
        if (step) {
          addLog(`Demo Step ${step}: ${message}`);
          setCurrentStep(step);
          setIsRunning(step < 8);
          setMessages(prev => ({ ...prev, [step]: message }));
          setError(null);
          
          if (step === 7 || step === 8) {
            setFakeKarma(prev => prev + 15);
          }
        }
      }
    }
  }, [lastEvent]);

  const startDemo = async () => {
    setError(null);
    setIsRunning(true);
    setCurrentStep(0);
    setMessages({});
    addLog('Starting demo...');
    
    try {
      const headers = getAuthHeaders();
      addLog(`POST ${API_BASE}/api/v1/demo/start (Auth: ${headers['Authorization'] ? 'Bearer ***' : 'NONE'})`);
      
      const res = await fetch(`${API_BASE}/api/v1/demo/start`, {
        method: 'POST',
        headers
      });
      
      if (!res.ok) {
        const body = await res.text().catch(() => '');
        const errMsg = `HTTP ${res.status}: ${res.statusText}${body ? ` — ${body}` : ''}`;
        addLog(`ERROR: ${errMsg}`);
        setError(errMsg);
        setIsRunning(false);
        return;
      }
      
      const data = await res.json().catch(() => null);
      addLog(`Response OK: ${JSON.stringify(data)}`);
      
      if (!isConnected) {
        addLog('WARNING: SSE not connected — steps will not update in real-time');
        setError('⚠️ SSE未接続: デモは開始されましたが、リアルタイム更新ができません。Settings でAPIトークンを確認してください。');
      }
    } catch (err: any) {
      const errMsg = err.message || 'Network error';
      addLog(`FETCH ERROR: ${errMsg}`);
      setError(`接続エラー: ${errMsg}`);
      setIsRunning(false);
    }
  };

  return (
    <div className="glass-panel" style={{ padding: '2rem', minHeight: '80vh', display: 'flex', flexDirection: 'column', gap: '2rem' }}>
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
        <div>
          <h2 style={{ marginBottom: '0.5rem', color: 'var(--accent-cyan)' }}>Autonomous AI Economy Demo</h2>
          <p style={{ color: 'var(--text-secondary)' }}>Observe the 60-second autonomous evolution cycle: Sense → Market → Deliver → Evolve.</p>
        </div>
        <button 
          className="panel-button" 
          onClick={startDemo} 
          disabled={isRunning}
          style={{ 
            display: 'flex', alignItems: 'center', gap: '8px', 
            padding: '0.75rem 1.5rem', fontSize: '1rem',
            background: isRunning ? 'var(--bg-panel)' : 'var(--accent-cyan)',
            color: isRunning ? 'var(--text-muted)' : '#000',
            fontWeight: 'bold'
          }}
        >
          {isRunning ? <Clock className="ani-pulse"/> : <Play />}
          {isRunning ? 'Demo in Progress...' : 'Start Demo'}
        </button>
      </div>

      {/* SSE Connection Warning */}
      {!isConnected && (
        <motion.div 
          initial={{ opacity: 0, y: -10 }} animate={{ opacity: 1, y: 0 }}
          style={{ 
            display: 'flex', alignItems: 'center', gap: '12px', padding: '1rem 1.5rem',
            background: 'rgba(255, 170, 50, 0.08)', border: '1px solid rgba(255, 170, 50, 0.3)',
            borderRadius: '12px', color: 'var(--accent-amber)', fontSize: '0.9rem'
          }}
        >
          <WifiOff size={20} />
          <div style={{ flex: 1 }}>
            <strong>SSE 未接続</strong>: Samsara Hub との接続が確立されていません。デモのリアルタイム更新には SSE 接続が必要です。
            <br/>
            <span style={{ fontSize: '0.8rem', color: 'var(--text-secondary)' }}>
              Settingsで正しい API Secret を入力してください（開発環境: <code style={{ background: 'rgba(255,255,255,0.1)', padding: '2px 6px', borderRadius: '4px' }}>mock_valid_token_dev</code>）
            </span>
          </div>
        </motion.div>
      )}

      {/* Error Banner */}
      <AnimatePresence>
        {error && (
          <motion.div 
            initial={{ opacity: 0, height: 0 }} animate={{ opacity: 1, height: 'auto' }} exit={{ opacity: 0, height: 0 }}
            style={{ 
              display: 'flex', alignItems: 'flex-start', gap: '12px', padding: '1rem 1.5rem',
              background: 'rgba(255, 77, 109, 0.08)', border: '1px solid rgba(255, 77, 109, 0.3)',
              borderRadius: '12px', color: '#ff4d6d', fontSize: '0.9rem'
            }}
          >
            <AlertTriangle size={20} style={{ flexShrink: 0, marginTop: '2px' }} />
            <div style={{ flex: 1 }}>
              <strong>Error</strong>: {error}
            </div>
            <button onClick={() => setError(null)} style={{ background: 'none', border: 'none', color: '#ff4d6d', cursor: 'pointer', padding: '4px' }}>✕</button>
          </motion.div>
        )}
      </AnimatePresence>

      <div style={{ display: 'grid', gridTemplateColumns: 'minmax(300px, 1fr) 400px', gap: '2rem', flex: 1 }}>
        
        {/* Left: Timeline */}
        <div className="glass-panel timeline" style={{ padding: '1.5rem', background: 'rgba(0,0,0,0.3)' }}>
          <h3 style={{ marginBottom: '1.5rem', borderBottom: '1px solid var(--border-color)', paddingBottom: '0.5rem' }}>Execution Timeline</h3>
          
          <div style={{ display: 'flex', flexDirection: 'column', gap: '1rem' }}>
            {DEMO_STEPS_META.map((meta) => {
              const num = meta.step;
              const isActive = currentStep === num;
              const isPast = currentStep > num || currentStep === 8;
              const message = messages[num];
              
              let color = 'var(--text-muted)';
              if (isActive) color = 'var(--accent-cyan)';
              if (isPast) color = 'var(--accent-emerald)';
              
              return (
                <div key={num} style={{ display: 'flex', gap: '1rem', opacity: currentStep === 0 || isActive || isPast ? 1 : 0.4, transition: 'all 0.3s' }}>
                  <div style={{ 
                    width: '32px', height: '32px', borderRadius: '50%', 
                    background: isActive ? 'rgba(0,242,255,0.2)' : isPast ? 'rgba(16,185,129,0.2)' : 'rgba(255,255,255,0.05)',
                    border: `1px solid ${color}`,
                    display: 'flex', alignItems: 'center', justifyContent: 'center',
                    color: color,
                    flexShrink: 0,
                    boxShadow: isActive ? '0 0 10px rgba(0,242,255,0.5)' : 'none'
                  }}>
                    {isPast ? <Check size={16} /> : meta.icon}
                  </div>
                  <div style={{ display: 'flex', flexDirection: 'column' }}>
                    <div style={{ fontWeight: 'bold', color: isActive || isPast ? 'var(--text-primary)' : 'var(--text-muted)' }}>
                      Step {num}: {meta.title}
                    </div>
                    {message && (
                      <motion.div initial={{ opacity: 0, height: 0 }} animate={{ opacity: 1, height: 'auto' }} style={{ fontSize: '0.85rem', color: 'var(--text-secondary)', marginTop: '4px' }}>
                        {message}
                      </motion.div>
                    )}
                  </div>
                </div>
              );
            })}
          </div>
        </div>

        {/* Right: Agent Status & Graphs */}
        <div style={{ display: 'flex', flexDirection: 'column', gap: '1.5rem' }}>
          
          <div className="glass-panel" style={{ padding: '1.5rem' }}>
            <h4 style={{ marginBottom: '1rem', color: 'var(--accent-purple)' }}>Agent Status Array</h4>
            
            <div style={{ display: 'flex', gap: '1rem', flexDirection: 'column' }}>
              <div style={{ padding: '1rem', border: '1px solid rgba(0,242,255,0.3)', borderRadius: '8px', background: currentStep > 0 && currentStep <= 4 ? 'rgba(0,242,255,0.05)' : 'transparent', transition: 'all 0.5s' }}>
                <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: '0.5rem' }}>
                  <span style={{ fontWeight: 'bold' }}>Agent A (Requester)</span>
                  <span style={{ color: 'var(--accent-cyan)' }}>Lvl {stats.level}</span>
                </div>
                <div style={{ fontSize: '0.8rem', color: 'var(--text-secondary)' }}>
                  State: {currentStep === 0 ? 'Idle' : currentStep < 3 ? 'Analyzing Intents' : currentStep < 5 ? 'Awaiting Bids' : currentStep < 7 ? 'Escrow Locked' : 'Harvesting Karma'}
                </div>
              </div>

              <AnimatePresence>
                {currentStep >= 4 && (
                  <motion.div 
                    initial={{ opacity: 0, y: 10 }} animate={{ opacity: 1, y: 0 }} 
                    style={{ padding: '1rem', border: '1px solid rgba(188,140,255,0.3)', borderRadius: '8px', background: currentStep >= 4 && currentStep < 7 ? 'rgba(188,140,255,0.05)' : 'transparent', transition: 'all 0.5s' }}
                  >
                    <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: '0.5rem' }}>
                      <span style={{ fontWeight: 'bold' }}>Swarm Agent B (Deliverer)</span>
                      <span style={{ color: 'var(--accent-purple)' }}>External</span>
                    </div>
                    <div style={{ fontSize: '0.8rem', color: 'var(--text-secondary)' }}>
                      State: {currentStep < 5 ? 'Bidding' : currentStep < 6 ? 'Delivering Artifact' : 'Task Completed'}
                    </div>
                  </motion.div>
                )}
              </AnimatePresence>
            </div>
          </div>

          <div className="glass-panel" style={{ padding: '1.5rem', flex: 1 }}>
            <h4 style={{ marginBottom: '1rem', color: 'var(--accent-emerald)' }}>Evolution Pulse (Karma)</h4>
            
            <div style={{ height: '120px', display: 'flex', alignItems: 'flex-end', gap: '8px' }}>
              {[...Array(12)].map((_, i) => {
                const height = i < 10 ? 20 + Math.random() * 20 : (isRunning && currentStep >= 7 ? 60 + Math.random() * 40 : 20 + Math.random() * 10);
                return (
                  <motion.div 
                    key={i}
                    initial={{ height: 10 }}
                    animate={{ height: `${height}%` }}
                    transition={{ duration: 1 }}
                    style={{ 
                      flex: 1, 
                      background: i >= 10 && currentStep >= 7 ? 'var(--accent-emerald)' : 'rgba(255,255,255,0.1)',
                      borderRadius: '4px' 
                    }}
                  />
                );
              })}
            </div>
            
            <div style={{ marginTop: '1rem', display: 'flex', justifyContent: 'space-between', fontSize: '0.85rem' }}>
              <span style={{ color: 'var(--text-muted)' }}>Resonance Buffer</span>
              <motion.span style={{ color: 'var(--accent-emerald)', fontWeight: 'bold', fontSize: '1.2rem' }}>
                {fakeKarma} µ
              </motion.span>
            </div>
          </div>

          {/* Debug Console */}
          <div className="glass-panel" style={{ padding: '1rem', maxHeight: '180px', overflow: 'auto' }}>
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '0.5rem' }}>
              <h4 style={{ fontSize: '0.75rem', color: 'var(--text-muted)', textTransform: 'uppercase', letterSpacing: '0.1em' }}>
                Debug Console
              </h4>
              <button 
                onClick={() => setDebugLog([])} 
                style={{ background: 'none', border: 'none', color: 'var(--text-muted)', cursor: 'pointer', fontSize: '0.7rem' }}
              >
                <RefreshCw size={12} /> Clear
              </button>
            </div>
            <div style={{ fontFamily: 'monospace', fontSize: '0.7rem', color: 'var(--text-secondary)', lineHeight: '1.6' }}>
              {debugLog.length === 0 ? (
                <div style={{ color: 'var(--text-muted)', fontStyle: 'italic' }}>No events yet. Click "Start Demo" to begin.</div>
              ) : (
                debugLog.map((log, i) => (
                  <div key={i} style={{ 
                    color: log.includes('ERROR') ? '#ff4d6d' : log.includes('WARNING') ? 'var(--accent-amber)' : 'var(--text-secondary)',
                    borderBottom: '1px solid rgba(255,255,255,0.03)', padding: '2px 0'
                  }}>
                    {log}
                  </div>
                ))
              )}
            </div>
          </div>
          
        </div>
      </div>
    </div>
  );
}

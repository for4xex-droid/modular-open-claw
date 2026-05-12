/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import { useState, useEffect } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { Play, CheckCircle, Clock, Check, BrainCircuit, Activity, Zap, Cpu, Network, ShieldCheck, AlertTriangle, WifiOff, RefreshCw } from 'lucide-react';
import { AgentStats } from '../types';
import { API_BASE } from '../config';
import { getAuthHeaders } from '../lib/auth';
import { useTranslation } from '../i18n';

interface DemoViewProps {
  stats: AgentStats;
  lastEvent: any;
  isConnected: boolean;
}

export default function DemoView({ stats, lastEvent, isConnected }: DemoViewProps) {
  const { t } = useTranslation();

  const DEMO_STEPS_META = [
    { step: 1, title: t('demo.steps.intentGeneration'), icon: <BrainCircuit size={18}/> },
    { step: 2, title: t('demo.steps.trendAnalysis'), icon: <Activity size={18}/> },
    { step: 3, title: t('demo.steps.gigPublishing'), icon: <Network size={18}/> },
    { step: 4, title: t('demo.steps.biddingSimulation'), icon: <Zap size={18}/> },
    { step: 5, title: t('demo.steps.acceptanceSimulation'), icon: <ShieldCheck size={18}/> },
    { step: 6, title: t('demo.steps.deliverySimulation'), icon: <Cpu size={18}/> },
    { step: 7, title: t('demo.steps.settlementKarma'), icon: <Clock size={18}/> },
    { step: 8, title: t('demo.steps.evolutionComplete'), icon: <CheckCircle size={18}/> }
  ];

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
      // Intentional: Using getAuthHeaders() + raw fetch (not authenticatedFetch)
      // because this pattern is the project-wide SSE idiom — see also:
      // useSystemVitality.tsx:86, useModelStatus.ts:58
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
    } catch (err: unknown) {
      const errMsg = err instanceof Error ? err.message : 'Network error';
      addLog(`FETCH ERROR: ${errMsg}`);
      setError(`接続エラー: ${errMsg}`);
      setIsRunning(false);
    }
  };

  return (
    <div className="glass-panel" style={{ padding: '2rem', minHeight: '80vh', display: 'flex', flexDirection: 'column', gap: '2rem' }}>
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
        <div>
          <h2 style={{ marginBottom: '0.5rem', color: 'var(--accent-cyan)' }}>{t('demo.title')}</h2>
          <p style={{ color: 'var(--text-secondary)' }}>{t('demo.description')}</p>
        </div>
        <button 
          className="panel-button" 
          onClick={startDemo} 
          disabled={isRunning}
          style={{ 
            display: 'flex', alignItems: 'center', gap: '8px', 
            padding: '0.75rem 1.5rem', fontSize: '1rem',
            background: isRunning ? 'var(--bg-panel)' : 'var(--accent-cyan)',
            color: isRunning ? 'var(--text-muted)' : 'var(--bg-primary)',
            fontWeight: 'bold'
          }}
        >
          {isRunning ? <Clock className="ani-pulse"/> : <Play />}
          {isRunning ? t('demo.running') : t('demo.startDemo')}
        </button>
      </div>

      {/* SSE Connection Warning */}
      {!isConnected && (
        <motion.div 
          initial={{ opacity: 0, y: -10 }} animate={{ opacity: 1, y: 0 }}
          style={{ 
            display: 'flex', alignItems: 'center', gap: '12px', padding: '1rem 1.5rem',
            background: 'var(--accent-amber-10)', border: '1px solid var(--accent-amber-30)',
            borderRadius: '12px', color: 'var(--accent-amber)', fontSize: '0.9rem'
          }}
        >
          <WifiOff size={20} />
          <div style={{ flex: 1 }}>
            <strong>{t('demo.sseWarning').split(':')[0]}</strong>: {t('demo.sseWarning').split(':').slice(1).join(':')}
            <br/>
            <span style={{ fontSize: '0.8rem', color: 'var(--text-secondary)' }}>
              {t('demo.sseHint')}
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
              background: 'var(--accent-rose-10)', border: '1px solid var(--accent-rose-30)',
              borderRadius: '12px', color: 'var(--accent-rose)', fontSize: '0.9rem'
            }}
          >
            <AlertTriangle size={20} style={{ flexShrink: 0, marginTop: '2px' }} />
            <div style={{ flex: 1 }}>
              <strong>Error</strong>: {error}
            </div>
            <button onClick={() => setError(null)} style={{ background: 'none', border: 'none', color: 'var(--accent-rose)', cursor: 'pointer', padding: '4px' }}>✕</button>
          </motion.div>
        )}
      </AnimatePresence>

      <div style={{ display: 'grid', gridTemplateColumns: 'minmax(300px, 1fr) 400px', gap: '2rem', flex: 1 }}>
        
        {/* Left: Timeline */}
        <div className="glass-panel timeline" style={{ padding: '1.5rem', background: 'var(--black-30)' }}>
          <h3 style={{ marginBottom: '1.5rem', borderBottom: '1px solid var(--border-color)', paddingBottom: '0.5rem' }}>{t('demo.executionTimeline')}</h3>
          
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
                    background: isActive ? 'var(--accent-cyan-20)' : isPast ? 'var(--accent-emerald-20)' : 'var(--white-05)',
                    border: `1px solid ${color}`,
                    display: 'flex', alignItems: 'center', justifyContent: 'center',
                    color: color,
                    flexShrink: 0,
                    boxShadow: isActive ? '0 0 10px var(--accent-cyan-70)' : 'none'
                  }}>
                    {isPast ? <Check size={16} /> : meta.icon}
                  </div>
                  <div style={{ display: 'flex', flexDirection: 'column' }}>
                    <div style={{ fontWeight: 'bold', color: isActive || isPast ? 'var(--text-primary)' : 'var(--text-muted)' }}>
                      {t('demo.step')} {num}: {meta.title}
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
            <h4 style={{ marginBottom: '1rem', color: 'var(--accent-purple)' }}>{t('demo.agentStatusArray')}</h4>
            
            <div style={{ display: 'flex', gap: '1rem', flexDirection: 'column' }}>
              <div style={{ padding: '1rem', border: '1px solid var(--accent-cyan-30)', borderRadius: '8px', background: currentStep > 0 && currentStep <= 4 ? 'var(--accent-cyan-05)' : 'transparent', transition: 'all 0.5s' }}>
                <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: '0.5rem' }}>
                  <span style={{ fontWeight: 'bold' }}>{t('demo.agentA')}</span>
                  <span style={{ color: 'var(--accent-cyan)' }}>Lvl {stats.level}</span>
                </div>
                <div style={{ fontSize: '0.8rem', color: 'var(--text-secondary)' }}>
                  {t('demo.status')}: {currentStep === 0 ? t('demo.stateIdle') : currentStep < 3 ? t('demo.stateAnalyzing') : currentStep < 5 ? t('demo.stateAwaiting') : currentStep < 7 ? t('demo.stateEscrow') : t('demo.stateHarvesting')}
                </div>
              </div>

              <AnimatePresence>
                {currentStep >= 4 && (
                  <motion.div 
                    initial={{ opacity: 0, y: 10 }} animate={{ opacity: 1, y: 0 }} 
                    style={{ padding: '1rem', border: '1px solid var(--accent-purple-30)', borderRadius: '8px', background: currentStep >= 4 && currentStep < 7 ? 'var(--accent-purple-05)' : 'transparent', transition: 'all 0.5s' }}
                  >
                    <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: '0.5rem' }}>
                      <span style={{ fontWeight: 'bold' }}>{t('demo.agentB')}</span>
                      <span style={{ color: 'var(--accent-purple)' }}>{t('demo.external')}</span>
                    </div>
                    <div style={{ fontSize: '0.8rem', color: 'var(--text-secondary)' }}>
                      {t('demo.status')}: {currentStep < 5 ? t('demo.stateBidding') : currentStep < 6 ? t('demo.stateDelivering') : t('demo.stateCompleted')}
                    </div>
                  </motion.div>
                )}
              </AnimatePresence>
            </div>
          </div>

          <div className="glass-panel" style={{ padding: '1.5rem', flex: 1 }}>
            <h4 style={{ marginBottom: '1rem', color: 'var(--accent-emerald)' }}>{t('demo.evolutionPulse')}</h4>
            
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
                      background: i >= 10 && currentStep >= 7 ? 'var(--accent-emerald)' : 'var(--white-10)',
                      borderRadius: '4px' 
                    }}
                  />
                );
              })}
            </div>
            
            <div style={{ marginTop: '1rem', display: 'flex', justifyContent: 'space-between', fontSize: '0.85rem' }}>
              <span style={{ color: 'var(--text-muted)' }}>{t('demo.resonanceBuffer')}</span>
              <motion.span style={{ color: 'var(--accent-emerald)', fontWeight: 'bold', fontSize: '1.2rem' }}>
                {fakeKarma} µ
              </motion.span>
            </div>
          </div>

          {/* Debug Console */}
          <div className="glass-panel" style={{ padding: '1rem', maxHeight: '180px', overflow: 'auto' }}>
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '0.5rem' }}>
              <h4 style={{ fontSize: '0.75rem', color: 'var(--text-muted)', textTransform: 'uppercase', letterSpacing: '0.1em' }}>
                {t('demo.debugConsole')}
              </h4>
              <button 
                onClick={() => setDebugLog([])} 
                style={{ background: 'none', border: 'none', color: 'var(--text-muted)', cursor: 'pointer', fontSize: '0.7rem' }}
              >
                <RefreshCw size={12} /> {t('demo.clear')}
              </button>
            </div>
            <div className="font-mono" style={{ fontSize: '0.7rem', color: 'var(--text-secondary)', lineHeight: '1.6' }}>
              {debugLog.length === 0 ? (
                <div style={{ color: 'var(--text-muted)', fontStyle: 'italic' }}>{t('demo.noEvents')}</div>
              ) : (
                debugLog.map((log, i) => (
                  <div key={i} style={{ 
                    color: log.includes('ERROR') ? 'var(--accent-rose)' : log.includes('WARNING') ? 'var(--accent-amber)' : 'var(--text-secondary)',
                    borderBottom: '1px solid var(--white-03)', padding: '2px 0'
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

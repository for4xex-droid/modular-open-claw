/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import React, { useState, useEffect, useRef } from "react";
import { motion, AnimatePresence } from "framer-motion";
import { 
  Wifi, 
  Play, 
  Square, 
  User, 
  Bot, 
  History, 
  Target,
  MessageSquare,
  Network
} from "lucide-react";
import { API_BASE } from "../config";
import { authenticatedFetch } from "../lib/auth";
import { useTranslation } from '../i18n';
import { useToast } from './common/Toast';
import { LoadingState } from './ui/LoadingState';

interface CommuneMessage {
  id: number;
  sender_pubkey: string;
  recipient_pubkey: string;
  topic_id: string;
  content: string;
  created_at: string;
}

interface AutonomousStatus {
  running: boolean;
  config: {
    topic_id: string;
    peer_pubkey: string;
    interval_secs: number;
    max_rounds: number;
  } | null;
}

const CommuneDialogueView: React.FC = () => {
    const { t } = useTranslation();
    const { showToast } = useToast();
  const [messages, setMessages] = useState<CommuneMessage[]>([]);
  const [status, setStatus] = useState<AutonomousStatus | null>(null);
  const [peerPubkey, setPeerPubkey] = useState("PEER_NODE_DEFAULT_B");
  const [topicId, setTopicId] = useState("general_deliberation");
  const [isStarting, setIsStarting] = useState(false);
  const [loading, setLoading] = useState(true);
  
  const scrollRef = useRef<HTMLDivElement>(null);

  const fetchMessages = async () => {
    try {
      const res = await authenticatedFetch(`${API_BASE}/api/commune/list`);
      if (res.ok) {
        const data = await res.json();
        setMessages(data.reverse());
      } else {
        showToast('error', t('commune.loadFailed', { defaultValue: 'Failed to load dialogue messages.' }));
      }
    } catch (e) {
      console.error("Failed to fetch messages", e);
      showToast('error', t('common.networkError', { defaultValue: 'A network error occurred.' }));
    }
  };

  const fetchStatus = async () => {
    try {
      const res = await authenticatedFetch(`${API_BASE}/api/commune/autonomous/status`);
      if (res.ok) {
        const data = await res.json();
        setStatus(data);
      } else {
        showToast('error', t('commune.loadFailed', { defaultValue: 'Failed to load dialogue messages.' }));
      }
    } catch (e) {
      console.error("Failed to fetch autonomous status", e);
      showToast('error', t('common.networkError', { defaultValue: 'A network error occurred.' }));
    }
  };

  const startAutonomous = async () => {
    setIsStarting(true);
    try {
      const res = await authenticatedFetch(`${API_BASE}/api/commune/autonomous/start`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          topic_id: topicId,
          peer_pubkey: peerPubkey,
          interval_secs: 15,
          max_rounds: 20
        })
      });
      if (res.ok) {
        fetchStatus();
      }
    } catch (e) {
      console.error("Failed to start autonomous dialogue", e);
    } finally {
      setIsStarting(false);
    }
  };

  const stopAutonomous = async () => {
    try {
      const res = await authenticatedFetch(`${API_BASE}/api/commune/autonomous/stop`, { method: "POST" });
      if (res.ok) {
        fetchStatus();
      }
    } catch (e) {
      console.error("Failed to stop autonomous dialogue", e);
    }
  };

  useEffect(() => {
    const loadInitial = async () => {
      setLoading(true);
      await Promise.all([fetchMessages(), fetchStatus()]);
      setLoading(false);
    };
    loadInitial();
    const interval = setInterval(() => {
      fetchMessages();
      fetchStatus();
    }, 5000);
    return () => clearInterval(interval);
  }, []);

  useEffect(() => {
    if (scrollRef.current) {
      scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
    }
  }, [messages]);

  return (
    <div className="commune-dialogue-view">
      {/* Main Chat Area */}
      <div className="main-panel commune-chat-panel">
        <div style={{ padding: 'var(--space-md)', borderBottom: '1px solid var(--border-glass)', display: 'flex', justifyContent: 'space-between', alignItems: 'center', background: 'var(--bg-glass-light)' }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: 'var(--space-sm)' }}>
            <Network color="var(--accent-cyan)" size={20} />
            <h3 style={{ margin: 0, fontSize: '1rem', fontWeight: 700 }}>{t('commune.dialogueStream')}</h3>
          </div>
          <div style={{ fontSize: '0.7rem', color: 'var(--text-muted)', display: 'flex', alignItems: 'center', gap: 'var(--space-md)' }}>
            <span style={{ display: 'flex', alignItems: 'center', gap: '0.3rem' }}>
              <Target size={12} /> {t('commune.topic') || 'Topic:'} {status?.config?.topic_id || topicId}
            </span>
            <span style={{ display: 'flex', alignItems: 'center', gap: '0.3rem' }}>
              <Wifi size={12} color={status?.running ? "var(--accent-emerald)" : "var(--text-muted)"} />
              {status?.running ? t('commune.autonomousActive') : t('commune.manualMode')}
            </span>
          </div>
        </div>

        <div 
          ref={scrollRef}
          style={{ 
            flex: 1, 
            overflowY: 'auto', 
            padding: 'var(--space-lg)', 
            display: 'flex', 
            flexDirection: 'column', 
            gap: 'var(--space-md)',
            background: 'var(--black-20)'
          }}
        >
          {loading ? (
            <LoadingState messageKey="loading" />
          ) : messages.length === 0 && (
            <div style={{ height: '100%', display: 'flex', flexDirection: 'column', alignItems: 'center', justifyContent: 'center', color: 'var(--text-muted)', opacity: 0.5 }}>
              <MessageSquare size={48} style={{ marginBottom: '1rem' }} />
              <p>{t('commune.waitingMessages')}</p>
            </div>
          )}

          <AnimatePresence>
            {messages.map((msg) => {
              // heuristic: assume 'self' is the local node. 
              // In a real system, we'd compare with the authenticated public key.
              const isSelf = msg.sender_pubkey === "self" || msg.sender_pubkey.startsWith("SYSTEM_");
              return (
                <motion.div
                  key={msg.id}
                  initial={{ opacity: 0, y: 10 }}
                  animate={{ opacity: 1, y: 0 }}
                  style={{ 
                    display: 'flex', 
                    flexDirection: 'column',
                    alignItems: isSelf ? 'flex-end' : 'flex-start',
                    maxWidth: '85%',
                    alignSelf: isSelf ? 'flex-end' : 'flex-start'
                  }}
                >
                  <div style={{ fontSize: '0.65rem', color: 'var(--text-muted)', marginBottom: '0.2rem', display: 'flex', alignItems: 'center', gap: '0.42rem' }}>
                    {isSelf ? <Bot size={12} color="var(--accent-cyan)" /> : <User size={12} color="var(--accent-purple)" />}
                    {isSelf ? t('commune.localIntelligence') : `${t('commune.peer')} [${msg.sender_pubkey.substring(0, 8)}]`}
                    <span style={{ opacity: 0.5 }}>• {new Date(msg.created_at).toLocaleTimeString()}</span>
                  </div>
                  <div style={{ 
                    padding: '0.8rem 1.2rem', 
                    borderRadius: 'var(--radius-md)', 
                    borderTopRightRadius: isSelf ? 0 : 'var(--radius-md)',
                    borderTopLeftRadius: isSelf ? 'var(--radius-md)' : 0,
                    background: isSelf ? 'var(--accent-cyan-15)' : 'var(--bg-glass-heavy)',
                    border: isSelf ? '1px solid var(--accent-cyan-30)' : '1px solid var(--border-glass)',
                    color: 'var(--text-primary)',
                    lineHeight: 1.5,
                    fontSize: '0.95rem'
                  }}>
                    {msg.content}
                  </div>
                </motion.div>
              );
            })}
          </AnimatePresence>
        </div>
      </div>

      {/* Control Sidebar */}
      <div className="commune-control-sidebar">
        <div className="stat-card" style={{ padding: 'var(--space-md)', textAlign: 'left' }}>
          <h4 style={{ margin: '0 0 var(--space-sm) 0', fontSize: '0.85rem', fontWeight: 800, color: 'var(--accent-cyan)', display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
            <Play size={14} /> {t('commune.autonomousEngine') || 'AUTONOMOUS ENGINE'}
          </h4>
          
          <div style={{ display: 'flex', flexDirection: 'column', gap: 'var(--space-sm)' }}>
             <div className="input-field-container">
               <label style={{ fontSize: '0.7rem', color: 'var(--text-muted)' }}>{t('commune.targetPeer')}</label>
               <input 
                 className="custom-input"
                 value={peerPubkey} 
                 onChange={(e) => setPeerPubkey(e.target.value)} 
                 disabled={status?.running}
               />
             </div>
             <div className="input-field-container">
               <label style={{ fontSize: '0.7rem', color: 'var(--text-muted)' }}>{t('commune.topicIdentity')}</label>
               <input 
                 className="custom-input"
                 value={topicId} 
                 onChange={(e) => setTopicId(e.target.value)} 
                 disabled={status?.running}
               />
             </div>

             {status?.running ? (
               <button 
                onClick={stopAutonomous}
                className="card-hover"
                style={{ 
                    width: '100%', 
                    padding: '0.75rem', 
                    borderRadius: 'var(--radius-sm)', 
                    background: 'var(--accent-rose-10)', 
                    color: 'var(--accent-rose)', 
                    border: '1px solid var(--accent-rose)', 
                    cursor: 'pointer', 
                    fontWeight: 700, 
                    display: 'flex', 
                    alignItems: 'center', 
                    justifyContent: 'center', 
                    gap: '0.5rem',
                    transition: 'all var(--speed-normal)'
                }}
               >
                 <Square size={16} fill="currentColor" /> {t('commune.stopLoop') || 'Stop Autonomous Loop'}
               </button>
             ) : (
               <button 
                onClick={startAutonomous}
                disabled={isStarting}
                className="primary-button"
                style={{ width: '100%', padding: '0.75rem', display: 'flex', alignItems: 'center', justifyContent: 'center', gap: '0.5rem' }}
               >
                 <Play size={16} fill="currentColor" /> {isStarting ? t('commune.initializing') : t('commune.startDialogue')}
               </button>
             )}
          </div>
        </div>

        <div className="stat-card" style={{ padding: 'var(--space-md)', textAlign: 'left', background: 'var(--accent-purple-05)' }}>
          <h4 style={{ margin: '0 0 var(--space-sm) 0', fontSize: '0.85rem', fontWeight: 800, color: 'var(--accent-purple)', display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
            <History size={14} /> {t('commune.protocolStats') || 'PROTOCOL STATS'}
          </h4>
          <div style={{ display: 'flex', flexDirection: 'column', gap: '0.5rem', fontSize: '0.75rem' }}>
            <div style={{ display: 'flex', justifyContent: 'space-between' }}>
              <span style={{ color: 'var(--text-muted)' }}>{t('commune.messagesSent')}:</span>
              <span style={{ fontWeight: 700 }}>{messages.length}</span>
            </div>
            <div style={{ display: 'flex', justifyContent: 'space-between' }}>
              <span style={{ color: 'var(--text-muted)' }}>{t('commune.protocolVersion')}:</span>
              <span style={{ fontWeight: 700 }}>v20-COMMUNE</span>
            </div>
          </div>
        </div>

        <div className="info-box-glass">
          <p>{t('commune.sandboxNote') || 'In Sandbox Mode, AI will fallback to local storage if Sync Hub is offline. Topic constraints are enforced by DialogueManager.'}</p>
        </div>
      </div>
      
      <style>{`
        .commune-dialogue-view {
          display: grid;
          grid-template-columns: 1fr 300px;
          gap: var(--space-lg);
          height: calc(85vh - 100px);
        }
        .commune-chat-panel {
          display: flex;
          flex-direction: column;
          padding: 0;
          overflow: hidden;
        }
        .commune-control-sidebar {
          display: flex;
          flex-direction: column;
          gap: var(--space-md);
        }
        .info-box-glass {
          padding: 1rem;
          background: var(--bg-glass-light);
          border-radius: var(--radius-md);
          border: 1px solid var(--border-glass);
          font-size: 0.7rem;
          color: var(--text-muted);
          line-height: 1.4;
        }
        .custom-input {
          width: 100%;
          background: var(--white-05);
          border: 1px solid var(--border-glass);
          border-radius: 4px;
          padding: 0.5rem;
          color: var(--text-primary);
          font-family: var(--font-mono);
          margin-top: 0.2rem;
          font-size: 0.75rem;
        }
        .custom-input:focus {
          outline: none;
          border-color: var(--accent-cyan);
        }

        @media (max-width: 900px) {
          .commune-dialogue-view {
            grid-template-columns: 1fr;
            height: auto;
          }
          .commune-chat-panel {
            height: 60vh;
          }
        }
      `}</style>
    </div>
  );
};

export default CommuneDialogueView;

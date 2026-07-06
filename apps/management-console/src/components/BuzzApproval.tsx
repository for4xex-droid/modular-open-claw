/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import { useState, useEffect } from "react";
import { motion, AnimatePresence } from "framer-motion";
import {
  XCircle,
  MessageSquare,
  TrendingUp,
  RefreshCw,
  Send,
  X,
  Zap,
} from "lucide-react";
import { API_BASE } from "../config";
import { authenticatedFetch } from "../lib/auth";
import { useTranslation } from '../i18n';
import { useToast } from './common/Toast';
import { LockedOverlay } from './ui/LockedOverlay';

interface BuzzJob {
  id: string;
  category: string;
  status: string;
  output_artifacts: string;
  created_at: string;
  // In a real app we might parse execution_log or other metadata for trend_source and template
}

const BuzzApproval = () => {
  const { t } = useTranslation();
  const { showToast } = useToast();
  const [jobs, setJobs] = useState<BuzzJob[]>([]);
  const [loading, setLoading] = useState(true);
  const [selectedJob, setSelectedJob] = useState<BuzzJob | null>(null);
  const [editContent, setEditContent] = useState("");
  const [processing, setProcessing] = useState(false);

  useEffect(() => {
    fetchPending();
  }, []);

  const fetchPending = async () => {
    setLoading(true);
    try {
      const res = await authenticatedFetch(`${API_BASE}/api/v1/buzz/pending`);
      if (res.ok) {
        const data = await res.json();
        if (Array.isArray(data)) {
          setJobs(data);
        } else {
          setJobs([]);
        }
      }
    } catch (e) {
      console.error("Failed to fetch pending buzz content", e);
    } finally {
      setLoading(false);
    }
  };

  const handleApprove = async () => {
    if (!selectedJob) return;
    setProcessing(true);
    try {
      // If we wanted to save the edited content we'd need another endpoint, but for now we just approve
      const res = await authenticatedFetch(`${API_BASE}/api/v1/buzz/approve/${selectedJob.id}`, {
        method: "POST"
      });
      if (res.ok) {
        showToast('success', t('buzz.approveSuccess', { defaultValue: 'Buzz content approved.' }));
        
        // Then auto-publish
        const pubRes = await authenticatedFetch(`${API_BASE}/api/v1/buzz/publish/${selectedJob.id}`, {
          method: "POST"
        });
        if (pubRes.ok) {
           showToast('success', t('buzz.publishSuccess', { defaultValue: 'Published to X successfully!' }));
        } else {
           showToast('error', t('buzz.publishError', { defaultValue: 'Failed to publish to X.' }));
        }
        
        setJobs(prev => prev.filter(j => j.id !== selectedJob.id));
        setSelectedJob(null);
      } else {
        showToast('error', t('buzz.approveFailed', { defaultValue: 'Failed to approve.' }));
      }
    } catch (e) {
      console.error(e);
      showToast('error', t('buzz.approveFailed', { defaultValue: 'Failed to approve.' }));
    } finally {
      setProcessing(false);
    }
  };

  const handleReject = async () => {
    if (!selectedJob) return;
    setProcessing(true);
    try {
      const res = await authenticatedFetch(`${API_BASE}/api/v1/buzz/reject/${selectedJob.id}`, {
        method: "POST"
      });
      if (res.ok) {
        showToast('success', t('buzz.rejectSuccess', { defaultValue: 'Buzz content rejected.' }));
        setJobs(prev => prev.filter(j => j.id !== selectedJob.id));
        setSelectedJob(null);
      } else {
        showToast('error', t('buzz.rejectFailed', { defaultValue: 'Failed to reject.' }));
      }
    } catch (e) {
      console.error(e);
      showToast('error', t('buzz.rejectFailed', { defaultValue: 'Failed to reject.' }));
    } finally {
      setProcessing(false);
    }
  };

  const generateNew = async () => {
    setProcessing(true);
    try {
      const res = await authenticatedFetch(`${API_BASE}/api/v1/buzz/generate`, {
        method: "POST"
      });
      if (res.ok) {
        showToast('success', t('buzz.generateSuccess', { defaultValue: 'New Buzz generated!' }));
        fetchPending();
      } else {
        showToast('error', t('buzz.generateFailed', { defaultValue: 'Generation failed.' }));
      }
    } catch (e) {
      console.error(e);
    } finally {
      setProcessing(false);
    }
  };

  const openModal = (job: BuzzJob) => {
    setSelectedJob(job);
    setEditContent(job.output_artifacts || "");
  };

  return (
    <LockedOverlay featureNameKey="pro.featureBuzz">
    <div className="vault-container">
      <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: 'var(--space-lg)', alignItems: 'center' }}>
        <h2><Zap size={24} color="var(--accent-cyan)" style={{ verticalAlign: 'middle', marginRight: '8px' }} />
          {t('buzz.title', { defaultValue: 'Buzz Protocol' })}
        </h2>
        <button 
          className="btn-primary" 
          onClick={generateNew} 
          disabled={processing}
          style={{ display: 'flex', alignItems: 'center', gap: '8px' }}
        >
          <RefreshCw size={16} className={processing ? "ani-spin" : ""} />
          {t('buzz.generateNew', { defaultValue: 'Generate New' })}
        </button>
      </div>

      {loading ? (
        <div style={{ padding: 'var(--space-xl)', textAlign: 'center' }}>
          <Zap className="ani-pulse" size={48} color="var(--accent-cyan)" style={{ margin: '0 auto 1.5rem' }} />
          <p style={{ color: 'var(--text-secondary)' }}>{t('buzz.loading', { defaultValue: 'Scanning X algorithms...' })}</p>
        </div>
      ) : jobs.length === 0 ? (
        <div style={{ textAlign: 'center', padding: 'var(--space-xl)', color: 'var(--text-muted)' }}>
          <MessageSquare size={48} style={{ opacity: 0.5, marginBottom: '1rem' }} />
          <p>{t('buzz.noPending', { defaultValue: 'No pending buzz content to approve.' })}</p>
        </div>
      ) : (
        <div className="artifact-grid">
          {jobs.map((job) => (
            <motion.div
              key={job.id}
              layoutId={job.id}
              className="artifact-card"
              onClick={() => openModal(job)}
            >
              <div className="card-header">
                <div className="category-tag">
                  <TrendingUp size={14} />
                  <span>{t('buzz.draftLabel', { defaultValue: 'BUZZ DRAFT' })}</span>
                </div>
                <div className="timestamp">
                  {new Date(job.created_at).toLocaleString()}
                </div>
              </div>

              <div className="card-title" style={{ fontSize: '0.9rem', color: 'var(--text-secondary)', marginBottom: '1rem', fontStyle: 'italic', display: '-webkit-box', WebkitLineClamp: 3, WebkitBoxOrient: 'vertical', overflow: 'hidden' }}>
                "{job.output_artifacts}"
              </div>

              <div className="card-meta">
                <div className="meta-item">
                  <span className="tag">#auto-generated</span>
                </div>
                <div className="meta-item" style={{ color: 'var(--accent-orange)' }}>
                  <Zap size={14} />
                  <span>{t('buzz.awaitingApproval', { defaultValue: 'Awaiting Approval' })}</span>
                </div>
              </div>
            </motion.div>
          ))}
        </div>
      )}

      {/* Details Modal */}
      <AnimatePresence>
        {selectedJob && (
          <>
            <motion.div
              initial={{ opacity: 0 }}
              animate={{ opacity: 1 }}
              exit={{ opacity: 0 }}
              className="modal-backdrop"
              onClick={() => !processing && setSelectedJob(null)}
            />
            <motion.div
              layoutId={selectedJob.id}
              className="artifact-modal"
              style={{ maxWidth: '600px', left: '50%', transform: 'translateX(-50%)', right: 'auto' }}
            >
              <div className="modal-header">
                <div>
                  <div className="category-tag">
                    <TrendingUp size={16} />
                    <span>{t('buzz.approvalLabel', { defaultValue: 'BUZZ APPROVAL' })}</span>
                  </div>
                  <h2 style={{ margin: '0.4rem 0 0', fontSize: '1.2rem' }}>{t('buzz.reviewTitle', { defaultValue: 'Review Content for X' })}</h2>
                </div>
                <button 
                    onClick={() => !processing && setSelectedJob(null)}
                    style={{ background: 'transparent', border: 'none', color: 'var(--text-muted)', cursor: 'pointer' }}
                    disabled={processing}
                    aria-label={t('common.close', { defaultValue: 'Close' })}
                >
                    <X size={24} />
                </button>
              </div>

              <div className="modal-content" style={{ display: 'flex', flexDirection: 'column' }}>
                <div style={{ padding: '2rem' }}>
                  <div style={{ marginBottom: '1rem', display: 'flex', justifyContent: 'space-between', fontSize: '0.8rem', color: 'var(--text-muted)' }}>
                    <span>{t('buzz.editInstruction', { defaultValue: 'Edit content before publishing:' })}</span>
                    <span style={{ color: editContent.length > 280 ? 'var(--accent-rose)' : 'inherit' }}>
                      {editContent.length} / 280 {t('buzz.chars', { defaultValue: 'chars' })}
                    </span>
                  </div>
                  <textarea 
                    value={editContent}
                    onChange={(e) => setEditContent(e.target.value)}
                    style={{
                      width: '100%',
                      height: '150px',
                      background: 'var(--white-05)',
                      border: '1px solid var(--white-10)',
                      borderRadius: 'var(--radius-md)',
                      color: 'white',
                      padding: '1rem',
                      fontFamily: 'inherit',
                      resize: 'none',
                      outline: 'none'
                    }}
                  />
                  
                  <div style={{ marginTop: '2rem', display: 'flex', gap: '1rem' }}>
                    <button 
                      onClick={handleReject}
                      disabled={processing}
                      style={{
                        flex: 1,
                        padding: '1rem',
                        background: 'var(--accent-rose-10)',
                        border: '1px solid var(--accent-rose-30)',
                        color: 'var(--accent-rose)',
                        borderRadius: 'var(--radius-md)',
                        cursor: processing ? 'not-allowed' : 'pointer',
                        display: 'flex',
                        justifyContent: 'center',
                        alignItems: 'center',
                        gap: '8px',
                        fontWeight: 'bold'
                      }}
                    >
                      <XCircle size={18} /> {t('buzz.reject', { defaultValue: 'Reject' })}
                    </button>
                    <button 
                      onClick={handleApprove}
                      disabled={processing || editContent.length > 280}
                      style={{
                        flex: 2,
                        padding: '1rem',
                        background: 'var(--accent-cyan-20)',
                        border: '1px solid var(--accent-cyan)',
                        color: 'var(--accent-cyan)',
                        borderRadius: 'var(--radius-md)',
                        cursor: (processing || editContent.length > 280) ? 'not-allowed' : 'pointer',
                        display: 'flex',
                        justifyContent: 'center',
                        alignItems: 'center',
                        gap: '8px',
                        fontWeight: 'bold'
                      }}
                    >
                      <Send size={18} /> {t('buzz.approveAndPublish', { defaultValue: 'Approve & Publish' })}
                    </button>
                  </div>
                </div>
              </div>
            </motion.div>
          </>
        )}
      </AnimatePresence>

      <style>{`
        /* Reuse styles from ArtifactVault */
        .vault-container { padding: 1rem; }
        .artifact-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(280px, 1fr)); gap: var(--space-md); }
        .artifact-card { background: var(--bg-glass-light); backdrop-filter: blur(10px); border: 1px solid var(--border-glass); border-radius: var(--radius-lg); padding: 1.2rem; cursor: pointer; transition: transform var(--speed-normal), background var(--speed-normal); position: relative; overflow: hidden; }
        .artifact-card:hover { transform: translateY(-4px); background: var(--bg-glass-heavy); border-color: var(--accent-cyan-30); }
        .card-header { display: flex; justify-content: space-between; margin-bottom: 1rem; align-items: center; }
        .category-tag { display: flex; align-items: center; gap: 0.4rem; color: var(--accent-cyan); font-size: 0.7rem; font-weight: 700; letter-spacing: 0.05em; }
        .timestamp { font-size: 0.7rem; color: var(--text-muted); }
        .card-meta { display: flex; gap: 1rem; margin-bottom: 0; }
        .meta-item { display: flex; align-items: center; gap: 0.3rem; color: var(--text-secondary); font-size: 0.75rem; }
        .tag { font-size: 0.7rem; color: var(--accent-purple); background: var(--accent-purple-15); padding: 0.1rem 0.4rem; border-radius: 4px; }
        .btn-primary { background: var(--accent-cyan); color: var(--bg-primary); border: none; padding: 0.5rem 1rem; border-radius: var(--radius-md); font-weight: 600; cursor: pointer; transition: opacity var(--speed-normal); }
        .btn-primary:hover { opacity: 0.9; }
        .btn-primary:disabled { opacity: 0.5; cursor: not-allowed; }
        .modal-backdrop { position: fixed; inset: 0; background: var(--black-80); z-index: 100; backdrop-filter: blur(4px); }
        .artifact-modal { position: fixed; top: 10%; bottom: 10%; background: var(--bg-secondary); border: 1px solid var(--border-glass); border-radius: var(--radius-xl); z-index: 101; display: flex; flex-direction: column; box-shadow: var(--shadow-deep); }
        .modal-header { padding: 1.5rem 2rem; border-bottom: 1px solid var(--border-glass); display: flex; justify-content: space-between; align-items: center; }
        .modal-content { flex: 1; overflow: hidden; }
        textarea:focus { border-color: var(--accent-cyan) !important; }
      `}</style>
    </div>
    </LockedOverlay>
  );
};

export default BuzzApproval;

import { useState, useEffect } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { useTranslation } from '../i18n';
import { AlertTriangle, Check, X, ShieldAlert } from 'lucide-react';
import { useSystemVitality } from '../hooks/useSystemVitality';
import { authenticatedFetch } from '../lib/auth';
import { API_BASE } from '../config';
import './TaskApprovalOverlay.css';

interface PendingJob {
    id: string;
    reason: string;
}

export default function TaskApprovalOverlay() {
    const { t } = useTranslation();
    const { lastEvent } = useSystemVitality();
    
    const [pendingJobs, setPendingJobs] = useState<PendingJob[]>([]);
    const [isSubmitting, setIsSubmitting] = useState(false);
    const [comment, setComment] = useState('');
    const [notification, setNotification] = useState<{type: 'success' | 'error', message: string} | null>(null);

    useEffect(() => {
        if (notification) {
            const timer = setTimeout(() => setNotification(null), 5000);
            return () => clearTimeout(timer);
        }
    }, [notification]);

    // 初期化時、まだ対応していない AwaitingInput のジョブを取得する
    useEffect(() => {
        const fetchAwaitingJobs = async () => {
            try {
                const response = await authenticatedFetch(`${API_BASE}/api/v1/jobs/awaiting-input`);
                if (response.ok) {
                    const jobs = await response.json();
                    // Map to expected format. Default reason if error_message is null.
                    const formattedJobs = jobs.map((j: { id: string; error_message?: string | null }) => ({
                        id: j.id,
                        reason: j.error_message || t('approval.default_reason')
                    }));
                    setPendingJobs(formattedJobs);
                }
            } catch (error) {
                console.error("Failed to fetch awaiting input jobs", error);
            }
        };

        fetchAwaitingJobs();
    }, [t]);

    // SSE 経由で TaskAwaitingInput を受信した場合にリストへ追加
    useEffect(() => {
        if (lastEvent?.type === 'task_awaiting_input' && lastEvent.data && typeof lastEvent.data === 'object') {
            const data = lastEvent.data as Record<string, unknown>;
            if (typeof data.job_id === 'string') {
                const jobId = data.job_id;
                const reason = typeof data.reason === 'string' ? data.reason : t('approval.default_reason');
                
                setPendingJobs(prev => {
                    if (prev.find(j => j.id === jobId)) return prev;
                    return [...prev, { id: jobId, reason }];
                });
            }
        }
    }, [lastEvent, t]);

    const handleAction = async (jobId: string, isApproval: boolean) => {
        setIsSubmitting(true);
        try {
            const payload = {
                status: isApproval ? "approved" : "rejected",
                comments: comment.trim() || undefined
            };

            const response = await authenticatedFetch(`${API_BASE}/api/v1/jobs/${jobId}/review`, {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify(payload)
            });

            if (response.ok || response.status === 409) {
                // Remove the job from the queue (even on 409 Conflict we assume it was already processed)
                setPendingJobs(prev => prev.filter(j => j.id !== jobId));
                setComment('');
            } else {
                console.error(`Failed to submit review for job ${jobId}`, await response.text());
                setNotification({ type: 'error', message: t('approval.error_submit', { status: response.status }) });
            }
        } catch (error) {
            console.error("Error submitting job review", error);
            setNotification({ type: 'error', message: t('approval.error_network') });
        } finally {
            setIsSubmitting(false);
        }
    };

    const showOverlay = pendingJobs.length > 0;
    const currentJob = showOverlay ? pendingJobs[0] : null;

    return (
        <>
            <AnimatePresence>
                {notification && (
                    <motion.div 
                        initial={{ opacity: 0, y: -50 }}
                        animate={{ opacity: 1, y: 0 }}
                        exit={{ opacity: 0, scale: 0.95 }}
                        style={{ 
                            position: 'fixed', 
                            top: 'var(--space-xl)', 
                            right: 'var(--space-xl)', 
                            zIndex: 1100,
                            display: 'flex',
                            alignItems: 'center',
                            gap: 'var(--space-sm)',
                            padding: 'var(--space-sm) var(--space-md)',
                            background: 'var(--bg-glass-heavy)',
                            backdropFilter: 'blur(20px)',
                            border: `1px solid ${notification.type === 'success' ? 'var(--accent-emerald-50)' : 'var(--accent-rose-50)'}`,
                            borderRadius: 'var(--radius-md)',
                            boxShadow: 'var(--shadow-deep)',
                            color: notification.type === 'success' ? 'var(--accent-emerald)' : 'var(--accent-rose)',
                            pointerEvents: 'none'
                        }}
                    >
                        {notification.type === 'success' ? <Check size={20} /> : <AlertTriangle size={20} />}
                        <span style={{ fontWeight: 600 }}>{notification.message}</span>
                    </motion.div>
                )}
            </AnimatePresence>

            <AnimatePresence>
                {showOverlay && currentJob && (
                    <motion.div
                        initial={{ opacity: 0 }}
                        animate={{ opacity: 1 }}
                        exit={{ opacity: 0 }}
                        className="approval-overlay-backdrop"
                    >
                    <motion.div
                        initial={{ scale: 0.95, opacity: 0, y: 20 }}
                        animate={{ scale: 1, opacity: 1, y: 0 }}
                        transition={{ type: 'spring', damping: 25, stiffness: 300 }}
                        className="approval-overlay-modal"
                    >
                        <div className="approval-overlay-glow" />
                        
                        <div className="approval-overlay-content">
                            <div className="approval-overlay-header">
                                <ShieldAlert size={32} />
                                <div className="approval-header-texts">
                                    <h2>{t('approval.title')}</h2>
                                    <p>{t('approval.required')}</p>
                                </div>
                            </div>

                            <div className="approval-overlay-job-box">
                                <div className="approval-job-label">{t('approval.job_id')}</div>
                                <div className="approval-job-id">{currentJob.id}</div>
                                
                                <div className="approval-reason-header">
                                    <AlertTriangle size={16} />
                                    {t('approval.reason')}
                                </div>
                                <div className="approval-reason-text">
                                    {currentJob.reason}
                                </div>
                            </div>

                            <div className="approval-overlay-textarea-wrapper">
                                <textarea
                                    value={comment}
                                    onChange={(e) => setComment(e.target.value)}
                                    placeholder={t('approval.comments')}
                                    className="approval-overlay-textarea"
                                    rows={2}
                                />
                            </div>

                            <div className="approval-overlay-actions">
                                <button
                                    onClick={() => handleAction(currentJob.id, false)}
                                    disabled={isSubmitting}
                                    className="approval-btn approval-btn-reject"
                                >
                                    <X size={16} />
                                    <span className="approval-btn-text">{t('approval.reject')}</span>
                                </button>
                                <button
                                    onClick={() => handleAction(currentJob.id, true)}
                                    disabled={isSubmitting}
                                    className="approval-btn approval-btn-approve"
                                >
                                    <Check size={16} />
                                    <span className="approval-btn-text">{isSubmitting ? t('approval.submitting') : t('approval.approve')}</span>
                                </button>
                            </div>
                        </div>
                        {pendingJobs.length > 1 && (
                            <div className="approval-overlay-footer">
                                {t('approval.more_requests', { count: pendingJobs.length - 1 })}
                            </div>
                        )}
                    </motion.div>
                </motion.div>
            )}
            </AnimatePresence>
        </>
    );
}

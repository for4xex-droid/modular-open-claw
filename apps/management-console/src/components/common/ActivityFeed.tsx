import React, { useMemo } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { useSystemVitality, VitalityEvent } from '../../hooks/useSystemVitality';
import { ShieldAlert, CreditCard, Activity, CheckCircle, XCircle } from 'lucide-react';

interface ActivityFeedProps {
    maxItems?: number;
}

export const ActivityFeed: React.FC<ActivityFeedProps> = ({ maxItems = 5 }) => {
    const { events } = useSystemVitality();

    const feedEvents = useMemo(() => {
        return events.filter(e => 
            e.type === 'commerce_event' || 
            e.type === 'aegis_sentinel' || 
            e.type === 'task_progress' ||
            e.type === 'task_completed' ||
            e.type === 'task_failed'
        ).slice(0, maxItems);
    }, [events, maxItems]);

    if (feedEvents.length === 0) {
        return null;
    }

    return (
        <div style={{
            position: 'absolute',
            top: '5rem',
            right: '1.5rem',
            width: '320px',
            zIndex: 50,
            display: 'flex',
            flexDirection: 'column',
            gap: '0.5rem',
            pointerEvents: 'none'
        }}>
            <AnimatePresence>
                {feedEvents.map((event, index) => {
                    const id = event.id || `${event.type}-${index}`;
                    return (
                        <motion.div
                            key={id}
                            initial={{ opacity: 0, x: 50, scale: 0.9 }}
                            animate={{ opacity: 1, x: 0, scale: 1 }}
                            exit={{ opacity: 0, scale: 0.9 }}
                            style={{
                                background: 'var(--bg-glass-heavy)',
                                border: '1px solid var(--border-glass)',
                                borderRadius: 'var(--radius-md)',
                                padding: '0.75rem 1rem',
                                boxShadow: 'var(--shadow-deep)',
                                backdropFilter: 'blur(8px)',
                                pointerEvents: 'auto',
                                display: 'flex',
                                alignItems: 'flex-start',
                                gap: '0.75rem'
                            }}
                        >
                            <EventIcon type={event.type} />
                            <div style={{ flex: 1, minWidth: 0 }}>
                                <EventContent event={event} />
                            </div>
                        </motion.div>
                    );
                })}
            </AnimatePresence>
        </div>
    );
};

const EventIcon: React.FC<{ type: string }> = ({ type }) => {
    switch (type) {
        case 'commerce_event':
            return <CreditCard size={16} color="var(--accent-emerald)" style={{ marginTop: '2px' }} />;
        case 'aegis_sentinel':
            return <ShieldAlert size={16} color="var(--accent-rose)" style={{ marginTop: '2px' }} />;
        case 'task_progress':
            return <Activity size={16} color="var(--accent-cyan)" style={{ marginTop: '2px' }} />;
        case 'task_completed':
            return <CheckCircle size={16} color="var(--accent-purple)" style={{ marginTop: '2px' }} />;
        case 'task_failed':
            return <XCircle size={16} color="var(--accent-amber)" style={{ marginTop: '2px' }} />;
        default:
            return <Activity size={16} color="var(--text-muted)" style={{ marginTop: '2px' }} />;
    }
};

const EventContent: React.FC<{ event: VitalityEvent }> = ({ event }) => {
    const data = event.data as any;
    if (!data) return <div style={{ fontSize: '0.8rem' }}>{event.type}</div>;
    
    switch (event.type) {
        case 'commerce_event':
            return (
                <>
                    <div style={{ fontSize: '0.75rem', fontWeight: 600, color: 'var(--accent-emerald)', textTransform: 'uppercase' }}>
                        Ledger Tx: {data.event_type}
                    </div>
                    <div style={{ fontSize: '0.8rem', color: 'var(--text-primary)', marginTop: '0.2rem' }}>
                        {data.description || 'Commerce transaction processed'}
                    </div>
                    <div style={{ fontSize: '0.7rem', color: 'var(--text-muted)', marginTop: '0.2rem' }}>
                        {data.amount > 0 ? '+' : ''}{data.amount} {data.currency}
                    </div>
                </>
            );
        case 'aegis_sentinel':
            return (
                <>
                    <div style={{ fontSize: '0.75rem', fontWeight: 600, color: 'var(--accent-rose)', textTransform: 'uppercase' }}>
                        Aegis Block (Lv {data.level})
                    </div>
                    <div style={{ fontSize: '0.8rem', color: 'var(--text-primary)', marginTop: '0.2rem' }}>
                        {data.message}
                    </div>
                </>
            );
        case 'task_progress':
            return (
                <>
                    <div style={{ fontSize: '0.75rem', fontWeight: 600, color: 'var(--accent-cyan)', textTransform: 'uppercase' }}>
                        Task Activity
                    </div>
                    <div style={{ fontSize: '0.8rem', color: 'var(--text-primary)', marginTop: '0.2rem', whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' }}>
                        {data.message}
                    </div>
                </>
            );
        case 'task_completed':
            return (
                <>
                    <div style={{ fontSize: '0.75rem', fontWeight: 600, color: 'var(--accent-purple)', textTransform: 'uppercase' }}>
                        Task Completed
                    </div>
                </>
            );
        case 'task_failed':
            return (
                <>
                    <div style={{ fontSize: '0.75rem', fontWeight: 600, color: 'var(--accent-amber)', textTransform: 'uppercase' }}>
                        Task Failed
                    </div>
                    <div style={{ fontSize: '0.8rem', color: 'var(--text-primary)', marginTop: '0.2rem', whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' }}>
                        {data.error}
                    </div>
                </>
            );
        default:
            return <div style={{ fontSize: '0.8rem' }}>System event occurred</div>;
    }
};

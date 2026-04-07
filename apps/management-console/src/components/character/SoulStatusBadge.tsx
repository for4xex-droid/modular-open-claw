import React from 'react';
import { Shield, Sparkles, Heart } from 'lucide-react';

interface SoulStatusBadgeProps {
    level: number;
    state: string;
    attachmentStyle?: string;
    healthStatus?: string;
}

export const SoulStatusBadge: React.FC<SoulStatusBadgeProps> = ({ 
    level, 
    state, 
    attachmentStyle = 'Secure',
    healthStatus = 'Healthy'
}) => {
    return (
        <>
            <span style={{ background: 'var(--bg-glass-light)', color: 'var(--text-primary)', padding: '0.2rem 0.6rem', borderRadius: '12px', fontSize: '0.75rem', display: 'flex', alignItems: 'center' }}>
                Lvl {level} | {state}
            </span>
            <span style={{ background: 'var(--accent-cyan-glass)', color: 'var(--accent-cyan)', padding: '0.2rem 0.6rem', borderRadius: '12px', fontSize: '0.75rem', display: 'flex', alignItems: 'center', gap: '0.25rem' }}>
                <Shield size={12} /> {attachmentStyle}
            </span>
            <span style={{ background: 'rgba(16, 185, 129, 0.1)', color: 'var(--accent-emerald)', padding: '0.2rem 0.6rem', borderRadius: '12px', fontSize: '0.75rem', display: 'flex', alignItems: 'center', gap: '0.25rem' }}>
                <Heart size={12} /> {healthStatus}
            </span>
            <span style={{ background: 'var(--accent-purple-glass)', color: 'var(--accent-purple)', padding: '0.2rem 0.6rem', borderRadius: '12px', fontSize: '0.75rem', display: 'flex', alignItems: 'center', gap: '0.25rem' }}>
                <Sparkles size={12} /> Curious
            </span>
        </>
    );
};

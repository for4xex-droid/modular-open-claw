import React from 'react';
import { Shield, Sparkles, Heart } from 'lucide-react';
import { useTranslation } from '../../i18n';

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
    const { t } = useTranslation();
    const stateTooltip = t(`soulStatus.state_${state}`, { defaultValue: `Status: ${state}` });
    const attachTooltip = t(`soulStatus.attachment_${attachmentStyle}`, { defaultValue: `Attachment: ${attachmentStyle}` });
    const healthTooltip = t(`soulStatus.health_${healthStatus}`, { defaultValue: `Health: ${healthStatus}` });
    const curiosityTooltip = t(`soulStatus.curiosity_Curious`, { defaultValue: 'Curiosity state' });

    const badgeStyle = (bg: string, color: string): React.CSSProperties => ({
        background: bg,
        color: color,
        padding: '0.2rem 0.6rem',
        borderRadius: '12px',
        fontSize: '0.75rem',
        display: 'flex',
        alignItems: 'center',
        gap: '0.25rem',
        cursor: 'help',
        transition: 'opacity 0.15s',
    });

    return (
        <>
            <span
                data-tooltip={`Lv ${level} | ${state}\n${stateTooltip}`}
                style={badgeStyle('var(--bg-glass-light)', 'var(--text-primary)')}
            >
                Lvl {level} | {state}
            </span>
            <span
                data-tooltip={attachTooltip}
                style={badgeStyle('var(--accent-cyan-glass)', 'var(--accent-cyan)')}
            >
                <Shield size={12} /> {attachmentStyle}
            </span>
            <span
                data-tooltip={healthTooltip}
                style={badgeStyle('var(--accent-emerald-10)', 'var(--accent-emerald)')}
            >
                <Heart size={12} /> {healthStatus}
            </span>
            <span
                data-tooltip={curiosityTooltip}
                style={badgeStyle('var(--accent-purple-glass)', 'var(--accent-purple)')}
            >
                <Sparkles size={12} /> Curious
            </span>
        </>
    );
};

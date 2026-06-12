import React from 'react';
import { useTranslation } from '../../i18n';

interface EkycStatusBadgeProps {
    status: boolean | null;
}

export const EkycStatusBadge: React.FC<EkycStatusBadgeProps> = ({ status }) => {
    const { t } = useTranslation();
    if (status === null) return null;
    
    return status ? (
        <span
            data-tooltip={t('ekyc.verified')}
            style={{ background: 'var(--accent-emerald-10)', color: 'var(--accent-emerald)', padding: '0.2rem 0.6rem', borderRadius: '12px', fontSize: '0.75rem', display: 'flex', alignItems: 'center', cursor: 'help' }}
        >
            ✓ Verified
        </span>
    ) : (
        <span
            data-tooltip={t('ekyc.unverified')}
            style={{ background: 'var(--accent-rose-10)', color: 'var(--accent-rose)', padding: '0.2rem 0.6rem', borderRadius: '12px', fontSize: '0.75rem', display: 'flex', alignItems: 'center', cursor: 'help' }}
        >
            ⚠ Unverified
        </span>
    );
};

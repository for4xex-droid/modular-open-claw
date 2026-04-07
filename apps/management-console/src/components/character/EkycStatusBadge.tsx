import React from 'react';

interface EkycStatusBadgeProps {
    status: boolean | null;
}

export const EkycStatusBadge: React.FC<EkycStatusBadgeProps> = ({ status }) => {
    if (status === null) return null;
    
    return status ? (
        <span style={{ background: 'rgba(16, 185, 129, 0.1)', color: 'var(--accent-emerald)', padding: '0.2rem 0.6rem', borderRadius: '12px', fontSize: '0.75rem', display: 'flex', alignItems: 'center' }}>
            ✓ Verified
        </span>
    ) : (
        <span style={{ background: 'rgba(255, 77, 148, 0.1)', color: 'var(--accent-rose)', padding: '0.2rem 0.6rem', borderRadius: '12px', fontSize: '0.75rem', display: 'flex', alignItems: 'center' }}>
            ⚠ Unverified
        </span>
    );
};

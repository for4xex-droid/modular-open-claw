/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import React from 'react';
import { Lock } from 'lucide-react';
import { useTranslation } from '../../i18n';
import { useSubscriptionStatus, openProUpgradeModal } from '../../hooks/useSubscriptionStatus';

export interface LockedOverlayProps {
  featureNameKey: string;
  children: React.ReactNode;
}

export const LockedOverlay: React.FC<LockedOverlayProps> = ({ featureNameKey, children }) => {
  const { t } = useTranslation();
  const { isPro, isLoading } = useSubscriptionStatus();

  if (isLoading || isPro) {
    return <>{children}</>;
  }

  const featureName = t(featureNameKey);

  return (
    <div style={{ position: 'relative' }}>
      <div style={{ opacity: 0.45, pointerEvents: 'none', filter: 'grayscale(0.3)' }}>{children}</div>
      <button
        type="button"
        onClick={() => openProUpgradeModal(featureNameKey)}
        style={{
          position: 'absolute',
          inset: 0,
          display: 'flex',
          flexDirection: 'column',
          alignItems: 'center',
          justifyContent: 'center',
          gap: '0.5rem',
          background: 'var(--black-40)',
          border: '1px dashed var(--accent-purple-30)',
          borderRadius: 'var(--radius-md)',
          cursor: 'pointer',
          padding: '1rem',
        }}
        aria-label={t('pro.unlockHint', { feature: featureName })}
      >
        <Lock size={24} color="var(--accent-purple)" />
        <span style={{ color: 'var(--text-primary)', fontWeight: 600, fontSize: '0.9rem' }}>
          {t('pro.lockedTitle', { feature: featureName })}
        </span>
        <span style={{ color: 'var(--text-muted)', fontSize: '0.8rem' }}>{t('pro.unlockHint')}</span>
      </button>
    </div>
  );
};

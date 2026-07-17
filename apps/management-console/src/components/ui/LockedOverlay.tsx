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
  /** panel: full-area lock overlay; badge: compact inline Pro pill (small controls) */
  variant?: 'panel' | 'badge';
}

export const LockedOverlay: React.FC<LockedOverlayProps> = ({
  featureNameKey,
  children,
  variant = 'panel',
}) => {
  const { t } = useTranslation();
  const { isPro, isLoading } = useSubscriptionStatus();

  if (isPro) {
    return <>{children}</>;
  }

  // Fail-closed while status is unknown: keep the feature blocked (no Free unlock flash).
  if (isLoading) {
    if (variant === 'badge') {
      return (
        <span className="locked-badge-wrap">
          <span className="locked-badge-wrap__content" aria-hidden="true">
            {children}
          </span>
          <span className="locked-badge" aria-busy="true" aria-label={t('common.loading')}>
            …
          </span>
        </span>
      );
    }
    return (
      <div className="locked-overlay-panel">
        <div className="locked-overlay-panel__content" aria-hidden="true">
          {children}
        </div>
        <div className="locked-overlay-panel__cta" aria-busy="true" aria-label={t('common.loading')}>
          <Lock size={24} color="var(--accent-purple)" />
          <span className="locked-overlay-panel__title">…</span>
        </div>
      </div>
    );
  }

  const featureName = t(featureNameKey);
  const lockedTitle = t('pro.lockedTitle', { feature: featureName });
  const unlockHint = t('pro.unlockHint');

  if (variant === 'badge') {
    return (
      <span className="locked-badge-wrap">
        <span className="locked-badge-wrap__content" aria-hidden="true">
          {children}
        </span>
        <button
          type="button"
          className="locked-badge"
          onClick={() => openProUpgradeModal(featureNameKey)}
          aria-label={lockedTitle}
          data-tooltip={`${lockedTitle} — ${unlockHint}`}
        >
          <Lock size={12} />
          Pro
        </button>
      </span>
    );
  }

  return (
    <div className="locked-overlay-panel">
      <div className="locked-overlay-panel__content">{children}</div>
      <button
        type="button"
        className="locked-overlay-panel__cta"
        onClick={() => openProUpgradeModal(featureNameKey)}
        aria-label={unlockHint}
      >
        <Lock size={24} color="var(--accent-purple)" />
        <span className="locked-overlay-panel__title">{lockedTitle}</span>
        <span className="locked-overlay-panel__hint">{unlockHint}</span>
      </button>
    </div>
  );
};

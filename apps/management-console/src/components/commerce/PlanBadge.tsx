/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import React from 'react';
import { useTranslation } from '../../i18n';
import { useSubscriptionStatus, openProUpgradeModal } from '../../hooks/useSubscriptionStatus';

/** Pattern: EkycStatusBadge — pill + tooltip, token colors only */
export const PlanBadge: React.FC = () => {
  const { t } = useTranslation();
  const { isPro, isLoading } = useSubscriptionStatus();

  if (isLoading) {
    return (
      <span
        style={{
          background: 'var(--white-05)',
          color: 'var(--text-muted)',
          padding: '0.2rem 0.6rem',
          borderRadius: '12px',
          fontSize: '0.75rem',
        }}
      >
        …
      </span>
    );
  }

  if (isPro) {
    return (
      <span
        data-tooltip={t('pro.badgeProTooltip')}
        style={{
          background: 'var(--accent-emerald-10)',
          color: 'var(--accent-emerald)',
          padding: '0.2rem 0.6rem',
          borderRadius: '12px',
          fontSize: '0.75rem',
          display: 'flex',
          alignItems: 'center',
          cursor: 'help',
          fontWeight: 600,
        }}
      >
        Pro
      </span>
    );
  }

  return (
    <button
      type="button"
      onClick={() => openProUpgradeModal()}
      data-tooltip={t('pro.badgeFreeTooltip')}
      style={{
        background: 'var(--white-05)',
        color: 'var(--text-secondary)',
        padding: '0.2rem 0.6rem',
        borderRadius: '12px',
        fontSize: '0.75rem',
        border: '1px solid var(--white-10)',
        cursor: 'pointer',
        fontWeight: 600,
      }}
    >
      {t('pro.badgeFree')}
    </button>
  );
};

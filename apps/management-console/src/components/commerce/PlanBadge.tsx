/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import React from 'react';
import { useTranslation } from '../../i18n';
import { useSubscriptionStatus, openProUpgradeModal } from '../../hooks/useSubscriptionStatus';
import { useCheckoutSession } from '../../hooks/useCheckoutSession';
import { useAgentIdentity } from '../../hooks/useAgentIdentity';
import { STRIPE_PRICE_ID } from '../../config';

/** Pattern: EkycStatusBadge — pill + tooltip, token colors only */
export const PlanBadge: React.FC = () => {
  const { t } = useTranslation();
  const { agentId } = useAgentIdentity();
  const { isPro, isLoading } = useSubscriptionStatus();
  const { handlePortal, isPortalLoading, error: portalError } = useCheckoutSession(
    STRIPE_PRICE_ID,
    agentId ?? undefined
  );

  if (isLoading) {
    return (
      <span
        style={{
          background: 'var(--white-05)',
          color: 'var(--text-muted)',
          padding: '0.2rem 0.6rem',
          borderRadius: 'var(--radius-md)',
          fontSize: '0.75rem',
        }}
      >
        …
      </span>
    );
  }

  if (isPro) {
    return (
      <button
        type="button"
        onClick={() => void handlePortal()}
        disabled={isPortalLoading || !agentId}
        data-tooltip={portalError ?? t('pro.badgeProTooltip')}
        title={portalError ?? t('pro.badgeProTooltip')}
        aria-busy={isPortalLoading}
        style={{
          background: 'var(--accent-emerald-10)',
          color: portalError ? 'var(--accent-rose)' : 'var(--accent-emerald)',
          padding: '0.2rem 0.6rem',
          borderRadius: 'var(--radius-md)',
          fontSize: '0.75rem',
          display: 'flex',
          alignItems: 'center',
          cursor: isPortalLoading ? 'wait' : 'pointer',
          fontWeight: 600,
          border: '1px solid var(--accent-emerald-30)',
        }}
      >
        {isPortalLoading ? '…' : 'Pro'}
      </button>
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
        borderRadius: 'var(--radius-md)',
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

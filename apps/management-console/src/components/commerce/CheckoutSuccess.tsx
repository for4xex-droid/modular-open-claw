/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import React, { useEffect, useRef } from 'react';
import { CheckCircle, RefreshCw } from 'lucide-react';
import { useTranslation } from '../../i18n';
import { useSubscriptionStatus } from '../../hooks/useSubscriptionStatus';

interface CheckoutSuccessProps {
  onGoHome: () => void;
}

export const CheckoutSuccess: React.FC<CheckoutSuccessProps> = ({ onGoHome }) => {
  const { t } = useTranslation();
  const { status, isPro, isLoading, refresh } = useSubscriptionStatus();
  const pollCountRef = useRef(0);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  // Sequential short polling (max 3 attempts, ~2s apart). Avoids overlapping
  // setInterval ticks; each attempt awaits refresh before scheduling the next.
  useEffect(() => {
    if (isPro) {
      pollCountRef.current = 0;
      return;
    }
    if (pollCountRef.current >= 3) {
      return;
    }

    let cancelled = false;
    let timeoutId: number | undefined;

    const scheduleNext = () => {
      timeoutId = window.setTimeout(() => {
        void (async () => {
          if (cancelled || pollCountRef.current >= 3) {
            return;
          }
          pollCountRef.current += 1;
          await refresh();
          if (!cancelled && pollCountRef.current < 3) {
            scheduleNext();
          }
        })();
      }, 2000);
    };

    scheduleNext();
    return () => {
      cancelled = true;
      if (timeoutId !== undefined) {
        window.clearTimeout(timeoutId);
      }
    };
  }, [isPro, refresh]);

  return (
    <div
      style={{
        display: 'flex',
        flexDirection: 'column',
        alignItems: 'center',
        justifyContent: 'center',
        minHeight: '60vh',
        padding: 'var(--space-xl)',
        textAlign: 'center',
        gap: '1rem',
      }}
    >
      <CheckCircle size={56} color="var(--accent-emerald)" />
      <h2 style={{ color: 'var(--text-primary)', margin: 0 }}>{t('checkout.successTitle')}</h2>
      <p style={{ color: 'var(--text-secondary)', maxWidth: '480px', lineHeight: 1.6 }}>
        {t('checkout.successBody')}
      </p>
      <div
        style={{
          padding: '1rem 1.25rem',
          background: 'var(--white-03)',
          borderRadius: 'var(--radius-md)',
          border: '1px solid var(--border-glass)',
          minWidth: '280px',
        }}
      >
        <span style={{ color: 'var(--text-muted)', fontSize: '0.85rem' }}>
          {t('checkout.currentStatus')}:{' '}
        </span>
        <strong style={{ color: isPro ? 'var(--accent-emerald)' : 'var(--text-primary)' }}>
          {isLoading ? '…' : status ?? t('checkout.statusUnknown')}
        </strong>
      </div>
      <div style={{ display: 'flex', gap: '0.75rem', flexWrap: 'wrap', justifyContent: 'center' }}>
        <button type="button" className="primary-button" onClick={() => void refresh()} disabled={isLoading}>
          <RefreshCw size={16} style={{ marginRight: '0.4rem', verticalAlign: 'middle' }} />
          {t('checkout.refreshStatus')}
        </button>
        <button type="button" className="secondary-button" onClick={onGoHome}>
          {t('checkout.goHome')}
        </button>
      </div>
      {!isPro && !isLoading && (
        <p style={{ color: 'var(--text-muted)', fontSize: '0.8rem', maxWidth: '520px' }}>
          {t('checkout.notReflectedYet')}
        </p>
      )}
    </div>
  );
};

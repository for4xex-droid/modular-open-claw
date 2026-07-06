/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import React from 'react';
import { Loader2 } from 'lucide-react';
import { useTranslation } from '../../i18n';
import { AiomeSkeleton } from '../common/AiomeSkeleton';

export interface LoadingStateProps {
  variant?: 'skeleton' | 'spinner' | 'inline';
  messageKey?: string;
}

export const LoadingState: React.FC<LoadingStateProps> = ({
  variant = 'spinner',
  messageKey = 'loading',
}) => {
  const { t } = useTranslation();

  if (variant === 'skeleton') {
    return (
      <div style={{ padding: 'var(--space-lg)', display: 'grid', gap: 'var(--space-md)' }}>
        <AiomeSkeleton height="40px" width="30%" />
        <AiomeSkeleton height="150px" />
        <AiomeSkeleton height="300px" />
      </div>
    );
  }

  if (variant === 'inline') {
    return (
      <span style={{ color: 'var(--text-muted)', fontSize: '0.85rem' }}>{t(messageKey)}</span>
    );
  }

  return (
    <div
      style={{
        display: 'flex',
        flexDirection: 'column',
        alignItems: 'center',
        justifyContent: 'center',
        gap: '0.75rem',
        padding: 'var(--space-xl)',
        color: 'var(--text-muted)',
      }}
    >
      <Loader2 size={28} className="ani-spin" />
      <span>{t(messageKey)}</span>
    </div>
  );
};

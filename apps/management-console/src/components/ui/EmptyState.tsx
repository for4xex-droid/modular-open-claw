/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import React from 'react';
import type { LucideIcon } from 'lucide-react';
import { useTranslation } from '../../i18n';

export interface EmptyStateProps {
  icon: LucideIcon;
  titleKey: string;
  detailKey?: string;
  cta?: { labelKey: string; onClick: () => void };
}

/** Extracted from ArtifactVault .empty-state pattern */
export const EmptyState: React.FC<EmptyStateProps> = ({ icon: Icon, titleKey, detailKey, cta }) => {
  const { t } = useTranslation();

  return (
    <div
      className="empty-state"
      style={{
        padding: 'var(--space-xl)',
        textAlign: 'center',
        background: 'var(--white-03)',
        borderRadius: 'var(--radius-lg)',
        border: '1px dashed var(--white-10)',
      }}
    >
      <Icon size={48} color="var(--text-muted)" style={{ margin: '0 auto 1.5rem', opacity: 0.5 }} />
      <p style={{ color: 'var(--text-secondary)', fontWeight: 500 }}>{t(titleKey)}</p>
      {detailKey && (
        <p style={{ color: 'var(--text-muted)', fontSize: 'var(--font-sm)', marginTop: '0.5rem' }}>
          {t(detailKey)}
        </p>
      )}
      {cta && (
        <button
          type="button"
          onClick={cta.onClick}
          className="primary-button"
          style={{ marginTop: '1rem' }}
        >
          {t(cta.labelKey)}
        </button>
      )}
    </div>
  );
};

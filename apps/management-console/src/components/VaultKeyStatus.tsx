/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

import React from 'react';
import { useTranslation } from '../i18n';
import { CheckCircle, XCircle } from 'lucide-react';

interface VaultKeyStatusProps {
  isSet: boolean;
}

export function VaultKeyStatus({ isSet }: VaultKeyStatusProps) {
  const { t } = useTranslation();

  const scrollToVault = () => {
    const el = document.querySelector('.vault-secrets-manager');
    if (el) {
      el.scrollIntoView({ behavior: 'smooth' });
    }
  };

  return (
    <div 
      className="vault-key-status" 
      onClick={scrollToVault}
      style={{ 
        display: 'inline-flex', 
        alignItems: 'center', 
        gap: 'var(--space-xs)',
        padding: '2px 8px',
        borderRadius: 'var(--radius-sm)',
        background: 'var(--bg-secondary)',
        border: '1px solid var(--border-color)',
        cursor: 'pointer',
        fontSize: 'var(--font-sm)',
        color: 'var(--text-secondary)'
      }}
    >
      <span style={{ fontSize: 'var(--font-xs)', fontWeight: 'bold', display: 'inline-flex', alignItems: 'center', gap: '2px' }}>
        {t('vault.indicator.managed')}
      </span>
      {isSet ? (
        <span style={{ color: 'var(--color-success)', display: 'inline-flex', alignItems: 'center', gap: '2px' }}>
          <CheckCircle size={12} /> {t('vault.status.set')}
        </span>
      ) : (
        <span style={{ color: 'var(--color-warning)', display: 'inline-flex', alignItems: 'center', gap: '2px' }}>
          <XCircle size={12} /> {t('vault.status.notSet')}
        </span>
      )}
    </div>
  );
}

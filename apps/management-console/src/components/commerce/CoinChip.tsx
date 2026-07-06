import React from 'react';
import { useCoinBalance } from '../../hooks/useCoinBalance';

export const CoinChip: React.FC = () => {
  const { balance, isLoading, error } = useCoinBalance();

  const handleClick = () => {
    window.dispatchEvent(
      new CustomEvent('a2ui-navigate', { detail: { tab: 'nurture' } }),
    );
  };

  return (
    <button
      type="button"
      onClick={handleClick}
      aria-label="Open nurture economy tab"
      style={{
        display: 'inline-flex',
        alignItems: 'center',
        gap: '0.35rem',
        padding: '0.25rem 0.6rem',
        borderRadius: '999px',
        border: '1px solid var(--border-subtle, rgba(255,255,255,0.12))',
        background: 'var(--surface-elevated, rgba(0,0,0,0.25))',
        color: 'var(--text-primary, inherit)',
        cursor: 'pointer',
        fontSize: '0.85rem',
        fontWeight: 600,
      }}
    >
      <span aria-hidden>🪙</span>
      <span>{isLoading ? '…' : error ? '—' : balance.toLocaleString()}</span>
    </button>
  );
};

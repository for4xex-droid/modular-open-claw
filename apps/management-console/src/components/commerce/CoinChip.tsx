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
        border: '1px solid var(--white-10)',
        background: 'var(--black-20)',
        color: 'var(--text-primary)',
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

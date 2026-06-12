import React from 'react';
import { useTranslation } from '../../i18n';
import { motion, useSpring, useTransform } from 'framer-motion';
import { Zap } from 'lucide-react';

export interface TokenSavingsIndicatorProps {
  savedChars: number;
  variant?: 'compact' | 'full';
}

export const TokenSavingsIndicator: React.FC<TokenSavingsIndicatorProps> = ({ savedChars, variant = 'compact' }) => {
  const { t } = useTranslation();
  // Use framer-motion useSpring for smooth counting
  const springValue = useSpring(savedChars, { stiffness: 60, damping: 15 });
  const displayChars = useTransform(springValue, (latest) => Math.round(latest));
  const displayTokens = useTransform(springValue, (latest) => Math.round(latest / 4));
  // Rough estimate of saving cost, e.g., $0.01 per 1k tokens
  const costEstimate = useTransform(springValue, (latest) => ((latest / 4) / 1000 * 0.01).toFixed(3));

  if (savedChars === 0) {
    if (variant === 'compact') {
      return (
        <div className="stat-badge" style={{ background: 'color-mix(in srgb, var(--accent-emerald) 10%, transparent)', color: 'var(--accent-emerald)' }}>
          <Zap size={12} />
          <span>⚡ {t('token.waiting')}</span>
        </div>
      );
    }
    return (
      <div style={{
        background: 'var(--black-30)',
        border: '1px solid var(--white-05)',
        backdropFilter: 'blur(10px)',
        padding: '1rem',
        borderRadius: '12px',
        display: 'flex',
        alignItems: 'center',
        gap: '0.75rem',
        color: 'var(--text-muted)'
      }}>
        <div className="status-dot offline" />
        <span>{t('token.waitingFull')}</span>
      </div>
    );
  }

  if (variant === 'compact') {
    return (
      <motion.div
        className="stat-badge"
        style={{
          background: 'color-mix(in srgb, var(--accent-emerald) 10%, transparent)',
          color: 'var(--accent-emerald)',
          border: '1px solid color-mix(in srgb, var(--accent-emerald) 30%, transparent)',
          display: 'flex',
          gap: '6px'
        }}
        initial={{ scale: 0.9, opacity: 0 }}
        animate={{ scale: 1, opacity: 1 }}
        data-tooltip={t('token.savingsTooltip', { tokens: Math.round(savedChars / 4), cost: ((savedChars / 4) / 1000 * 0.01).toFixed(4) })}
      >
        <Zap size={12} />
        <motion.span data-testid="token-saved-chars-compact">{displayChars}</motion.span>
        <span data-testid="token-saved-chars-exact" style={{ display: 'none' }}>{savedChars}</span>
        <span>chars saved</span>
      </motion.div>
    );
  }

  return (
    <motion.div
      initial={{ opacity: 0, y: 10 }}
      animate={{ opacity: 1, y: 0 }}
      style={{
        background: 'color-mix(in srgb, var(--accent-emerald) 5%, transparent)',
        backgroundBlendMode: 'overlay',
        border: '1px solid color-mix(in srgb, var(--accent-emerald) 20%, transparent)',
        backdropFilter: 'blur(12px)',
        padding: '1.25rem',
        borderRadius: '16px',
        display: 'flex',
        flexDirection: 'column',
        gap: '0.5rem',
        position: 'relative',
        overflow: 'hidden'
      }}
    >
      <div style={{
        position: 'absolute',
        top: 0, left: 0, right: 0, height: '1px',
        background: 'linear-gradient(90deg, transparent, var(--accent-emerald), transparent)'
      }} />

      <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem', color: 'var(--accent-emerald)' }}>
        <Zap size={18} className="ani-pulse" />
        <h4 style={{ margin: 0, fontSize: '0.85rem', letterSpacing: '1px', textTransform: 'uppercase' }}>Token Optimized</h4>
      </div>

      <div style={{ display: 'flex', alignItems: 'baseline', gap: '0.5rem' }}>
        <motion.span data-testid="token-saved-chars-full" style={{ fontSize: '2.5rem', fontWeight: 800, color: 'white', lineHeight: 1 }}>
          {displayChars}
        </motion.span>
        <span style={{ color: 'var(--text-secondary)', fontSize: '0.9rem' }}>chars saved</span>
      </div>

      <div style={{ display: 'flex', alignItems: 'center', gap: '1rem', marginTop: '0.5rem' }}>
        <div style={{ display: 'inline-flex', padding: '0.25rem 0.6rem', background: 'var(--black-40)', borderRadius: '6px', fontSize: '0.75rem', color: 'var(--text-muted)' }}>
          <strong style={{ color: 'var(--accent-emerald)', marginRight: '4px' }}>≈</strong>
          <motion.span>{displayTokens}</motion.span>
          <span style={{ marginLeft: '4px' }}>tokens</span>
        </div>
        <div style={{ display: 'inline-flex', padding: '0.25rem 0.6rem', background: 'var(--black-40)', borderRadius: '6px', fontSize: '0.75rem', color: 'var(--text-muted)' }}>
          <strong style={{ color: 'var(--accent-emerald)', marginRight: '4px' }}>≈</strong>
          $<motion.span>{costEstimate}</motion.span>
        </div>
      </div>
    </motion.div>
  );
};

export default React.memo(TokenSavingsIndicator);

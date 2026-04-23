import React, { useState, useEffect } from 'react';
import { motion, useSpring, useTransform } from 'framer-motion';
import { ShieldCheck } from 'lucide-react';
import { authenticatedFetch } from '../../lib/auth';
import { API_BASE } from '../../config';
import type { components } from '../../types/generated';

type OxiLeanPowerResponse = components['schemas']['OxiLeanPowerResponse'];

export interface ProofPowerIndicatorProps {
  variant?: 'compact' | 'full';
}

export const ProofPowerIndicator: React.FC<ProofPowerIndicatorProps> = ({ variant = 'compact' }) => {
  const [proofPower, setProofPower] = useState<number>(0);
  
  useEffect(() => {
    let mounted = true;
    const fetchPower = async () => {
      try {
        const res = await authenticatedFetch(`${API_BASE}/api/v1/security/oxilean/power`);
        if (res.ok) {
          const data: OxiLeanPowerResponse = await res.json();
          if (mounted) setProofPower(data.power);
        }
      } catch (e) {
        console.error('Failed to fetch OxiLean power', e);
      }
    };
    fetchPower();
    return () => { mounted = false; };
  }, []);

  const springValue = useSpring(proofPower, { stiffness: 60, damping: 15 });
  const displayPower = useTransform(springValue, (latest) => Math.round(latest));

  if (proofPower === 0) {
    if (variant === 'compact') {
      return (
        <div className="stat-badge" style={{ background: 'color-mix(in srgb, var(--accent-blue) 10%, transparent)', color: 'var(--accent-blue)' }}>
          <ShieldCheck size={12} />
          <span>🛡️ 測定中...</span>
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
        <span>証明力測定中...</span>
      </div>
    );
  }

  if (variant === 'compact') {
    return (
      <motion.div
        className="stat-badge"
        style={{
          background: 'color-mix(in srgb, var(--accent-blue) 10%, transparent)',
          color: 'var(--accent-blue)',
          border: '1px solid color-mix(in srgb, var(--accent-blue) 30%, transparent)',
          display: 'flex',
          gap: '6px'
        }}
        initial={{ scale: 0.9, opacity: 0 }}
        animate={{ scale: 1, opacity: 1 }}
        title={`OxiLean 証明力: ${proofPower} OXP`}
      >
        <ShieldCheck size={12} />
        <motion.span data-testid="proof-power-compact">{displayPower}</motion.span>
        <span>OXP</span>
      </motion.div>
    );
  }

  return (
    <motion.div
      initial={{ opacity: 0, y: 10 }}
      animate={{ opacity: 1, y: 0 }}
      style={{
        background: 'color-mix(in srgb, var(--accent-blue) 5%, transparent)',
        backgroundBlendMode: 'overlay',
        border: '1px solid color-mix(in srgb, var(--accent-blue) 20%, transparent)',
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
        background: 'linear-gradient(90deg, transparent, var(--accent-blue), transparent)'
      }} />

      <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem', color: 'var(--accent-blue)' }}>
        <ShieldCheck size={18} className="ani-pulse" />
        <h4 style={{ margin: 0, fontSize: '0.85rem', letterSpacing: '1px', textTransform: 'uppercase' }}>OxiLean Proof Power</h4>
      </div>

      <div style={{ display: 'flex', alignItems: 'baseline', gap: '0.5rem' }}>
        <motion.span data-testid="proof-power-full" style={{ fontSize: '2.5rem', fontWeight: 800, color: 'white', lineHeight: 1 }}>
          {displayPower}
        </motion.span>
        <span style={{ color: 'var(--text-secondary)', fontSize: '0.9rem' }}>OXP</span>
      </div>

      <div style={{ display: 'flex', alignItems: 'center', gap: '1rem', marginTop: '0.5rem' }}>
        <div style={{ display: 'inline-flex', padding: '0.25rem 0.6rem', background: 'var(--black-40)', borderRadius: '6px', fontSize: '0.75rem', color: 'var(--text-muted)' }}>
          <span style={{ marginLeft: '4px' }}>Secured by OxiLean Infrastructure</span>
        </div>
      </div>
    </motion.div>
  );
};

export default React.memo(ProofPowerIndicator);

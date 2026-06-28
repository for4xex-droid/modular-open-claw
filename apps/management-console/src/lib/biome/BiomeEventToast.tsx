/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import { useEffect } from 'react';

export interface BiomeEventToastProps {
  events: Array<{ type: string; message: string; icon: string }>;
  onDismiss: (index: number) => void;
}

export function BiomeEventToast({ events, onDismiss }: BiomeEventToastProps) {
  useEffect(() => {
    if (events.length === 0) return;
    // 最新のトーストを3秒後に自動フェードアウト（Dismiss）するタイマー
    const timer = setTimeout(() => {
      onDismiss(0);
    }, 3000);
    return () => clearTimeout(timer);
  }, [events, onDismiss]);

  if (events.length === 0) return null;

  return (
    <div style={{
      position: 'absolute',
      top: '24px',
      left: '50%',
      transform: 'translateX(-50%)',
      display: 'flex',
      flexDirection: 'column',
      gap: '8px',
      zIndex: 100,
      pointerEvents: 'none',
      width: '100%',
      maxWidth: '360px',
    }}>
      {events.map((e, idx) => (
        <div
          key={idx}
          style={{
            display: 'flex',
            alignItems: 'center',
            gap: '12px',
            background: 'rgba(15, 23, 42, 0.85)',
            border: '1px solid var(--accent-cyan, #00f0ff)',
            boxShadow: '0 8px 32px rgba(0, 240, 255, 0.25), inset 0 0 12px rgba(0, 240, 255, 0.1)',
            backdropFilter: 'blur(12px)',
            borderRadius: '12px',
            padding: '12px 16px',
            color: 'var(--white-100, #ffffff)',
            fontFamily: 'var(--font-main, sans-serif)',
            fontSize: '0.9rem',
            animation: 'biomeToastSlideIn 0.3s cubic-bezier(0.16, 1, 0.3, 1) forwards',
            pointerEvents: 'auto',
            cursor: 'pointer',
            transition: 'transform 0.2s, opacity 0.2s',
          }}
          onClick={() => onDismiss(idx)}
          onMouseEnter={(el) => {
            el.currentTarget.style.transform = 'scale(1.02)';
          }}
          onMouseLeave={(el) => {
            el.currentTarget.style.transform = 'scale(1)';
          }}
        >
          <span style={{ fontSize: '1.25rem' }}>{e.icon}</span>
          <span style={{ flex: '1', fontWeight: 500 }}>{e.message}</span>
          <span style={{ fontSize: '0.75rem', color: 'var(--white-40, #666)' }}>✕</span>
        </div>
      ))}
      <style>{`
        @keyframes biomeToastSlideIn {
          from {
            opacity: 0;
            transform: translateY(-20px) scale(0.95);
          }
          to {
            opacity: 1;
            transform: translateY(0) scale(1);
          }
        }
      `}</style>
    </div>
  );
}

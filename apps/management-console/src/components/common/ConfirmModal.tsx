/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import { ReactNode } from 'react';
import { motion } from 'framer-motion';
import { AlertTriangle, Info, CheckCircle, AlertCircle } from 'lucide-react';
import { cssVar } from '../../utils/cssVar';
import { Modal } from '../ui/Modal';

export type ConfirmModalType = 'warning' | 'danger' | 'info' | 'success';

interface ConfirmModalProps {
  isOpen: boolean;
  type?: ConfirmModalType;
  title: string;
  message: ReactNode;
  details?: ReactNode;
  confirmText?: string;
  cancelText?: string;
  onConfirm: () => void;
  onCancel: () => void;
}

const getIcon = (type: ConfirmModalType) => {
  switch (type) {
    case 'warning':
      return <AlertTriangle size={24} color="var(--accent-amber)" />;
    case 'danger':
      return <AlertCircle size={24} color="var(--accent-rose)" />;
    case 'success':
      return <CheckCircle size={24} color="var(--accent-emerald)" />;
    case 'info':
    default:
      return <Info size={24} color="var(--accent-cyan)" />;
  }
};

const getConfirmButtonColor = (type: ConfirmModalType) => {
  switch (type) {
    case 'warning':
      return { background: 'var(--accent-amber)', color: cssVar('--text-primary') };
    case 'danger':
      return { background: 'var(--accent-rose)', color: cssVar('--text-on-accent') };
    case 'success':
      return { background: 'var(--accent-emerald)', color: cssVar('--text-on-accent') };
    case 'info':
    default:
      return { background: 'var(--accent-cyan)', color: cssVar('--text-primary') };
  }
};

export default function ConfirmModal({
  isOpen,
  type = 'warning',
  title,
  message,
  details,
  confirmText = 'Confirm',
  cancelText = 'Cancel',
  onConfirm,
  onCancel
}: ConfirmModalProps) {
  return (
    <Modal
      isOpen={isOpen}
      onClose={onCancel}
      labelledBy="confirm-modal-title"
      overlayStyle={{
        background: 'var(--bg-glass-heavy)',
        backdropFilter: 'blur(4px)',
        padding: '1rem',
      }}
    >
      <motion.div
        initial={{ scale: 0.9, opacity: 0 }}
        animate={{ scale: 1, opacity: 1 }}
        exit={{ scale: 0.9, opacity: 0 }}
        style={{
          background: 'var(--bg-primary)',
          border: '1px solid var(--border-glass)',
          borderRadius: 'var(--radius-lg)',
          padding: '2rem',
          maxWidth: '500px',
          width: '100%',
          boxShadow: 'var(--shadow-deep)',
        }}
      >
        <div style={{ display: 'flex', alignItems: 'center', gap: '0.75rem', marginBottom: '1rem' }}>
          {getIcon(type)}
          <h3 id="confirm-modal-title" style={{ margin: 0, color: 'var(--text-primary)' }}>{title}</h3>
        </div>
        <p style={{ color: 'var(--text-secondary)', marginBottom: '1rem', lineHeight: 1.5 }}>
          {message}
        </p>
        {details && (
          <div style={{ background: 'var(--black-20)', padding: '1rem', borderRadius: 'var(--radius-md)', marginBottom: '1.5rem', fontSize: '0.85rem', color: 'var(--text-muted)' }}>
            {details}
          </div>
        )}
        <div style={{ display: 'flex', justifyContent: 'flex-end', gap: '1rem' }}>
          <button
            type="button"
            onClick={onCancel}
            className="secondary-button"
            style={{ padding: '0.5rem 1rem' }}
          >
            {cancelText}
          </button>
          <button
            type="button"
            onClick={onConfirm}
            className="primary-button"
            style={{ ...getConfirmButtonColor(type), padding: '0.5rem 1rem', border: 'none' }}
          >
            {confirmText}
          </button>
        </div>
      </motion.div>
    </Modal>
  );
}

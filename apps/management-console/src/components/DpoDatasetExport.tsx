import React, { useState, useCallback } from 'react';
import { Download, Loader2 } from 'lucide-react';
import { authenticatedFetch } from '../lib/auth';
import { useTranslation } from '../i18n';

export default function DpoDatasetExport() {
  const { t } = useTranslation();
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleDownload = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const res = await authenticatedFetch('/api/v1/cortex/dpo/dataset');
      if (!res.ok) {
        throw new Error(t('dpoExport.downloadFailed') || 'Failed to export dataset');
      }

      const blob = await res.blob();
      const url = window.URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = 'dpo_dataset.jsonl';
      document.body.appendChild(a);
      a.click();
      a.remove();
      window.URL.revokeObjectURL(url);
    } catch (err: unknown) {
      const message =
        err instanceof Error ? err.message : (t('dpoExport.downloadFailed') || 'Failed to export dataset');
      setError(message);
    } finally {
      setLoading(false);
    }
  }, [t]);

  return (
    <div style={{
      padding: 'var(--space-md)',
      border: '1px solid var(--border-glass)',
      borderRadius: 'var(--radius-lg)',
      background: 'var(--bg-glass-heavy)',
      display: 'flex',
      flexDirection: 'column',
      gap: 'var(--space-md)',
    }}>
      <div>
        <h3 style={{ fontSize: '1.1rem', fontWeight: 600, color: 'var(--text-primary)', marginBottom: 'var(--space-xs)' }}>
          {t('dpoExport.title') || 'DPO Dataset Export'}
        </h3>
        <p style={{ fontSize: '0.85rem', color: 'var(--text-muted)' }}>
          {t('dpoExport.description') || 'Download the Direct Preference Optimization (DPO) dataset generated from Arena matches and manual feedback.'}
        </p>
      </div>

      {error && (
        <div style={{
          padding: 'var(--space-sm)',
          background: 'var(--accent-rose-10)',
          border: '1px solid var(--accent-rose)',
          color: 'var(--accent-rose)',
          borderRadius: 'var(--radius-sm)',
          fontSize: '0.85rem',
        }}>
          {error}
        </div>
      )}

      <div>
        <button
          onClick={handleDownload}
          disabled={loading}
          aria-busy={loading}
          style={{
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            gap: 'var(--space-sm)',
            width: '100%',
            padding: 'var(--space-sm) var(--space-md)',
            background: 'var(--accent-purple)',
            color: 'var(--text-primary)',
            border: 'none',
            borderRadius: 'var(--radius-md)',
            cursor: loading ? 'not-allowed' : 'pointer',
            opacity: loading ? 0.6 : 1,
            fontWeight: 600,
            transition: 'all 0.2s ease',
            boxShadow: 'var(--glow-purple)',
          }}
        >
          {loading ? <Loader2 className="animate-spin" size={16} /> : <Download size={16} />}
          {loading
            ? (t('dpoExport.exporting') || 'Exporting...')
            : (t('dpoExport.downloadButton') || 'Download Dataset (JSONL)')}
        </button>
      </div>
    </div>
  );
}

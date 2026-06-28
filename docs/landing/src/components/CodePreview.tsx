/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import { useState, useRef, useCallback, useEffect } from 'react';
import { useTranslation } from 'react-i18next';
import { motion } from 'framer-motion';
import { Copy, Check, Terminal } from 'lucide-react';

export function CodePreview() {
  const { t } = useTranslation();
  const [copied, setCopied] = useState(false);
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  
  const rawCommand = "git clone https://github.com/motivationstudio-llc/aiome\ncd aiome\ndocker compose -f docker-compose.quickstart.yml up -d";

  // Cleanup timer on unmount to prevent setState on unmounted component
  useEffect(() => {
    return () => {
      if (timerRef.current) clearTimeout(timerRef.current);
    };
  }, []);

  const handleCopy = useCallback(async () => {
    try {
      await navigator.clipboard.writeText(rawCommand);
      setCopied(true);
      if (timerRef.current) clearTimeout(timerRef.current);
      timerRef.current = setTimeout(() => setCopied(false), 2000);
    } catch {
      // Fallback: clipboard API unavailable in insecure context
      setCopied(false);
    }
  }, [rawCommand]);

  return (
    <section id="quickstart" className="py-24 relative overflow-hidden bg-brand-surface border-y border-white/5" aria-labelledby="quickstart-title">
      <div className="container mx-auto px-4 max-w-5xl">
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-12 items-center">
          <motion.div 
            initial={{ opacity: 0, x: -20 }}
            whileInView={{ opacity: 1, x: 0 }}
            viewport={{ once: true }}
            transition={{ duration: 0.6 }}
          >
            <h2 id="quickstart-title" className="text-3xl md:text-5xl font-extrabold text-white mb-6 font-display">
              {t('code_preview.title')}
            </h2>
            <p className="text-xl text-gray-400 leading-relaxed mb-8">
              {t('code_preview.desc')}
            </p>
          </motion.div>

          <motion.div 
            initial={{ opacity: 0, scale: 0.95 }}
            whileInView={{ opacity: 1, scale: 1 }}
            viewport={{ once: true }}
            transition={{ duration: 0.6, delay: 0.2 }}
            className="rounded-xl overflow-hidden bg-brand-bg border border-white/10 shadow-2xl"
          >
            <div className="flex items-center justify-between px-4 py-3 bg-white/5 border-b border-white/10">
              <div className="flex items-center gap-2">
                <Terminal size={16} className="text-gray-400" aria-hidden="true" />
                <span className="text-xs font-mono text-gray-400">Terminal</span>
              </div>
              <button 
                type="button"
                onClick={handleCopy}
                className="flex items-center gap-1.5 text-xs text-gray-400 hover:text-white transition-colors"
                aria-label={t('code_preview.copy')}
              >
                {copied ? <Check size={14} className="text-emerald-500" aria-hidden="true" /> : <Copy size={14} aria-hidden="true" />}
                {copied ? t('code_preview.copied') : t('code_preview.copy')}
              </button>
            </div>
            <div className="p-6 overflow-x-auto">
              <pre className="text-sm font-mono text-gray-300 leading-relaxed">
                <code>
                  <span className="text-brand-rose">git</span> clone https://github.com/motivationstudio-llc/aiome
                  <br /><span className="text-brand-rose">cd</span> aiome
                  <br /><span className="text-brand-rose">docker compose</span> -f docker-compose.quickstart.yml up -d
                </code>
              </pre>
            </div>
          </motion.div>
        </div>
      </div>
    </section>
  );
}

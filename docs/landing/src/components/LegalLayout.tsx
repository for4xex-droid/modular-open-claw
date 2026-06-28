/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import type { ReactNode } from 'react';
import { useTranslation } from 'react-i18next';
import { ArrowLeft } from 'lucide-react';

interface LegalLayoutProps {
  title: string;
  lastUpdated?: string;
  children: ReactNode;
}

export function LegalLayout({ title, lastUpdated, children }: LegalLayoutProps) {
  const { t } = useTranslation();

  const handleGoBack = (e: React.MouseEvent) => {
    e.preventDefault();
    window.location.href = '/';
  };

  return (
    <div className="min-h-screen bg-brand-bg text-white font-body selection:bg-brand-cyan/30 py-12 px-4 relative overflow-hidden">
      {/* Background gradients for premium wow factor */}
      <div aria-hidden="true" className="absolute inset-0 z-0 flex justify-around items-center pointer-events-none">
        <div className="w-[600px] h-[600px] bg-brand-cyan/5 blur-[180px] rounded-full" />
        <div className="w-[600px] h-[600px] bg-brand-purple/5 blur-[180px] rounded-full" />
      </div>

      <div className="container mx-auto max-w-4xl relative z-10">
        {/* Back Button */}
        <div className="mb-8">
          <a
            href="/"
            onClick={handleGoBack}
            className="inline-flex items-center gap-2 text-gray-400 hover:text-brand-cyan transition-colors group text-sm font-semibold"
          >
            <ArrowLeft size={16} className="transform group-hover:-translate-x-1 transition-transform" />
            {t('legal.back_to_home', 'Back to Home')}
          </a>
        </div>

        {/* Paper Container with Glassmorphism */}
        <article className="border border-white/10 bg-brand-surface/60 backdrop-blur-md rounded-3xl p-8 md:p-12 shadow-2xl">
          <header className="border-b border-white/5 pb-6 mb-8">
            <h1 className="text-3xl md:text-5xl font-extrabold text-white mb-4 font-display tracking-tight leading-tight">
              {title}
            </h1>
            {lastUpdated && (
              <p className="text-sm text-gray-500 font-semibold tracking-wider uppercase">
                {t('legal.last_updated', 'Last Updated')}: {lastUpdated}
              </p>
            )}
          </header>

          {/* Styled Document Body */}
          <div className="prose prose-invert max-w-none text-gray-300 leading-relaxed space-y-6 text-base md:text-lg">
            {children}
          </div>
        </article>
      </div>
    </div>
  );
}

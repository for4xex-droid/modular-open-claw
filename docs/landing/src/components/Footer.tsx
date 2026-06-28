/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import { useTranslation } from 'react-i18next';
import { Github } from 'lucide-react'; // NOTE: Github icon in lucide-react is deprecated in newer versions. Consider migrating to SVG in the future.

export function Footer() {
  const { t } = useTranslation();

  return (
    <footer className="py-16 bg-brand-bg border-t border-white/5">
      <div className="container mx-auto px-4 flex flex-col md:flex-row items-center justify-between gap-6 text-sm text-gray-500">
        <div>
          {t('footer.copyright')}
        </div>
        
        <div className="flex flex-col sm:flex-row items-center gap-6">
          {/* Social Links */}
          <div className="flex gap-4 items-center">
            <a 
              href="https://github.com/motivationstudio-llc/aiome" 
              target="_blank" 
              rel="noopener noreferrer" 
              className="hover:text-white transition-colors flex items-center gap-1.5"
              aria-label={t('footer.github')}
            >
              <Github size={18} aria-hidden="true" />
              <span>{t('footer.github')}</span>
            </a>
            <a 
              href="https://x.com/aiome_dev" 
              target="_blank" 
              rel="noopener noreferrer" 
              className="hover:text-white transition-colors flex items-center gap-1.5"
              aria-label={t('footer.twitter')}
            >
              <svg className="w-[18px] h-[18px] fill-current" viewBox="0 0 24 24" aria-hidden="true">
                <path d="M18.244 2.25h3.308l-7.227 8.26 8.502 11.24H16.17l-5.214-6.817L4.99 21.75H1.68l7.73-8.835L1.254 2.25H8.08l4.713 6.231zm-1.161 17.52h1.833L7.084 4.126H5.117z" />
              </svg>
              <span>{t('footer.twitter')}</span>
            </a>
          </div>

          {/* Legal Links */}
          <div className="flex gap-6">
            <a href="/privacy" className="hover:text-white transition-colors">{t('footer.privacy')}</a>
            <a href="/terms" className="hover:text-white transition-colors">{t('footer.terms')}</a>
            <a href="/tokushoho" className="hover:text-white transition-colors">{t('footer.tokushoho')}</a>
          </div>
        </div>
      </div>
    </footer>
  );
}

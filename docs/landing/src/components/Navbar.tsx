/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import { useState, useEffect, useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import { Globe, Menu, X } from 'lucide-react';

export function Navbar() {
  const { t, i18n } = useTranslation();
  const [mobileOpen, setMobileOpen] = useState(false);

  const toggleLang = useCallback(() => {
    const newLang = i18n.language === 'en' ? 'ja' : 'en';
    i18n.changeLanguage(newLang);
  }, [i18n]);

  // Close mobile menu on Escape key
  useEffect(() => {
    const handleEscape = (e: KeyboardEvent) => {
      if (e.key === 'Escape' && mobileOpen) setMobileOpen(false);
    };
    document.addEventListener('keydown', handleEscape);
    return () => document.removeEventListener('keydown', handleEscape);
  }, [mobileOpen]);

  const navLinks = [
    { href: '#features', label: t('nav.features'), external: false },
    { href: '#economy', label: t('nav.economy'), external: false },
    { href: '#quickstart', label: t('nav.quickstart'), external: false },
    { href: 'https://github.com/motivationstudio-llc/aiome#-ドキュメント-documentation', label: t('nav.docs'), external: true },
    { href: 'https://github.com/motivationstudio-llc/aiome', label: t('nav.github'), external: true },
  ];

  return (
    <nav className="fixed top-0 left-0 right-0 z-50 bg-brand-bg/80 backdrop-blur-md border-b border-white/5" aria-label="Main navigation">
      {/* Skip link for a11y — i18n aware */}
      <a href="#main-content" className="sr-only focus:not-sr-only focus:absolute focus:top-2 focus:left-2 focus:z-[60] focus:bg-brand-cyan focus:text-black focus:px-4 focus:py-2 focus:rounded">
        {t('nav.skip')}
      </a>
      <div className="container mx-auto px-4 h-16 flex items-center justify-between">
        <div className="flex items-center gap-8">
          <a href="/" className="flex items-center">
            <img src="/aiome-horizontal-white.png" alt="Aiome" className="h-8 w-auto" data-testid="navbar-logo" />
          </a>
          
          {/* Desktop nav */}
          <div className="hidden md:flex items-center gap-6 text-sm font-medium text-gray-400">
            {navLinks.map((link) => (
              <a
                key={link.href}
                href={link.href}
                className="hover:text-white transition-colors"
                {...(link.external ? { target: '_blank', rel: 'noopener noreferrer' } : {})}
              >
                {link.label}
              </a>
            ))}
          </div>
        </div>
        
        <div className="flex items-center gap-3">
          <button 
            type="button"
            onClick={toggleLang}
            className="flex items-center gap-2 text-sm text-gray-300 hover:text-white transition-colors px-3 py-1.5 rounded-md hover:bg-white/5"
            aria-label={t('nav.switch_lang')}
          >
            <Globe size={16} aria-hidden="true" />
            <span className="font-mono text-xs">{i18n.language === 'en' ? 'EN | 日' : '日 | EN'}</span>
          </button>

          {/* Mobile hamburger */}
          <button
            type="button"
            className="md:hidden text-gray-300 hover:text-white transition-colors p-1.5"
            onClick={() => setMobileOpen(!mobileOpen)}
            aria-label={mobileOpen ? t('nav.close_menu') : t('nav.open_menu')}
            aria-expanded={mobileOpen}
          >
            {mobileOpen ? <X size={22} aria-hidden="true" /> : <Menu size={22} aria-hidden="true" />}
          </button>
        </div>
      </div>

      {/* Mobile menu */}
      {mobileOpen && (
        <div className="md:hidden bg-brand-bg/95 backdrop-blur-lg border-t border-white/5 px-4 py-4 flex flex-col gap-3">
          {navLinks.map((link) => (
            <a
              key={link.href}
              href={link.href}
              className="text-gray-300 hover:text-white transition-colors py-2 text-base font-medium"
              onClick={() => setMobileOpen(false)}
              {...(link.external ? { target: '_blank', rel: 'noopener noreferrer' } : {})}
            >
              {link.label}
            </a>
          ))}
        </div>
      )}
    </nav>
  );
}

/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import React, { createContext, useContext, useState, useCallback, useMemo } from 'react';
import en from './en.json';
import ja from './ja.json';

export type Language = 'en' | 'ja';

const translations: Record<Language, Record<string, any>> = { en, ja };

interface LanguageContextType {
  lang: Language;
  setLang: (lang: Language) => void;
  t: (key: string, params?: Record<string, string | number>) => string;
}

const LanguageContext = createContext<LanguageContextType>({
  lang: 'en',
  setLang: () => {},
  t: (key: string) => key,
});

function getNestedValue(obj: any, path: string): string | undefined {
  const parts = path.split('.');
  let current = obj;
  for (const part of parts) {
    if (current == null || typeof current !== 'object') return undefined;
    current = current[part];
  }
  return typeof current === 'string' ? current : undefined;
}

function detectDefaultLanguage(): Language {
  const stored = localStorage.getItem('aiome_lang');
  if (stored === 'ja' || stored === 'en') return stored;
  const browserLang = navigator.language || '';
  return browserLang.startsWith('ja') ? 'ja' : 'en';
}

export function LanguageProvider({ children }: { children: React.ReactNode }) {
  const [lang, setLangState] = useState<Language>(detectDefaultLanguage);

  const setLang = useCallback((newLang: Language) => {
    setLangState(newLang);
    localStorage.setItem('aiome_lang', newLang);
  }, []);

  const t = useCallback((key: string, params?: Record<string, string | number>): string => {
    let value = getNestedValue(translations[lang], key)
      ?? getNestedValue(translations['en'], key)
      ?? key;

    if (params) {
      Object.entries(params).forEach(([k, v]) => {
        value = value.replace(`{{${k}}}`, String(v));
      });
    }
    return value;
  }, [lang]);

  const ctx = useMemo(() => ({ lang, setLang, t }), [lang, setLang, t]);

  return (
    <LanguageContext.Provider value={ctx}>
      {children}
    </LanguageContext.Provider>
  );
}

export function useTranslation() {
  const { t } = useContext(LanguageContext);
  return { t };
}

export function useLanguage() {
  const { lang, setLang } = useContext(LanguageContext);
  return { lang, setLang };
}

/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import { useEffect } from 'react';
import { useTranslation } from 'react-i18next';
import { Navbar } from './components/Navbar';
import { Hero } from './components/Hero';
import { Problem } from './components/Problem';
import { SocialProof } from './components/SocialProof';
import { Comparison } from './components/Comparison';
import { LiveDemo } from './components/LiveDemo';
import { Features } from './components/Features';
import { UseCases } from './components/UseCases';
import { HowItWorks } from './components/HowItWorks';
import { Economy } from './components/Economy';
import { CodePreview } from './components/CodePreview';
import { Architecture } from './components/Architecture';
import { Showcase } from './components/Showcase';
import { Pricing } from './components/Pricing';
import { Faq } from './components/Faq';
import { CTA } from './components/CTA';
import { Footer } from './components/Footer';
import { PrivacyPage, TermsPage, TokushohoPage } from './components/LegalPages';
import './i18n/config';

function App() {
  const { i18n } = useTranslation();

  // Sync <html lang="..."> with current i18n language
  useEffect(() => {
    document.documentElement.lang = i18n.language;
  }, [i18n.language]);

  const path = window.location.pathname;

  // Lightweight conditional routing
  if (path === '/privacy') {
    return <PrivacyPage />;
  }
  if (path === '/terms') {
    return <TermsPage />;
  }
  if (path === '/tokushoho') {
    return <TokushohoPage />;
  }

  return (
    <div className="min-h-screen bg-brand-bg text-white font-body selection:bg-brand-cyan/30">
      <Navbar />
      <main id="main-content">
        <Hero />
        <Problem />
        <SocialProof />
        <LiveDemo />
        <Features />
        <UseCases />
        <HowItWorks />
        <Economy />
        <CodePreview />
        <Architecture />
        <Showcase />
        <Comparison />
        <Pricing />
        <Faq />
        <CTA />
      </main>
      <Footer />
    </div>
  );
}

export default App;

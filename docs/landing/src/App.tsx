import { useEffect } from 'react';
import { useTranslation } from 'react-i18next';
import { Navbar } from './components/Navbar';
import { Hero } from './components/Hero';
import { SocialProof } from './components/SocialProof';
import { Features } from './components/Features';
import { CodePreview } from './components/CodePreview';
import { LiveDemo } from './components/LiveDemo';
import { Security } from './components/Security';
import { Showcase } from './components/Showcase';
import { Pricing } from './components/Pricing';
import { CTA } from './components/CTA';
import { Footer } from './components/Footer';
import './i18n/config';

function App() {
  const { i18n } = useTranslation();

  // Sync <html lang="..."> with current i18n language
  useEffect(() => {
    document.documentElement.lang = i18n.language;
  }, [i18n.language]);

  return (
    <div className="min-h-screen bg-brand-bg text-white font-body selection:bg-brand-cyan/30">
      <Navbar />
      <main id="main-content">
        <Hero />
        <SocialProof />
        <Features />
        <CodePreview />
        <LiveDemo />
        <Security />
        <Showcase />
        <Pricing />
        <CTA />
      </main>
      <Footer />
    </div>
  );
}

export default App;

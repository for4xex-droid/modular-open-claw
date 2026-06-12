import { useTranslation } from 'react-i18next';
import { motion } from 'framer-motion';
import { FluidHeroBackground } from './FluidHeroBackground';

export function Hero() {
  const { t } = useTranslation();

  return (
    <section className="relative pt-32 pb-24 overflow-hidden flex flex-col items-center justify-center text-center bg-brand-bg" aria-labelledby="hero-title">
      {/* Fluid Background (WebGL) with CSS fallback */}
      <FluidHeroBackground />

      {/* Non-interactive content layer — z-10, pointer-events: none
          Pointer events pass through to fluid pointer overlay (z-30).
          Canvas at z-20 with mix-blend-mode: screen renders ink OVER this layer. */}
      <div className="container mx-auto px-4 z-10 relative pointer-events-none">
        {/* Hero Brand Logo — OGP Inverted (white-ogp), 1200×630 */}
        <motion.div
          initial={{ scale: 0.9, opacity: 0 }}
          animate={{ scale: 1, opacity: 1 }}
          transition={{ duration: 0.8, ease: 'easeOut' }}
          className="relative mx-auto mb-12 max-w-2xl md:max-w-3xl"
        >
          {/* Glow behind logo */}
          <div
            className="absolute inset-0 blur-[60px] opacity-30"
            style={{ background: 'radial-gradient(ellipse at center, rgba(20,83,154,0.6) 0%, transparent 70%)' }}
            aria-hidden="true"
          />
          <img
            src="/aiome-hero-white.png"
            alt="Aiome logo"
            className="relative w-full h-auto drop-shadow-[0_0_40px_rgba(20,83,154,0.3)]"
            data-testid="hero-logo"
            fetchPriority="high"
            loading="eager"
          />
        </motion.div>

        <motion.h1
          id="hero-title"
          initial={{ filter: 'blur(8px)', opacity: 0 }}
          animate={{ filter: 'blur(0px)', opacity: 1 }}
          transition={{ duration: 0.8, delay: 0.3 }}
          className="text-4xl md:text-6xl lg:text-7xl font-extrabold tracking-tight mb-6 font-display"
        >
          {t('hero.title')}
        </motion.h1>
        
        <motion.p 
          initial={{ y: 20, opacity: 0 }}
          animate={{ y: 0, opacity: 1 }}
          transition={{ delay: 0.5, duration: 0.8 }}
          className="text-lg md:text-xl text-gray-400 max-w-3xl mx-auto mb-10 leading-relaxed"
        >
          {t('hero.subtitle')}
        </motion.p>
      </div>

      {/* CTA buttons — separate z-40 layer above all fluid layers, fully interactive */}
      <motion.div 
        initial={{ scale: 0.95, opacity: 0 }}
        animate={{ scale: 1, opacity: 1 }}
        transition={{ delay: 0.7, duration: 0.8 }}
        className="relative z-40 flex flex-col sm:flex-row items-center justify-center gap-4"
      >
        <a href="#quickstart" className="px-8 py-4 bg-brand-cyan text-black font-bold rounded-lg hover:opacity-90 transition-opacity inline-block text-center">
          {t('hero.cta_primary')}
        </a>
        <a href="https://github.com/motivationstudio-llc/aiome" target="_blank" rel="noopener noreferrer" className="px-8 py-4 bg-white/5 text-white font-bold rounded-lg border border-white/10 hover:bg-white/10 transition-colors inline-block text-center">
          {t('hero.cta_secondary')}
        </a>
      </motion.div>
    </section>
  );
}

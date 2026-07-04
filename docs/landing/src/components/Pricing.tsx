/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import { useTranslation } from 'react-i18next';
import { motion } from 'framer-motion';
import { Check, Shield, Zap } from 'lucide-react';

export function Pricing() {
  const { t } = useTranslation();

  return (
    <section id="pricing" className="py-24 relative overflow-hidden bg-brand-bg" aria-labelledby="pricing-title">
      {/* Background gradients */}
      <div className="absolute inset-0 z-0 flex justify-around items-center pointer-events-none" aria-hidden="true">
        <div className="w-[500px] h-[500px] bg-brand-cyan/10 blur-[150px] rounded-full" />
        <div className="w-[500px] h-[500px] bg-brand-cyan/10 blur-[150px] rounded-full" />
      </div>

      <div className="container mx-auto px-4 relative z-10">
        {/* Section Header */}
        <div className="text-center max-w-3xl mx-auto mb-16">
          <motion.div
            initial={{ opacity: 0, y: 20 }}
            whileInView={{ opacity: 1, y: 0 }}
            viewport={{ once: true }}
            transition={{ duration: 0.6 }}
          >
            <h2 id="pricing-title" className="text-3xl md:text-5xl font-extrabold text-white mb-6 font-display">
              {t('pricing.title')}
            </h2>
            <p className="text-xl text-gray-400 leading-relaxed">
              {t('pricing.subtitle')}
            </p>
          </motion.div>
        </div>

        {/* Pricing Grid */}
        <div className="grid md:grid-cols-2 gap-8 max-w-5xl mx-auto items-stretch">
          {/* Sovereign Free Plan */}
          <motion.div
            initial={{ opacity: 0, x: -30 }}
            whileInView={{ opacity: 1, x: 0 }}
            viewport={{ once: true }}
            transition={{ duration: 0.6, delay: 0.1 }}
            className="flex flex-col border border-white/10 bg-brand-surface/60 backdrop-blur-md rounded-3xl p-8 md:p-12 shadow-2xl relative"
          >
            <div className="flex items-center gap-3 mb-6">
              <div className="p-3 bg-white/5 rounded-xl text-brand-cyan">
                <Shield size={24} />
              </div>
              <h3 className="text-2xl font-bold text-white">{t('pricing.free_title')}</h3>
            </div>
            
            <div className="mb-8">
              <span className="text-4xl font-extrabold text-white">{t('pricing.free_price')}</span>
              <span className="text-gray-400 ml-2">{t('pricing.month')}</span>
            </div>

            <ul className="space-y-4 mb-10 flex-grow" aria-label="Free plan features">
              {[1, 2, 3, 4, 5, 6].map(n => (
                <li key={n} className="flex items-start gap-3 text-gray-300">
                  <Check className="text-brand-cyan mt-1 flex-shrink-0" size={18} />
                  <span>{t(`pricing.free_f${n}`)}</span>
                </li>
              ))}
            </ul>

            <a
              href="#quickstart"
              className="block w-full py-4 text-center border border-white/20 hover:border-white/40 text-white font-bold rounded-full transition-colors bg-white/5 hover:bg-white/10"
            >
              {t('pricing.free_cta')}
            </a>
          </motion.div>

          {/* Autonomous Pro Plan */}
          <motion.div
            initial={{ opacity: 0, x: 30 }}
            whileInView={{ opacity: 1, x: 0 }}
            viewport={{ once: true }}
            transition={{ duration: 0.6, delay: 0.2 }}
            className="flex flex-col border-2 border-brand-cyan bg-brand-surface/90 backdrop-blur-md rounded-3xl p-8 md:p-12 shadow-2xl relative overflow-hidden"
          >
            {/* "Popular" Badge */}
            <div className="absolute top-0 right-0 bg-brand-cyan text-black text-xs font-black uppercase px-6 py-2 rounded-bl-2xl">
              Highly Recommended
            </div>

            <div className="flex items-center gap-3 mb-6">
              <div className="p-3 bg-brand-cyan/10 rounded-xl text-brand-cyan animate-pulse">
                <Zap size={24} />
              </div>
              <h3 className="text-2xl font-bold text-white">{t('pricing.pro_title')}</h3>
            </div>
            
            <div className="mb-8">
              <span className="text-5xl font-black text-white">{t('pricing.pro_price')}</span>
              <span className="text-gray-400 ml-2">{t('pricing.month')}</span>
              <p className="text-sm text-brand-cyan mt-2 font-bold">{t('pricing.pro_trial')}</p>
            </div>

            <ul className="space-y-4 mb-10 flex-grow" aria-label="Pro plan features">
              {[1, 2, 3, 4, 5, 6, 7, 8].map(n => (
                <li key={n} className={`flex items-start gap-3 ${n === 1 ? 'text-brand-cyan font-bold' : 'text-gray-200 font-medium'}`}>
                  <Check className="text-brand-cyan mt-1 flex-shrink-0" size={18} />
                  <span>{t(`pricing.pro_f${n}`)}</span>
                </li>
              ))}
            </ul>

            <a
              href="https://buy.stripe.com/aFa00i9cEaVE4ay4y9f7i03"
              target="_blank"
              rel="noopener noreferrer"
              className="block w-full py-4 text-center bg-brand-cyan hover:bg-brand-cyan-hover text-black font-extrabold rounded-full transition-all duration-300 shadow-lg shadow-brand-cyan/20 hover:shadow-brand-cyan/40 transform hover:-translate-y-0.5"
            >
              {t('pricing.pro_cta')}
            </a>
          </motion.div>
        </div>

        <p className="text-xs text-gray-500 text-center mt-8 max-w-3xl mx-auto">
          {t('pricing.disclaimer')}
        </p>
      </div>
    </section>
  );
}

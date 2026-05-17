import { useTranslation } from 'react-i18next';
import { motion } from 'framer-motion';
import { ArrowRight } from 'lucide-react';

export function CTA() {
  const { t } = useTranslation();

  return (
    <section className="py-24 relative overflow-hidden bg-brand-bg" aria-labelledby="cta-title">
      {/* Background gradient */}
      <div className="absolute inset-0 z-0 flex justify-center items-center pointer-events-none" aria-hidden="true">
        <div className="w-[800px] h-[300px] bg-brand-cyan/20 blur-[120px] rounded-full" />
      </div>

      <div className="container mx-auto px-4 relative z-10 text-center">
        <motion.div
          initial={{ opacity: 0, scale: 0.95 }}
          whileInView={{ opacity: 1, scale: 1 }}
          viewport={{ once: true }}
          transition={{ duration: 0.6 }}
          className="max-w-4xl mx-auto border border-white/10 bg-brand-surface/80 backdrop-blur-md rounded-3xl p-8 md:p-16 shadow-2xl"
        >
          <h2 id="cta-title" className="text-3xl md:text-5xl font-extrabold text-white mb-6 font-display">
            {t('cta.title')}
          </h2>
          <p className="text-xl text-gray-400 mb-10 leading-relaxed">
            {t('cta.desc')}
          </p>
          <a href="#quickstart" className="inline-flex items-center gap-2 mx-auto px-8 py-4 bg-white text-black font-bold rounded-full hover:bg-gray-200 transition-colors">
            {t('cta.button')}
            <ArrowRight size={18} aria-hidden="true" />
          </a>
        </motion.div>
      </div>
    </section>
  );
}

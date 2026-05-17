import { useTranslation } from 'react-i18next';
import { motion } from 'framer-motion';
import { Lock } from 'lucide-react';

export function Security() {
  const { t } = useTranslation();

  return (
    <section className="py-24 bg-brand-bg relative" aria-labelledby="security-title">
      <div className="container mx-auto px-4 max-w-4xl text-center">
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true }}
          transition={{ duration: 0.6 }}
        >
          <div className="w-16 h-16 rounded-2xl bg-brand-purple/10 flex items-center justify-center mx-auto mb-8">
            <Lock className="text-brand-purple" size={32} aria-hidden="true" />
          </div>
          <h2 id="security-title" className="text-3xl md:text-5xl font-extrabold text-white mb-6 font-display">
            {t('security.title')}
          </h2>
          <p className="text-xl text-gray-400 leading-relaxed max-w-3xl mx-auto">
            {t('security.desc')}
          </p>
        </motion.div>
      </div>
    </section>
  );
}

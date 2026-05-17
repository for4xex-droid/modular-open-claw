import { useTranslation } from 'react-i18next';
import { motion } from 'framer-motion';

export function SocialProof() {
  const { t } = useTranslation();

  return (
    <section className="py-24 bg-brand-surface border-y border-white/5 relative z-10" aria-labelledby="social-proof-heading">
      <div className="container mx-auto px-4">
        {/* Visually hidden heading for proper heading hierarchy */}
        <h2 id="social-proof-heading" className="sr-only">Key metrics</h2>
        <div className="grid grid-cols-1 md:grid-cols-3 gap-12 divide-y md:divide-y-0 md:divide-x divide-white/5">
          {[1, 2, 3].map((num) => (
            <motion.div 
              key={num}
              initial={{ opacity: 0, y: 20 }}
              whileInView={{ opacity: 1, y: 0 }}
              viewport={{ once: true, margin: "-50px" }}
              transition={{ delay: num * 0.1, duration: 0.8 }}
              className="flex flex-col items-center text-center px-4 pt-8 md:pt-0 first:pt-0"
            >
              <h3 className="text-5xl md:text-6xl font-extrabold text-white mb-2 font-display">
                {t(`social_proof.metric${num}_value`)}
              </h3>
              <p className="text-lg text-brand-cyan mb-4 font-semibold">
                {t(`social_proof.metric${num}_label`)}
              </p>
              <p className="text-sm text-gray-400 italic max-w-xs">
                {t(`social_proof.metric${num}_desc`)}
              </p>
            </motion.div>
          ))}
        </div>
      </div>
    </section>
  );
}

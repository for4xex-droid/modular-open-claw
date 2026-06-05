import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { motion } from 'framer-motion';
import { ArrowRight, Check } from 'lucide-react';

export function CTA() {
  const { t } = useTranslation();
  const [email, setEmail] = useState('');
  const [gdpr, setGdpr] = useState(false);
  const [submitted, setSubmitted] = useState(false);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState(false);
  const formspreeId = import.meta.env.VITE_FORMSPREE_ID;

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!gdpr || !email || loading || !formspreeId) return;

    setLoading(true);
    setError(false);
    try {
      const res = await fetch(`https://formspree.io/f/${formspreeId}`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ email: email.trim() }),
      });
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      setSubmitted(true);
    } catch (err) {
      console.error('Waitlist submission error:', err);
      setError(true);
    } finally {
      setLoading(false);
    }
  };

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

          <div className="flex flex-col lg:flex-row items-center justify-center gap-12 max-w-3xl mx-auto">
            {/* Deploy Button */}
            <div className="flex-shrink-0">
              <a href="#quickstart" className="inline-flex items-center gap-2 px-8 py-4 bg-white hover:bg-gray-200 text-black font-bold rounded-full transition-colors">
                {t('cta.button')}
                <ArrowRight size={18} aria-hidden="true" />
              </a>
            </div>

            {formspreeId && (
              <>
                {/* Separator */}
                <div className="hidden lg:block w-px h-16 bg-white/10" aria-hidden="true" />
                <div className="lg:hidden w-full h-px bg-white/10" aria-hidden="true" />

                {/* Waitlist Email Form */}
                <div className="w-full max-w-md text-left">
                  {submitted ? (
                    <motion.div 
                      initial={{ opacity: 0, scale: 0.9 }}
                      animate={{ opacity: 1, scale: 1 }}
                      className="p-4 bg-emerald-500/10 border border-emerald-500/20 rounded-2xl flex items-center gap-3 text-emerald-400 font-medium"
                    >
                      <span className="p-1 bg-emerald-500/20 rounded-full flex-shrink-0">
                        <Check size={18} aria-hidden="true" />
                      </span>
                      <span>{t('cta.email_success')}</span>
                    </motion.div>
                  ) : (
                    <form onSubmit={handleSubmit} className="space-y-4">
                      {error && (
                        <motion.div
                          initial={{ opacity: 0, y: -4 }}
                          animate={{ opacity: 1, y: 0 }}
                          className="p-3 bg-brand-rose/10 border border-brand-rose/20 rounded-xl text-brand-rose font-medium text-sm"
                          role="alert"
                        >
                          {t('cta.email_error', 'Something went wrong. Please try again.')}
                        </motion.div>
                      )}
                      <div className="flex gap-2">
                        <input
                          id="waitlist-email"
                          type="email"
                          required
                          autoComplete="email"
                          value={email}
                          onChange={(e) => setEmail(e.target.value)}
                          placeholder={t('cta.email_placeholder') || ''}
                          className="flex-grow px-4 py-3 bg-white/5 border border-white/10 rounded-xl text-white placeholder-gray-500 focus:outline-none focus:border-brand-cyan focus:ring-2 focus:ring-brand-cyan/50 transition-colors"
                          aria-label="Email address"
                        />
                        <button
                          type="submit"
                          disabled={!gdpr || loading}
                          className="px-6 py-3 bg-brand-cyan hover:opacity-90 disabled:opacity-50 disabled:cursor-not-allowed text-black font-bold rounded-xl transition-all duration-300 flex-shrink-0"
                        >
                          {t('cta.notify_me')}
                        </button>
                      </div>
                      <label htmlFor="gdpr-consent" className="flex items-start gap-2.5 text-xs text-gray-500 cursor-pointer select-none">
                        <input
                          id="gdpr-consent"
                          type="checkbox"
                          checked={gdpr}
                          onChange={(e) => setGdpr(e.target.checked)}
                          className="mt-0.5 rounded border-white/10 bg-white/5 text-brand-cyan focus:ring-brand-cyan focus:ring-offset-brand-bg"
                        />
                        <span>{t('cta.gdpr_consent')}</span>
                      </label>
                    </form>
                  )}
                </div>
              </>
            )}
          </div>
        </motion.div>
      </div>
    </section>
  );
}

/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import { useTranslation } from 'react-i18next';
import { motion } from 'framer-motion';

/** R4-2 / OP-063: 実機証拠ビジュアル（docs/assets/evidence → public/evidence） */
const EVIDENCE_STILLS = [
  { img: '/evidence/02-audit.webp', altKey: 'showcase.img_audit' },
  { img: '/evidence/03-buzz-approval.webp', altKey: 'showcase.img_buzz' },
  { img: '/evidence/04-nurture-economy.webp', altKey: 'showcase.img_economy' },
  { img: '/evidence/05-workflow-builder.webp', altKey: 'showcase.img_workflow' },
  { img: '/evidence/06-agent-diorama.webp', altKey: 'showcase.img_agent' },
  { img: '/evidence/07-prompt-stats.webp', altKey: 'showcase.img_stats' },
] as const;

export function Showcase() {
  const { t } = useTranslation();

  return (
    <section id="showcase" className="py-24 relative overflow-hidden bg-brand-bg" aria-labelledby="showcase-title">
      <div className="absolute inset-0 z-0 flex justify-around items-center pointer-events-none" aria-hidden="true">
        <div className="w-[500px] h-[500px] bg-brand-cyan/5 blur-[150px] rounded-full" />
      </div>

      <div className="container mx-auto px-4 relative z-10">
        <div className="text-center max-w-3xl mx-auto mb-16">
          <motion.div
            initial={{ opacity: 0, y: 20 }}
            whileInView={{ opacity: 1, y: 0 }}
            viewport={{ once: true }}
            transition={{ duration: 0.6 }}
          >
            <h2 id="showcase-title" className="text-3xl md:text-5xl font-extrabold text-white mb-6 font-display">
              {t('showcase.title')}
            </h2>
            <p className="text-xl text-gray-400 leading-relaxed">
              {t('showcase.desc')}
            </p>
          </motion.div>
        </div>

        <motion.div
          initial={{ opacity: 0, y: 24 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true }}
          transition={{ duration: 0.6 }}
          className="max-w-5xl mx-auto mb-12 border border-white/10 bg-brand-surface/40 backdrop-blur-md rounded-2xl overflow-hidden shadow-2xl"
        >
          <div className="aspect-video w-full overflow-hidden relative border-b border-white/5 bg-black/40">
            <img
              src="/evidence/01-quickstart.webp"
              alt={t('showcase.gif_alt')}
              className="object-contain w-full h-full"
              loading="lazy"
            />
          </div>
          <div className="p-5">
            <h3 className="text-lg font-bold text-white font-display">{t('showcase.gif_caption')}</h3>
          </div>
        </motion.div>

        <div className="grid sm:grid-cols-2 lg:grid-cols-3 gap-6 max-w-6xl mx-auto">
          {EVIDENCE_STILLS.map((item, i) => (
            <motion.div
              key={item.img}
              initial={{ opacity: 0, y: 30 }}
              whileInView={{ opacity: 1, y: 0 }}
              viewport={{ once: true }}
              transition={{ duration: 0.6, delay: i * 0.08 }}
              className="group flex flex-col border border-white/10 bg-brand-surface/40 backdrop-blur-md rounded-2xl overflow-hidden shadow-2xl relative"
              style={{ contentVisibility: 'auto' }}
            >
              <div className="aspect-video w-full overflow-hidden relative border-b border-white/5">
                <img
                  src={item.img}
                  alt={t(item.altKey)}
                  className="object-cover w-full h-full transform group-hover:scale-105 transition-transform duration-500"
                  loading="lazy"
                />
                <div className="absolute inset-0 bg-gradient-to-t from-brand-bg/80 via-transparent to-transparent opacity-0 group-hover:opacity-100 transition-opacity duration-300" />
              </div>
              <div className="p-5">
                <h3 className="text-base font-bold text-white group-hover:text-brand-cyan transition-colors font-display">
                  {t(item.altKey)}
                </h3>
              </div>
            </motion.div>
          ))}
        </div>
      </div>
    </section>
  );
}

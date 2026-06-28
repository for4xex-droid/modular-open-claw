/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import { useTranslation } from 'react-i18next';
import { motion } from 'framer-motion';

export function Showcase() {
  const { t } = useTranslation();

  const items = [
    {
      img: '/dashboard_mock.webp',
      alt: t('showcase.img1_alt'),
    },
    {
      img: '/timeline_mock.webp',
      alt: t('showcase.img2_alt'),
    },
    {
      img: '/avatar_mock.webp',
      alt: t('showcase.img3_alt'),
    },
  ];

  return (
    <section id="showcase" className="py-24 relative overflow-hidden bg-brand-bg" aria-labelledby="showcase-title">
      <div className="absolute inset-0 z-0 flex justify-around items-center pointer-events-none" aria-hidden="true">
        <div className="w-[500px] h-[500px] bg-brand-purple/5 blur-[150px] rounded-full" />
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

        <div className="grid md:grid-cols-3 gap-8 max-w-6xl mx-auto">
          {items.map((item, i) => (
            <motion.div
              key={i}
              initial={{ opacity: 0, y: 30 }}
              whileInView={{ opacity: 1, y: 0 }}
              viewport={{ once: true }}
              transition={{ duration: 0.6, delay: i * 0.15 }}
              className="group flex flex-col border border-white/10 bg-brand-surface/40 backdrop-blur-md rounded-2xl overflow-hidden shadow-2xl relative"
              style={{ contentVisibility: 'auto' }}
            >
              <div className="aspect-video w-full overflow-hidden relative border-b border-white/5">
                <img
                  src={item.img}
                  alt={item.alt}
                  className="object-cover w-full h-full transform group-hover:scale-105 transition-transform duration-500"
                  loading="lazy"
                />
                <div className="absolute inset-0 bg-gradient-to-t from-brand-bg/80 via-transparent to-transparent opacity-0 group-hover:opacity-100 transition-opacity duration-300" />
              </div>
              <div className="p-6">
                <h3 className="text-lg font-bold text-white group-hover:text-brand-cyan transition-colors font-display">
                  {item.alt}
                </h3>
              </div>
            </motion.div>
          ))}
        </div>
      </div>
    </section>
  );
}

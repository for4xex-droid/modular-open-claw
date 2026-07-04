/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import { useTranslation } from 'react-i18next';
import { motion } from 'framer-motion';
import { CloudOff, AlertTriangle, EyeOff } from 'lucide-react';

/**
 * Empathy-first section (viral principle #21): describe the visitor's
 * problem better than they can, before selling the solution.
 */
export function Problem() {
  const { t } = useTranslation();

  const pains = [
    { icon: CloudOff, title: t('problem.p1_title'), desc: t('problem.p1_desc') },
    { icon: AlertTriangle, title: t('problem.p2_title'), desc: t('problem.p2_desc') },
    { icon: EyeOff, title: t('problem.p3_title'), desc: t('problem.p3_desc') },
  ];

  return (
    <section className="py-24 bg-brand-bg relative" aria-labelledby="problem-title">
      <div className="container mx-auto px-4">
        <div className="text-center max-w-3xl mx-auto mb-16">
          <motion.div
            initial={{ opacity: 0, y: 20 }}
            whileInView={{ opacity: 1, y: 0 }}
            viewport={{ once: true }}
            transition={{ duration: 0.6 }}
          >
            <h2 id="problem-title" className="text-3xl md:text-5xl font-extrabold text-white mb-6 font-display">
              {t('problem.title')}
            </h2>
            <p className="text-xl text-gray-400 leading-relaxed">
              {t('problem.desc')}
            </p>
          </motion.div>
        </div>

        <div className="grid md:grid-cols-3 gap-8 max-w-5xl mx-auto">
          {pains.map((pain, i) => (
            <motion.div
              key={pain.title}
              initial={{ opacity: 0, y: 20 }}
              whileInView={{ opacity: 1, y: 0 }}
              viewport={{ once: true }}
              transition={{ duration: 0.6, delay: i * 0.1 }}
              className="border border-white/10 bg-brand-surface/60 rounded-2xl p-8"
            >
              <div className="inline-flex p-3 bg-white/5 rounded-xl text-gray-300 mb-5">
                <pain.icon size={24} aria-hidden="true" />
              </div>
              <h3 className="text-xl font-bold text-white mb-3">{pain.title}</h3>
              <p className="text-gray-400 leading-relaxed">{pain.desc}</p>
            </motion.div>
          ))}
        </div>

        <motion.p
          initial={{ opacity: 0 }}
          whileInView={{ opacity: 1 }}
          viewport={{ once: true }}
          transition={{ duration: 0.6, delay: 0.4 }}
          className="text-center text-lg md:text-xl text-brand-cyan font-bold mt-14"
        >
          {t('problem.bridge')}
        </motion.p>
      </div>
    </section>
  );
}

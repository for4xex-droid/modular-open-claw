/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import { useTranslation } from 'react-i18next';
import { motion } from 'framer-motion';
import { Check, Minus } from 'lucide-react';

/**
 * Category comparison table (viral principle #31): show why switching
 * is worth it. Compares product categories, not named competitors,
 * per the MESSAGING.md rule against unverifiable claims.
 */
export function Comparison() {
  const { t } = useTranslation();

  const rows = [1, 2, 3, 4, 5, 6].map((n) => ({
    label: t(`comparison.row${n}`),
    cloud: t(`comparison.row${n}_cloud`),
    framework: t(`comparison.row${n}_framework`),
    aiome: t(`comparison.row${n}_aiome`),
  }));

  return (
    <section className="py-24 bg-brand-bg relative" aria-labelledby="comparison-title">
      <div className="container mx-auto px-4">
        <div className="text-center max-w-3xl mx-auto mb-16">
          <motion.div
            initial={{ opacity: 0, y: 20 }}
            whileInView={{ opacity: 1, y: 0 }}
            viewport={{ once: true }}
            transition={{ duration: 0.6 }}
          >
            <h2 id="comparison-title" className="text-3xl md:text-5xl font-extrabold text-white mb-6 font-display">
              {t('comparison.title')}
            </h2>
            <p className="text-xl text-gray-400 leading-relaxed">
              {t('comparison.desc')}
            </p>
          </motion.div>
        </div>

        <motion.div
          initial={{ opacity: 0, y: 20 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true }}
          transition={{ duration: 0.6, delay: 0.1 }}
          className="max-w-5xl mx-auto overflow-x-auto rounded-2xl border border-white/10"
        >
          <table className="w-full text-left text-sm md:text-base border-collapse min-w-[720px]">
            <caption className="sr-only">{t('comparison.title')}</caption>
            <thead>
              <tr className="bg-brand-surface/80 text-gray-300">
                <th scope="col" className="px-5 py-4 font-semibold w-[22%]" aria-label="Criteria" />
                <th scope="col" className="px-5 py-4 font-semibold w-[24%]">{t('comparison.col_cloud')}</th>
                <th scope="col" className="px-5 py-4 font-semibold w-[24%]">{t('comparison.col_framework')}</th>
                <th scope="col" className="px-5 py-4 font-bold text-brand-cyan bg-brand-cyan/5 w-[30%]">{t('comparison.col_aiome')}</th>
              </tr>
            </thead>
            <tbody>
              {rows.map((row, i) => (
                <tr key={row.label} className={i % 2 === 0 ? 'bg-white/[0.02]' : ''}>
                  <th scope="row" className="px-5 py-4 font-semibold text-white align-top">{row.label}</th>
                  <td className="px-5 py-4 text-gray-400 align-top">
                    <span className="inline-flex items-start gap-2">
                      <Minus size={16} className="mt-1 flex-shrink-0 text-gray-600" aria-hidden="true" />
                      {row.cloud}
                    </span>
                  </td>
                  <td className="px-5 py-4 text-gray-400 align-top">
                    <span className="inline-flex items-start gap-2">
                      <Minus size={16} className="mt-1 flex-shrink-0 text-gray-600" aria-hidden="true" />
                      {row.framework}
                    </span>
                  </td>
                  <td className="px-5 py-4 text-gray-100 font-medium bg-brand-cyan/5 align-top">
                    <span className="inline-flex items-start gap-2">
                      <Check size={16} className="mt-1 flex-shrink-0 text-brand-cyan" aria-hidden="true" />
                      {row.aiome}
                    </span>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </motion.div>

        <p className="text-xs text-gray-500 text-center mt-6 max-w-3xl mx-auto">
          {t('comparison.note')}
        </p>
      </div>
    </section>
  );
}

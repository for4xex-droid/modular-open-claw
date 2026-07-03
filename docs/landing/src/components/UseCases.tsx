/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import { useTranslation } from 'react-i18next';
import { motion } from 'framer-motion';
import { Search, Share2, TrendingUp, Headphones } from 'lucide-react';

export function UseCases() {
  const { t } = useTranslation();

  const cases = [
    { id: 1, icon: Search },
    { id: 2, icon: Share2 },
    { id: 3, icon: TrendingUp },
    { id: 4, icon: Headphones },
  ];

  return (
    <section id="use-cases" className="py-24 bg-brand-bg relative" aria-labelledby="use-cases-title">
      <div className="container mx-auto px-4 max-w-6xl">
        <div className="text-center mb-16">
          <motion.div
            initial={{ opacity: 0, y: 20 }}
            whileInView={{ opacity: 1, y: 0 }}
            viewport={{ once: true }}
            transition={{ duration: 0.6 }}
          >
            <h2 id="use-cases-title" className="text-3xl md:text-5xl font-extrabold text-white mb-6 font-display">
              {t('use_cases.title')}
            </h2>
            <p className="text-xl text-gray-400 max-w-3xl mx-auto leading-relaxed">
              {t('use_cases.desc')}
            </p>
          </motion.div>
        </div>

        <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
          {cases.map((useCase, index) => {
            const Icon = useCase.icon;
            return (
              <motion.div
                key={useCase.id}
                initial={{ opacity: 0, y: 20 }}
                whileInView={{ opacity: 1, y: 0 }}
                viewport={{ once: true, margin: "-50px" }}
                transition={{ delay: index * 0.1, duration: 0.6 }}
                className="p-8 rounded-2xl bg-brand-surface border border-white/10 hover:border-brand-cyan/30 transition-colors duration-500 relative overflow-hidden group"
              >
                <div className="absolute -bottom-6 -right-6 p-6 opacity-5 group-hover:opacity-10 transition-opacity duration-500 pointer-events-none" aria-hidden="true">
                  <Icon size={160} />
                </div>

                <div className="relative z-10">
                  <div className="w-12 h-12 rounded-lg bg-white/5 flex items-center justify-center mb-6 group-hover:bg-brand-cyan/10 group-hover:text-brand-cyan transition-colors duration-500 text-gray-400">
                    <Icon size={24} aria-hidden="true" />
                  </div>

                  <h3 className="text-2xl font-bold text-white mb-4 font-display">
                    {t(`use_cases.case${useCase.id}_title`)}
                  </h3>

                  <p className="text-gray-400 leading-relaxed text-lg max-w-2xl">
                    {t(`use_cases.case${useCase.id}_desc`)}
                  </p>
                </div>
              </motion.div>
            );
          })}
        </div>
      </div>
    </section>
  );
}

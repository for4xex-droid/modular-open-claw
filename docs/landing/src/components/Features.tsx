/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import { useTranslation } from 'react-i18next';
import { motion } from 'framer-motion';
import { Cpu, ShieldCheck, Zap, Store, Coins, Gift } from 'lucide-react';

export function Features() {
  const { t } = useTranslation();

  const features = [
    { id: 1, icon: Cpu, colSpan: "md:col-span-2" },
    { id: 2, icon: ShieldCheck, colSpan: "md:col-span-1" },
    { id: 3, icon: Zap, colSpan: "md:col-span-1" },
    { id: 4, icon: Store, colSpan: "md:col-span-2" },
    { id: 5, icon: Coins, colSpan: "md:col-span-2" },
    { id: 6, icon: Gift, colSpan: "md:col-span-1" },
  ];

  return (
    <section id="features" className="py-24 bg-brand-bg relative" aria-labelledby="features-title">
      <div className="container mx-auto px-4 max-w-6xl">
        <div className="text-center mb-16">
          <h2 id="features-title" className="text-3xl md:text-5xl font-extrabold text-white font-display">
            {t('features.title')}
          </h2>
        </div>

        <div className="grid grid-cols-1 md:grid-cols-3 gap-6">
          {features.map((feature, index) => {
            const Icon = feature.icon;
            return (
              <motion.div
                key={feature.id}
                initial={{ opacity: 0, y: 20 }}
                whileInView={{ opacity: 1, y: 0 }}
                viewport={{ once: true, margin: "-50px" }}
                transition={{ delay: index * 0.1, duration: 0.6 }}
                className={`p-8 rounded-2xl bg-brand-surface border border-white/10 hover:border-brand-cyan/30 transition-colors duration-500 ${feature.colSpan} relative overflow-hidden group`}
              >
                {/* Background oversized icon for visual interest */}
                <div className="absolute -bottom-6 -right-6 p-6 opacity-5 group-hover:opacity-10 transition-opacity duration-500 pointer-events-none" aria-hidden="true">
                  <Icon size={160} />
                </div>
                
                <div className="relative z-10">
                  <div className="w-12 h-12 rounded-lg bg-white/5 flex items-center justify-center mb-6 group-hover:bg-brand-cyan/10 group-hover:text-brand-cyan transition-colors duration-500 text-gray-400">
                    <Icon size={24} aria-hidden="true" />
                  </div>
                  
                  <h3 className="text-2xl font-bold text-white mb-4 font-display">
                    {t(`features.f${feature.id}_title`)}
                  </h3>
                  
                  <p className="text-gray-400 leading-relaxed text-lg max-w-2xl">
                    {t(`features.f${feature.id}_desc`)}
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

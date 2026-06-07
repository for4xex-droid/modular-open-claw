import React from 'react';
import { useTranslation } from 'react-i18next';
import { motion } from 'framer-motion';
import { Shield, Box, Dna } from 'lucide-react';

export const Architecture: React.FC = () => {
  const { t } = useTranslation();

  const layers = [
    {
      id: 'l1',
      title: t('architecture.l1_title'),
      desc: t('architecture.l1_desc'),
      icon: Shield,
      color: 'text-brand-cyan',
      bgColor: 'bg-brand-cyan/10',
    },
    {
      id: 'l2',
      title: t('architecture.l2_title'),
      desc: t('architecture.l2_desc'),
      icon: Box,
      color: 'text-brand-purple',
      bgColor: 'bg-brand-purple/10',
    },
    {
      id: 'l3',
      title: t('architecture.l3_title'),
      desc: t('architecture.l3_desc'),
      icon: Dna,
      color: 'text-brand-cyan',
      bgColor: 'bg-brand-cyan/10',
    },
  ];

  return (
    <section id="architecture" className="py-24 bg-brand-bg relative overflow-hidden" aria-labelledby="architecture-title">
      <div className="container mx-auto px-4 max-w-6xl">
        <div className="text-center mb-16">
          <motion.div
            initial={{ opacity: 0, y: 20 }}
            whileInView={{ opacity: 1, y: 0 }}
            viewport={{ once: true }}
            transition={{ duration: 0.6 }}
          >
            <h2 id="architecture-title" className="text-3xl md:text-5xl font-extrabold text-white mb-6 font-display">
              {t('architecture.title')}
            </h2>
          </motion.div>
        </div>

        <div className="grid grid-cols-1 md:grid-cols-3 gap-8">
          {layers.map((layer, index) => {
            const Icon = layer.icon;
            return (
              <motion.div
                key={layer.id}
                initial={{ opacity: 0, y: 30 }}
                whileInView={{ opacity: 1, y: 0 }}
                viewport={{ once: true }}
                transition={{ duration: 0.5, delay: index * 0.1 }}
                className="backdrop-blur-md bg-white/[0.02] border border-white/5 hover:border-white/10 hover:bg-white/[0.04] transition-all duration-300 rounded-3xl p-8 flex flex-col items-start"
              >
                <div className={`w-12 h-12 rounded-2xl ${layer.bgColor} flex items-center justify-center mb-6`}>
                  <Icon className={layer.color} size={24} aria-hidden="true" />
                </div>
                <h3 className="text-xl font-bold text-white mb-4 font-display">
                  {layer.title}
                </h3>
                <p className="text-gray-400 leading-relaxed">
                  {layer.desc}
                </p>
              </motion.div>
            );
          })}
        </div>
      </div>
    </section>
  );
};

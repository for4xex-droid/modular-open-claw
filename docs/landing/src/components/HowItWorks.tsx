import React from 'react';
import { useTranslation } from 'react-i18next';
import { motion } from 'framer-motion';
import { Container, Brain, TrendingUp } from 'lucide-react';

export const HowItWorks: React.FC = () => {
  const { t } = useTranslation();

  const steps = [
    {
      id: 1,
      title: t('how_it_works.step1_title'),
      desc: t('how_it_works.step1_desc'),
      icon: Container,
      color: 'text-brand-cyan',
      bgColor: 'bg-brand-cyan/10',
    },
    {
      id: 2,
      title: t('how_it_works.step2_title'),
      desc: t('how_it_works.step2_desc'),
      icon: Brain,
      color: 'text-brand-purple',
      bgColor: 'bg-brand-purple/10',
    },
    {
      id: 3,
      title: t('how_it_works.step3_title'),
      desc: t('how_it_works.step3_desc'),
      icon: TrendingUp,
      color: 'text-brand-cyan',
      bgColor: 'bg-brand-cyan/10',
    },
  ];

  return (
    <section id="how-it-works" className="py-24 bg-brand-bg relative overflow-hidden" aria-labelledby="how-it-works-title">
      <div className="container mx-auto px-4 max-w-6xl relative z-10">
        <div className="text-center mb-20">
          <motion.div
            initial={{ opacity: 0, y: 20 }}
            whileInView={{ opacity: 1, y: 0 }}
            viewport={{ once: true }}
            transition={{ duration: 0.6 }}
          >
            <h2 id="how-it-works-title" className="text-3xl md:text-5xl font-extrabold text-white mb-6 font-display">
              {t('how_it_works.title')}
            </h2>
          </motion.div>
        </div>

        {/* Steps container */}
        <div className="relative">
          {/* Connector Line for Desktop */}
          <div className="absolute top-1/2 left-0 w-full h-0.5 bg-gradient-to-r from-brand-cyan/50 via-brand-purple/50 to-brand-cyan/50 -translate-y-1/2 hidden md:block z-0" />

          <div className="grid grid-cols-1 md:grid-cols-3 gap-12 relative z-10">
            {steps.map((step, index) => {
              const Icon = step.icon;
              return (
                <motion.div
                  key={step.id}
                  initial={{ opacity: 0, y: 40 }}
                  whileInView={{ opacity: 1, y: 0 }}
                  viewport={{ once: true }}
                  transition={{ duration: 0.6, delay: index * 0.15 }}
                  className="flex flex-col items-center text-center px-4"
                >
                  {/* Circle / Icon Holder */}
                  <div className={`w-20 h-20 rounded-full ${step.bgColor} border-2 border-white/5 flex items-center justify-center mb-8 relative bg-brand-bg hover:scale-105 transition-transform duration-300`}>
                    <Icon className={step.color} size={32} aria-hidden="true" />
                    {/* Step Number Badge */}
                    <div className="absolute -top-1 -right-1 w-6 h-6 rounded-full bg-white text-brand-bg text-xs font-bold flex items-center justify-center font-display">
                      {step.id}
                    </div>
                  </div>
                  
                  <h3 className="text-2xl font-bold text-white mb-4 font-display">
                    {step.title}
                  </h3>
                  <p className="text-gray-400 leading-relaxed max-w-sm">
                    {step.desc}
                  </p>
                </motion.div>
              );
            })}
          </div>
        </div>
      </div>
    </section>
  );
};

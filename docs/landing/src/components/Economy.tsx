import React from 'react';
import { useTranslation } from 'react-i18next';
import { motion } from 'framer-motion';
import { ShoppingCart, ArrowLeftRight, Gift } from 'lucide-react';

export const Economy: React.FC = () => {
  const { t } = useTranslation();

  const cards = [
    {
      id: 'c1',
      title: t('economy.card1_title'),
      subtitle: t('economy.card1_subtitle'),
      desc: t('economy.card1_desc'),
      icon: ShoppingCart,
      color: 'text-brand-cyan',
      bgColor: 'bg-brand-cyan/10',
    },
    {
      id: 'c2',
      title: t('economy.card2_title'),
      subtitle: t('economy.card2_subtitle'),
      desc: t('economy.card2_desc'),
      icon: ArrowLeftRight,
      color: 'text-brand-purple',
      bgColor: 'bg-brand-purple/10',
    },
    {
      id: 'c3',
      title: t('economy.card3_title'),
      subtitle: t('economy.card3_subtitle'),
      desc: t('economy.card3_desc'),
      icon: Gift,
      color: 'text-brand-cyan',
      bgColor: 'bg-brand-cyan/10',
    },
  ];

  return (
    <section id="economy" className="py-24 bg-brand-bg relative overflow-hidden" aria-labelledby="economy-title">
      <div className="container mx-auto px-4 max-w-6xl relative z-10">
        <div className="text-center mb-16">
          <motion.div
            initial={{ opacity: 0, y: 20 }}
            whileInView={{ opacity: 1, y: 0 }}
            viewport={{ once: true }}
            transition={{ duration: 0.6 }}
          >
            <h2 id="economy-title" className="text-3xl md:text-5xl font-extrabold text-white mb-6 font-display">
              {t('economy.title')}
            </h2>
            <p className="text-xl text-gray-400 max-w-3xl mx-auto leading-relaxed">
              {t('economy.desc')}
            </p>
          </motion.div>
        </div>

        {/* 3 Columns Grid */}
        <div className="grid grid-cols-1 md:grid-cols-3 gap-8 mb-16">
          {cards.map((card, index) => {
            const Icon = card.icon;
            return (
              <motion.div
                key={card.id}
                initial={{ opacity: 0, y: 30 }}
                whileInView={{ opacity: 1, y: 0 }}
                viewport={{ once: true }}
                transition={{ duration: 0.5, delay: index * 0.1 }}
                className="backdrop-blur-md bg-white/[0.02] border border-white/5 hover:border-white/10 hover:bg-white/[0.04] transition-all duration-300 rounded-3xl p-8 flex flex-col items-start"
              >
                <div className="flex items-center justify-between w-full mb-6">
                  <div className={`w-12 h-12 rounded-2xl ${card.bgColor} flex items-center justify-center`}>
                    <Icon className={card.color} size={24} aria-hidden="true" />
                  </div>
                  <span className="text-sm font-bold text-brand-purple tracking-widest font-display px-3 py-1 rounded-full bg-brand-purple/5 border border-brand-purple/10">
                    {card.subtitle}
                  </span>
                </div>
                <h3 className="text-xl font-bold text-white mb-4 font-display">
                  {card.title}
                </h3>
                <p className="text-gray-400 leading-relaxed">
                  {card.desc}
                </p>
              </motion.div>
            );
          })}
        </div>

        {/* Bottom Mock Mode Note */}
        <motion.div
          initial={{ opacity: 0 }}
          whileInView={{ opacity: 1 }}
          viewport={{ once: true }}
          transition={{ duration: 0.8 }}
          className="text-center"
        >
          <span className="inline-block backdrop-blur-md bg-white/[0.02] border border-white/5 px-6 py-3 rounded-full text-gray-400 text-sm font-medium">
            💡 {t('economy.mock_note')}
          </span>
        </motion.div>
      </div>
    </section>
  );
};

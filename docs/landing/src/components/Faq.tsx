/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { motion } from 'framer-motion';
import { ChevronDown } from 'lucide-react';

export function Faq() {
  const { t } = useTranslation();
  const [openIndex, setOpenIndex] = useState<number | null>(null);

  const items = [1, 2, 3, 4];

  const toggle = (index: number) => {
    setOpenIndex(openIndex === index ? null : index);
  };

  return (
    <section id="faq" className="py-24 bg-brand-bg relative" aria-labelledby="faq-title">
      <div className="container mx-auto px-4 max-w-3xl">
        <div className="text-center mb-16">
          <motion.div
            initial={{ opacity: 0, y: 20 }}
            whileInView={{ opacity: 1, y: 0 }}
            viewport={{ once: true }}
            transition={{ duration: 0.6 }}
          >
            <h2 id="faq-title" className="text-3xl md:text-5xl font-extrabold text-white font-display">
              {t('faq.title')}
            </h2>
          </motion.div>
        </div>

        <div className="space-y-4">
          {items.map((n, index) => {
            const isOpen = openIndex === index;
            return (
              <motion.div
                key={n}
                initial={{ opacity: 0, y: 20 }}
                whileInView={{ opacity: 1, y: 0 }}
                viewport={{ once: true, margin: "-50px" }}
                transition={{ delay: index * 0.1, duration: 0.6 }}
                className="rounded-2xl bg-brand-surface border border-white/10 hover:border-brand-cyan/30 transition-colors duration-500 overflow-hidden"
              >
                <button
                  type="button"
                  id={`faq-question-${n}`}
                  aria-expanded={isOpen}
                  aria-controls={`faq-answer-${n}`}
                  onClick={() => toggle(index)}
                  className="w-full flex items-center justify-between gap-4 p-6 text-left cursor-pointer"
                >
                  <span className="text-lg font-bold text-white font-display">
                    {t(`faq.q${n}`)}
                  </span>
                  <ChevronDown
                    size={20}
                    className={`text-gray-400 flex-shrink-0 transition-transform duration-300 ${isOpen ? 'rotate-180 text-brand-cyan' : ''}`}
                    aria-hidden="true"
                  />
                </button>
                {isOpen && (
                  <div
                    id={`faq-answer-${n}`}
                    role="region"
                    aria-labelledby={`faq-question-${n}`}
                    className="px-6 pb-6 text-gray-400 leading-relaxed"
                  >
                    {t(`faq.a${n}`)}
                  </div>
                )}
              </motion.div>
            );
          })}
        </div>
      </div>
    </section>
  );
}

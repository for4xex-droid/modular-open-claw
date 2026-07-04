/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import { useState, useEffect } from 'react';
import { useTranslation } from 'react-i18next';
import { motion, AnimatePresence } from 'framer-motion';
import { Play, RotateCcw, Cpu, Search, AlertTriangle, CheckCircle, Award, TrendingUp } from 'lucide-react';

export function LiveDemo() {
  const { t } = useTranslation();
  const [isPlaying, setIsPlaying] = useState(false);
  const [step, setStep] = useState(0);

  const steps = [
    { icon: Cpu, label: t('live_demo.step1'), color: 'text-brand-cyan' },
    { icon: Search, label: t('live_demo.step2'), color: 'text-brand-cyan' },
    { icon: AlertTriangle, label: t('live_demo.step3'), color: 'text-brand-rose' },
    { icon: Cpu, label: t('live_demo.step4'), color: 'text-brand-cyan' },
    { icon: Search, label: t('live_demo.step5'), color: 'text-brand-cyan' },
    { icon: CheckCircle, label: t('live_demo.step6'), color: 'text-emerald-400' },
    { icon: Award, label: t('live_demo.step7'), color: 'text-brand-cyan' },
    { icon: TrendingUp, label: t('live_demo.step8'), color: 'text-brand-cyan' },
  ];

  useEffect(() => {
    let interval: ReturnType<typeof setInterval>;
    if (isPlaying) {
      interval = setInterval(() => {
        setStep((prev) => {
          if (prev >= steps.length - 1) {
            setIsPlaying(false);
            return prev;
          }
          return prev + 1;
        });
      }, 7500); // 8 steps * 7.5s = 60s total demo
    }
    return () => clearInterval(interval);
  }, [isPlaying]);

  const handleStart = () => {
    setStep(0);
    setIsPlaying(true);
  };

  const currentStep = steps[step];
  const StepIcon = currentStep?.icon || Cpu;

  return (
    <section id="live-demo" className="py-24 relative overflow-hidden bg-brand-surface border-y border-white/5" aria-labelledby="demo-title">
      <div className="absolute inset-0 z-0 flex justify-around items-center pointer-events-none" aria-hidden="true">
        <div className="w-[400px] h-[400px] bg-brand-rose/5 blur-[120px] rounded-full" />
      </div>

      <div className="container mx-auto px-4 max-w-4xl relative z-10 text-center">
        <div className="max-w-3xl mx-auto mb-16">
          <h2 id="demo-title" className="text-3xl md:text-5xl font-extrabold text-white mb-6 font-display">
            {t('live_demo.title')}
          </h2>
          <p className="text-xl text-gray-400 leading-relaxed">
            {t('live_demo.desc')}
          </p>
        </div>

        <div className="rounded-3xl border border-white/10 bg-brand-bg/80 backdrop-blur-xl p-8 md:p-12 shadow-2xl relative max-w-2xl mx-auto">
          {/* Glassmorphism terminal mock */}
          <div className="flex items-center gap-2 mb-6 border-b border-white/10 pb-4 justify-between">
            <div className="flex items-center gap-1.5">
              <span className="w-3 h-3 rounded-full bg-brand-rose/60" />
              <span className="w-3 h-3 rounded-full bg-amber-500/60" />
              <span className="w-3 h-3 rounded-full bg-emerald-500/60" />
            </div>
            <span className="text-xs font-mono text-gray-500">autonomous_cycle.log</span>
          </div>

          <div className="min-h-[220px] flex flex-col justify-center items-center">
            <AnimatePresence mode="wait">
              {!isPlaying && step === 0 ? (
                <motion.div
                  key="start-screen"
                  initial={{ opacity: 0, scale: 0.95 }}
                  animate={{ opacity: 1, scale: 1 }}
                  exit={{ opacity: 0, scale: 0.95 }}
                  className="flex flex-col items-center"
                >
                  <button
                    onClick={handleStart}
                    className="p-6 bg-brand-cyan hover:bg-brand-cyan-hover text-black rounded-full shadow-lg shadow-brand-cyan/20 hover:shadow-brand-cyan/40 transition-all duration-300 transform hover:scale-105"
                    aria-label={t('live_demo.start')}
                  >
                    <Play size={32} fill="currentColor" />
                  </button>
                  <span className="text-gray-400 font-mono text-sm mt-4">{t('live_demo.start')}</span>
                </motion.div>
              ) : (
                <motion.div
                  key={`step-${step}`}
                  initial={{ opacity: 0, y: 10 }}
                  animate={{ opacity: 1, y: 0 }}
                  exit={{ opacity: 0, y: -10 }}
                  transition={{ duration: 0.4 }}
                  className="flex flex-col items-center"
                >
                  <div className={`p-6 bg-white/5 rounded-2xl border border-white/10 mb-6 ${currentStep.color}`}>
                    <StepIcon size={48} />
                  </div>
                  <h3 className="text-2xl font-bold text-white mb-2 font-display">{currentStep.label}</h3>
                  <div className="text-xs font-mono text-gray-500 flex items-center gap-2">
                    <span className="w-2 h-2 rounded-full bg-brand-cyan animate-ping" aria-hidden="true" />
                    <span>CYCLE STATE: STEP {step + 1} OF {steps.length}</span>
                  </div>
                </motion.div>
              )}
            </AnimatePresence>
          </div>

          {/* Timeline visualization */}
          <div className="mt-12 flex justify-between gap-2 max-w-md mx-auto relative">
            <div className="absolute top-1/2 left-0 right-0 h-0.5 bg-white/10 -translate-y-1/2 z-0" />
            {steps.map((s, i) => (
              <button
                key={i}
                type="button"
                onClick={() => isPlaying && setStep(i)}
                aria-label={`Step ${i + 1}: ${s.label}`}
                className={`relative z-10 w-4 h-4 rounded-full border-2 transition-all duration-300 cursor-pointer ${
                  i === step
                    ? 'bg-brand-cyan border-brand-cyan scale-125 shadow-lg shadow-brand-cyan/50'
                    : i < step
                    ? 'bg-emerald-500 border-emerald-500'
                    : 'bg-brand-bg border-white/20 hover:border-white/40'
                }`}
                style={{ contentVisibility: 'auto' }}
              />
            ))}
          </div>

          <div className="mt-8 flex justify-between items-center text-xs font-mono text-gray-500 border-t border-white/10 pt-4">
            <span>TIME ELAPSED: {step * 7.5}s / 60.0s</span>
            {isPlaying && (
              <button
                onClick={() => { setIsPlaying(false); setStep(0); }}
                className="flex items-center gap-1.5 hover:text-white transition-colors"
              >
                <RotateCcw size={14} />
                RESET
              </button>
            )}
          </div>
        </div>
      </div>
    </section>
  );
}

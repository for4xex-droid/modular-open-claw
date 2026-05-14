/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

import { useState } from 'react';
import { motion } from 'framer-motion';
import { Briefcase, Link as LinkIcon, CheckCircle, ChevronRight, AlertCircle, Loader2 } from 'lucide-react';
import { authenticatedFetch } from '../lib/auth';
import { API_BASE } from '../config';
import { useTranslation } from '../i18n';

interface DiscoveryData {
  clientName: string;
  industry: string;
  targetTasks: string[];
  estimatedHoursSaved: number;
  setupFee: number;
  monthlyFee: number;
}

export const AiaaOnboardingWizard = () => {
  const { t } = useTranslation();
  const [step, setStep] = useState(1);
  const [data, setData] = useState<DiscoveryData>({
    clientName: '',
    industry: '',
    targetTasks: [],
    estimatedHoursSaved: 10,
    setupFee: 3000,
    monthlyFee: 500
  });

  const [stripeLink, setStripeLink] = useState('');
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const generateBlueprintAndLink = async () => {
    setIsLoading(true);
    setError(null);
    try {
      const res = await authenticatedFetch(`${API_BASE}/api/v1/commerce/checkout-session/create`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json'
        },
        body: JSON.stringify({
          agent_id: '00000000-0000-0000-0000-000000000000', // Dummy agent ID for onboarding
          price_id: 'price_dummy',
          success_url: `${window.location.origin}/checkout/success`,
          cancel_url: `${window.location.origin}/checkout/cancel`
        })
      });

      if (!res.ok) {
        throw new Error('Failed to create checkout session');
      }

      const responseData = await res.json();
      setStripeLink(responseData.url);
      setStep(3);
    } catch (err: any) {
      console.error('Error generating checkout link:', err);
      setError((t('aiaa.checkoutError') || 'Error generating checkout link') + ': ' + err.message);
    } finally {
      setIsLoading(false);
    }
  };

  const toggleTask = (task: string) => {
    setData(prev => ({
      ...prev,
      targetTasks: prev.targetTasks.includes(task)
        ? prev.targetTasks.filter(item => item !== task)
        : [...prev.targetTasks, task]
    }));
  };

  return (
    <div className="wizard-container" style={{ padding: 'var(--space-2xl)', maxWidth: '800px', margin: '0 auto' }}>
      <h1 style={{ display: 'flex', alignItems: 'center', gap: '0.5rem', marginBottom: '2rem' }}>
        <Briefcase color="var(--accent-cyan)" />
        {t('aiaa.title') || 'B2B Client Onboarding'}
      </h1>

      <div className="wizard-steps" style={{ display: 'flex', gap: '1rem', marginBottom: '3rem' }}>
        {[1, 2, 3].map(i => (
          <div
            key={i}
            style={{
              flex: 1,
              height: '4px',
              backgroundColor: step >= i ? 'var(--accent-cyan)' : 'var(--bg-tertiary)',
              borderRadius: '2px',
              transition: 'all 0.3s'
            }}
          />
        ))}
      </div>

      <AnimatePresence mode="wait">
        {step === 1 && (
          <motion.div
            key="step1"
            initial={{ opacity: 0, x: 20 }}
            animate={{ opacity: 1, x: 0 }}
            exit={{ opacity: 0, x: -20 }}
            className="form-group"
          >
            <h2>{t('aiaa.step1') || '1. Discovery Session'}</h2>
            <p style={{ color: 'var(--text-secondary)', marginBottom: '2rem' }}>
              {t('aiaa.step1Desc') || "Define the client's business and the automations they need."}
            </p>

            <div style={{ display: 'flex', flexDirection: 'column', gap: '1rem' }}>
              <input
                type="text"
                placeholder={t('aiaa.clientName') || 'Client / Company Name'}
                value={data.clientName}
                onChange={e => setData({ ...data, clientName: e.target.value })}
                className="aiome-input"
              />
              <input
                type="text"
                placeholder={t('aiaa.industry') || 'Industry (e.g. Real Estate, E-commerce)'}
                value={data.industry}
                onChange={e => setData({ ...data, industry: e.target.value })}
                className="aiome-input"
              />

              <h4>{t('aiaa.targetTasks') || 'Target Automations'}</h4>
              <div style={{ display: 'flex', gap: '0.5rem', flexWrap: 'wrap' }}>
                {([
                  { key: 'inboxTriage', fallback: 'Inbox Triage' },
                  { key: 'crmSync', fallback: 'CRM Sync' },
                  { key: 'invoiceExtraction', fallback: 'Invoice Extraction' },
                  { key: 'socialMediaPosting', fallback: 'Social Media Posting' }
                ] as const).map(task => {
                  const label = t(`aiaa.task.${task.key}`) || task.fallback;
                  return (
                  <button
                    key={task.key}
                    onClick={() => toggleTask(task.key)}
                    className={`chip ${data.targetTasks.includes(task.key) ? 'active' : ''}`}
                    style={{
                      border: data.targetTasks.includes(task.key) ? '1px solid var(--accent-cyan)' : '1px solid var(--border)',
                      background: data.targetTasks.includes(task.key) ? 'rgba(0,255,255,0.1)' : 'var(--bg-secondary)'
                    }}
                  >
                    {label}
                  </button>
                  );
                })}
              </div>

              <button
                className="aiome-btn-primary"
                onClick={() => setStep(2)}
                disabled={!data.clientName}
                style={{ marginTop: '2rem', alignSelf: 'flex-end' }}
              >
                {t('aiaa.nextStep') || 'Next Step'} <ChevronRight size={16} />
              </button>
            </div>
          </motion.div>
        )}

        {step === 2 && (
          <motion.div
            key="step2"
            initial={{ opacity: 0, x: 20 }}
            animate={{ opacity: 1, x: 0 }}
            exit={{ opacity: 0, x: -20 }}
            className="form-group"
          >
            <h2>{t('aiaa.step2') || '2. Economics & ROI'}</h2>
            <p style={{ color: 'var(--text-secondary)', marginBottom: '2rem' }}>
              {t('aiaa.step2Desc') || 'Set up the B2B pricing model based on the estimated value provided.'}
            </p>

            <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '1rem', marginBottom: '2rem' }}>
              <div>
                <label>{t('aiaa.hoursSaved') || 'Estimated Hours Saved (per week)'}</label>
                <input
                  type="number"
                  value={data.estimatedHoursSaved}
                  onChange={e => setData({ ...data, estimatedHoursSaved: Number(e.target.value) })}
                  className="aiome-input"
                />
              </div>
              <div>
                <label>{t('aiaa.setupFee') || 'Setup Fee ($)'}</label>
                <input
                  type="number"
                  value={data.setupFee}
                  onChange={e => setData({ ...data, setupFee: Number(e.target.value) })}
                  className="aiome-input"
                />
              </div>
              <div>
                <label>{t('aiaa.monthlyFee') || 'Monthly Retainer ($)'}</label>
                <input
                  type="number"
                  value={data.monthlyFee}
                  onChange={e => setData({ ...data, monthlyFee: Number(e.target.value) })}
                  className="aiome-input"
                />
              </div>
            </div>

            <div style={{ display: 'flex', justifyContent: 'space-between', marginTop: '2rem' }}>
              <button className="aiome-btn-secondary" onClick={() => setStep(1)} disabled={isLoading}>
                {t('common.back') || 'Back'}
              </button>
              
              <div style={{ display: 'flex', alignItems: 'center', gap: '1rem' }}>
                {error && (
                  <span style={{ color: 'var(--accent-red)', display: 'flex', alignItems: 'center', gap: '0.25rem', fontSize: '0.875rem' }}>
                    <AlertCircle size={14} /> {error}
                  </span>
                )}
                <button className="aiome-btn-primary" onClick={generateBlueprintAndLink} disabled={isLoading}>
                  {isLoading ? (
                    <>
                      <Loader2 size={16} className="spinner" /> {t('aiaa.generating') || 'Generating...'}
                    </>
                  ) : (
                    t('aiaa.generateLink') || 'Generate Blueprint & Checkout Link'
                  )}
                </button>
              </div>
            </div>
          </motion.div>
        )}

        {step === 3 && (
          <motion.div
            key="step3"
            initial={{ opacity: 0, scale: 0.95 }}
            animate={{ opacity: 1, scale: 1 }}
            className="form-group"
            style={{ textAlign: 'center', padding: '3rem 0' }}
          >
            <CheckCircle color="var(--accent-green)" size={64} style={{ margin: '0 auto 1rem' }} />
            <h2>{t('aiaa.step3') || 'Blueprint Ready!'}</h2>
            <p style={{ color: 'var(--text-secondary)', marginBottom: '2rem' }}>
              {(t('aiaa.step3Desc') || 'The Automation Blueprint has been created and securely stored. Send this link to {{client}} to begin their subscription.').replace('{{client}}', data.clientName)}
            </p>

            <div
              style={{
                background: 'var(--bg-secondary)',
                padding: '1rem',
                borderRadius: '8px',
                display: 'flex',
                alignItems: 'center',
                gap: '1rem',
                marginBottom: '2rem',
                border: '1px solid var(--border)'
              }}
            >
              <LinkIcon size={20} color="var(--text-secondary)" />
              <code style={{ flex: 1, textAlign: 'left', wordBreak: 'break-all' }}>{stripeLink}</code>
              <button
                className="aiome-btn-secondary"
                onClick={() => navigator.clipboard.writeText(stripeLink)}
              >
                {t('common.copy') || 'Copy'}
              </button>
            </div>
            
            <button className="aiome-btn-primary" onClick={() => setStep(1)}>
              {t('aiaa.createAnother') || 'Create Another'}
            </button>
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
};

// Add AnimatePresence if not imported natively from framer-motion in App
import { AnimatePresence as FramerAnimatePresence } from 'framer-motion';
const AnimatePresence = FramerAnimatePresence;

/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

import { useState } from 'react';
import { motion } from 'framer-motion';
import { Briefcase, Link as LinkIcon, CheckCircle, ChevronRight } from 'lucide-react';
// Remove useTranslation because it was unused

interface DiscoveryData {
  clientName: string;
  industry: string;
  targetTasks: string[];
  estimatedHoursSaved: number;
  setupFee: number;
  monthlyFee: number;
}

export const AiaaOnboardingWizard = () => {
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

  const generateBlueprintAndLink = () => {
    // Mocking the backend call to generate a Stripe Checkout link & Blueprint
    setTimeout(() => {
      setStripeLink(`https://checkout.stripe.com/pay/cs_test_${Math.random().toString(36).substr(2, 9)}`);
      setStep(3);
    }, 1500);
  };

  const toggleTask = (task: string) => {
    setData(prev => ({
      ...prev,
      targetTasks: prev.targetTasks.includes(task)
        ? prev.targetTasks.filter(t => t !== task)
        : [...prev.targetTasks, task]
    }));
  };

  return (
    <div className="wizard-container" style={{ padding: 'var(--space-2xl)', maxWidth: '800px', margin: '0 auto' }}>
      <h1 style={{ display: 'flex', alignItems: 'center', gap: '0.5rem', marginBottom: '2rem' }}>
        <Briefcase color="var(--accent-cyan)" />
        B2B Client Onboarding
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
            <h2>1. Discovery Session</h2>
            <p style={{ color: 'var(--text-secondary)', marginBottom: '2rem' }}>
              Define the client's business and the automations they need.
            </p>

            <div style={{ display: 'flex', flexDirection: 'column', gap: '1rem' }}>
              <input
                type="text"
                placeholder="Client / Company Name"
                value={data.clientName}
                onChange={e => setData({ ...data, clientName: e.target.value })}
                className="aiome-input"
              />
              <input
                type="text"
                placeholder="Industry (e.g. Real Estate, E-commerce)"
                value={data.industry}
                onChange={e => setData({ ...data, industry: e.target.value })}
                className="aiome-input"
              />

              <h4>Target Automations</h4>
              <div style={{ display: 'flex', gap: '0.5rem', flexWrap: 'wrap' }}>
                {['Inbox Triage', 'CRM Sync', 'Invoice Extraction', 'Social Media Posting'].map(task => (
                  <button
                    key={task}
                    onClick={() => toggleTask(task)}
                    className={`chip ${data.targetTasks.includes(task) ? 'active' : ''}`}
                    style={{
                      border: data.targetTasks.includes(task) ? '1px solid var(--accent-cyan)' : '1px solid var(--border)',
                      background: data.targetTasks.includes(task) ? 'rgba(0,255,255,0.1)' : 'var(--bg-secondary)'
                    }}
                  >
                    {task}
                  </button>
                ))}
              </div>

              <button
                className="aiome-btn-primary"
                onClick={() => setStep(2)}
                disabled={!data.clientName}
                style={{ marginTop: '2rem', alignSelf: 'flex-end' }}
              >
                Next Step <ChevronRight size={16} />
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
            <h2>2. Economics & ROI</h2>
            <p style={{ color: 'var(--text-secondary)', marginBottom: '2rem' }}>
              Set up the B2B pricing model based on the estimated value provided.
            </p>

            <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '1rem', marginBottom: '2rem' }}>
              <div>
                <label>Estimated Hours Saved (per week)</label>
                <input
                  type="number"
                  value={data.estimatedHoursSaved}
                  onChange={e => setData({ ...data, estimatedHoursSaved: Number(e.target.value) })}
                  className="aiome-input"
                />
              </div>
              <div>
                <label>Setup Fee ($)</label>
                <input
                  type="number"
                  value={data.setupFee}
                  onChange={e => setData({ ...data, setupFee: Number(e.target.value) })}
                  className="aiome-input"
                />
              </div>
              <div>
                <label>Monthly Retainer ($)</label>
                <input
                  type="number"
                  value={data.monthlyFee}
                  onChange={e => setData({ ...data, monthlyFee: Number(e.target.value) })}
                  className="aiome-input"
                />
              </div>
            </div>

            <div style={{ display: 'flex', justifyContent: 'space-between', marginTop: '2rem' }}>
              <button className="aiome-btn-secondary" onClick={() => setStep(1)}>
                Back
              </button>
              <button className="aiome-btn-primary" onClick={generateBlueprintAndLink}>
                Generate Blueprint & Checkout Link
              </button>
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
            <h2>Blueprint Ready!</h2>
            <p style={{ color: 'var(--text-secondary)', marginBottom: '2rem' }}>
              The Automation Blueprint has been created and securely stored. Send this link to {data.clientName} to begin their subscription.
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
                Copy
              </button>
            </div>
            
            <button className="aiome-btn-primary" onClick={() => setStep(1)}>
              Create Another
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

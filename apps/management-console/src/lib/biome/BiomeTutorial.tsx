/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import { useState, useEffect, useRef, type CSSProperties } from 'react';

export interface BiomeTutorialProps {
  onClose: () => void;
}

interface Step {
  title: string;
  desc: string;
  targetSelector: string; // DOM selector to highlight
}

export function BiomeTutorial({ onClose }: BiomeTutorialProps) {
  const [currentStep, setCurrentStep] = useState(0);
  const [highlightStyle, setHighlightStyle] = useState<CSSProperties>({});
  const [tooltipStyle, setTooltipStyle] = useState<CSSProperties>({});
  const tooltipRef = useRef<HTMLDivElement>(null);

  const steps: Step[] = [
    {
      title: '🧬 Lenia 生命場を観察する',
      desc: '128×128 の連続場上で、Orbium 系の生命パターンが自律的に動きます。世代が進むほど安定性・移動・対称性が評価されます。',
      targetSelector: 'canvas',
    },
    {
      title: '🌱 種まきで新しい種を誕生させる',
      desc: '右パネルの「種まき」を ON にして、キャンバスをタッチするとその地点に生命の種を撒けます。複数箇所に撒くと相互作用が起きます。',
      targetSelector: '[data-testid="control-seed-mode"]',
    },
    {
      title: '🎛 μ・σ で成長を調整する',
      desc: 'μ（成長中心）と σ（成長幅）スライダーで Lenia パラメータをリアルタイム変更できます。安定域外にするとパターンは崩壊します。',
      targetSelector: '[data-testid="control-lenia-mu"]',
    },
    {
      title: '📖 種図鑑と Legendary を目指す',
      desc: '200 世代で評価が確定します。質量・存続・移動・対称性が高いほどレア度が上がります。良い種は図鑑に保存しましょう！',
      targetSelector: '[data-testid="biome-rarity"]',
    },
  ];

  const handleNext = () => {
    if (currentStep < steps.length - 1) {
      setCurrentStep(currentStep + 1);
    } else {
      localStorage.setItem('biome_tutorial_done', 'true');
      onClose();
    }
  };

  const handlePrev = () => {
    if (currentStep > 0) {
      setCurrentStep(currentStep - 1);
    }
  };

  const handleSkip = () => {
    localStorage.setItem('biome_tutorial_done', 'true');
    onClose();
  };

  useEffect(() => {
    const updatePosition = () => {
      const step = steps[currentStep];
      const element = document.querySelector(step.targetSelector);
      if (!element) {
        // Fallback if target element is not found
        setHighlightStyle({ display: 'none' });
        setTooltipStyle({
          position: 'fixed',
          top: '50%',
          left: '50%',
          transform: 'translate(-50%, -50%)',
          zIndex: 1001
        });
        return;
      }

      const rect = element.getBoundingClientRect();
      const pad = 8;

      setHighlightStyle({
        position: 'fixed',
        top: rect.top - pad,
        left: rect.left - pad,
        width: rect.width + pad * 2,
        height: rect.height + pad * 2,
        border: '3px solid var(--accent-cyan)',
        boxShadow: '0 0 15px var(--accent-cyan), 0 0 0 9999px var(--black-70)',
        borderRadius: 'var(--radius-sm)',
        zIndex: 1001,
        transition: 'all 0.3s ease',
        pointerEvents: 'none'
      });

      // Calculate tooltip position (always show below/above the highlighted element)
      const tooltipHeight = tooltipRef.current?.offsetHeight || 180;
      const tooltipWidth = Math.min(460, tooltipRef.current?.offsetWidth || 380);
      
      let top = rect.bottom + pad + 12;
      let left = rect.left + (rect.width - tooltipWidth) / 2;

      // Adjust if tooltip goes off-screen vertically
      if (top + tooltipHeight > window.innerHeight) {
        top = rect.top - tooltipHeight - pad - 12;
      }
      // Viewport safety margins
      top = Math.max(16, Math.min(window.innerHeight - tooltipHeight - 16, top));
      // Adjust horizontally
      left = Math.max(16, Math.min(window.innerWidth - tooltipWidth - 16, left));

      setTooltipStyle({
        position: 'fixed',
        top,
        left,
        width: `${tooltipWidth}px`,
        maxWidth: '460px',
        zIndex: 1001,
        transition: 'all 0.3s ease'
      });
    };

    // Delay slightly to ensure layout has stabilized
    const timer = setTimeout(updatePosition, 100);
    window.addEventListener('resize', updatePosition);
    return () => {
      clearTimeout(timer);
      window.removeEventListener('resize', updatePosition);
    };
  }, [currentStep]);

  const step = steps[currentStep];

  return (
    <div style={{ position: 'fixed', top: 0, left: 0, width: '100vw', height: '100vh', zIndex: 999, pointerEvents: 'none' }}>
      {/* Spotlight highlight circle */}
      <div style={highlightStyle} />

      {/* Tooltip box */}
      <div 
        ref={tooltipRef}
        style={{
          ...tooltipStyle,
          pointerEvents: 'auto',
          background: 'var(--bg-deep-glass)',
          border: '1px solid var(--border-glass-bright)',
          borderRadius: 'var(--radius-md)',
          padding: 'var(--space-sm)',
          boxShadow: '0 8px 32px var(--black-85), 0 0 20px var(--accent-cyan-10)',
          backdropFilter: 'blur(12px)',
          color: 'var(--white-100)',
          fontFamily: 'var(--font-main)'
        }}
      >
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '8px' }}>
          <span style={{ fontSize: '0.75rem', color: 'var(--accent-cyan)', fontWeight: 'bold' }}>
            チュートリアル ({currentStep + 1} / {steps.length})
          </span>
          <button 
            onClick={handleSkip}
            style={{
              background: 'none',
              border: 'none',
              color: 'var(--white-40)',
              cursor: 'pointer',
              fontSize: '0.75rem',
              fontWeight: '600'
            }}
          >
            スキップ
          </button>
        </div>

        <h4 style={{ margin: '0 0 8px 0', fontSize: '1rem', fontWeight: 'bold' }}>{step.title}</h4>
        <p style={{ margin: '0 0 16px 0', fontSize: '0.8rem', lineHeight: '1.4', color: 'var(--white-80)' }}>
          {step.desc}
        </p>

        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
          <button
            onClick={handlePrev}
            disabled={currentStep === 0}
            style={{
              background: 'var(--white-05)',
              border: '1px solid var(--white-10)',
              borderRadius: '6px',
              color: currentStep === 0 ? 'var(--white-15)' : 'var(--white-90)',
              padding: '6px 12px',
              cursor: currentStep === 0 ? 'default' : 'pointer',
              fontSize: '0.75rem',
              fontWeight: '600'
            }}
          >
            戻る
          </button>
          <button
            onClick={handleNext}
            style={{
              background: 'linear-gradient(135deg, var(--accent-cyan), var(--accent-blue))',
              border: 'none',
              borderRadius: '6px',
              color: 'var(--text-inverse)',
              padding: '6px 16px',
              cursor: 'pointer',
              fontSize: '0.75rem',
              fontWeight: 'bold',
              boxShadow: '0 0 8px var(--accent-cyan-30)'
            }}
          >
            {currentStep === steps.length - 1 ? '開始！' : '次へ'}
          </button>
        </div>
      </div>
    </div>
  );
}

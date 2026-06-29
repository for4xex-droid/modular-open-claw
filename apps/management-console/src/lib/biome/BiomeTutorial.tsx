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
      title: '🧬 生命の進化を見守る',
      desc: 'Biomeシミュレーションへお越しいただきありがとうございます！ここでは128x128のグリッド上で生命体が自律進化していきます。進化の成り行きを観察しましょう。',
      targetSelector: 'canvas' // Highlight canvas
    },
    {
      title: '⏸ サイクル速度のコントロール',
      desc: 'シミュレーションの進行を「停止」したり、1x、2x、5x、10x の速度に切り替えて、生命の誕生と滅亡を高速にシミュレートできます。',
      targetSelector: '[data-testid="cycle-pause"]'
    },
    {
      title: '🌱 生命の種（元素）を注入する',
      desc: 'C(炭素)・N(窒素)・O(酸素)・Fe(鉄) など8種類の元素を選択して、画面上の好きな場所をタッチ/ドラッグすると、その周辺に元素エネルギーを直接注入して新しい生命を誕生させられます！元素の組み合わせで化学反応が起き、進化が加速します。',
      targetSelector: '[data-testid="inject-c"]'
    },
    {
      title: '⚗️ 元素反応の連鎖',
      desc: '注入した元素同士が近接セル内で混ざり合うと、元素反応が起こります（例：C+H→N、N+P→C+H、Fe+O→Siなど）。反応は質量を保存し、より上位の変異に必要な複雑な元素を生み出します。',
      targetSelector: 'canvas'
    },
    {
      title: '☄️ 環境災害を引き起こす',
      desc: '隕石落下(Meteor)や氷河期(IceAge)などの災害を選択して画面をタッチすると、その地点を中心に環境変化を引き起こし、進化の方向性を強制変異させることができます。',
      targetSelector: '[data-testid="crisis-meteor"]'
    },
    {
      title: '🔥 伝説の生命体 (Legendary) を目指す',
      desc: 'シミュレーションが 200世代 に達すると、生命の多様性から最終的な評価ランク(Rarity)が決定されます。様々な変異や災害を試し、最高評価を目指しましょう！',
      targetSelector: '[data-testid="biome-rarity"]'
    }
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
        boxShadow: '0 0 15px var(--accent-cyan), 0 0 0 9999px var(--black-75, rgba(0, 0, 0, 0.75))',
        borderRadius: 'var(--radius-sm)',
        zIndex: 1001,
        transition: 'all 0.3s ease',
        pointerEvents: 'none'
      });

      // Calculate tooltip position (always show below/above the highlighted element)
      const tooltipHeight = tooltipRef.current?.offsetHeight || 180;
      const tooltipWidth = tooltipRef.current?.offsetWidth || 340;
      
      let top = rect.bottom + pad + 12;
      let left = rect.left + (rect.width - tooltipWidth) / 2;

      // Adjust if tooltip goes off-screen vertically
      if (top + tooltipHeight > window.innerHeight) {
        top = rect.top - tooltipHeight - pad - 12;
      }
      // Adjust horizontally
      left = Math.max(16, Math.min(window.innerWidth - tooltipWidth - 16, left));

      setTooltipStyle({
        position: 'fixed',
        top,
        left,
        width: `${tooltipWidth}px`,
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
          boxShadow: '0 8px 32px var(--black-80, rgba(0, 0, 0, 0.8)), 0 0 20px var(--accent-cyan-10)',
          backdropFilter: 'blur(12px)',
          color: 'var(--white-100)',
          fontFamily: 'var(--font-main, sans-serif)'
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
              color: 'var(--bg-primary, #0c0f1d)',
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

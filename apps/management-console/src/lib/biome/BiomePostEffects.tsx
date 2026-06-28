/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import { EffectComposer, Bloom } from '@react-three/postprocessing';
import { TachyonEffect } from './effects/TachyonEffect';
import { HiggsEffect } from './effects/HiggsEffect';
import { useEffect, useMemo } from 'react';

interface BiomePostEffectsProps {
  effectType?: 'none' | 'higgs' | 'tachyon';
  effectIntensity?: number;
  effectCenter?: [number, number];
  bloomEnabled?: boolean;
}

export function BiomePostEffects({
  effectType = 'none',
  effectIntensity = 0.5,
  effectCenter = [0.5, 0.5],
  bloomEnabled = true,
}: BiomePostEffectsProps) {
  // カスタムエフェクトを useMemo で作成
  const tachyonEffect = useMemo(() => new TachyonEffect({ damp: 0.85 }), []);
  const higgsEffect = useMemo(() => new HiggsEffect({ intensity: effectIntensity, center: effectCenter }), []);

  // uniforms を props に合わせて更新
  useEffect(() => {
    // intensity に応じて damp (残像の減衰係数) を調整
    const dampValue = 0.95 - (effectIntensity * 0.45);
    tachyonEffect.damp = dampValue;
  }, [effectIntensity, tachyonEffect]);

  useEffect(() => {
    higgsEffect.intensity = effectIntensity;
    higgsEffect.center = effectCenter;
  }, [effectIntensity, effectCenter, higgsEffect]);

  // 使用しなくなったエフェクトは明示的に dispose
  useEffect(() => {
    return () => {
      tachyonEffect.dispose();
      higgsEffect.dispose();
    };
  }, [tachyonEffect, higgsEffect]);

  const passes: any[] = [];
  if (bloomEnabled) {
    passes.push(
      <Bloom 
        key="bloom"
        intensity={1.2} 
        luminanceThreshold={0.15} 
        luminanceSmoothing={0.9} 
        mipmapBlur
      />
    );
  }
  if (effectType === 'tachyon') {
    passes.push(<primitive key="tachyon" object={tachyonEffect} />);
  } else if (effectType === 'higgs') {
    passes.push(<primitive key="higgs" object={higgsEffect} />);
  }

  return (
    <EffectComposer>
      {passes}
    </EffectComposer>
  );
}

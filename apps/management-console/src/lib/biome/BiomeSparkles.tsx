/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import { Sparkles } from '@react-three/drei';

interface BiomeSparklesProps {
  rarity?: number;
}

export function BiomeSparkles({ rarity = 0 }: BiomeSparklesProps) {
  if (rarity < 4) return null; // Legendaryのみ

  return (
    <Sparkles
      count={40}
      scale={[128, 128, 5]}
      size={6}
      speed={0.4}
      opacity={0.3}
      color="#ffe080" // 金色調のパーティクル
    />
  );
}

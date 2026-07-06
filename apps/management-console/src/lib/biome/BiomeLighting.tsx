/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import { cssVar } from '../../utils/cssVar';

interface BiomeLightingProps {
  rarity?: number;
}

export function BiomeLighting({ rarity = 0 }: BiomeLightingProps) {
  // レアリティが高くなると、少し光を強く/色味を変える
  const ambientIntensity = 0.4 + rarity * 0.05;
  const directionalIntensity = 0.8 + rarity * 0.1;
  const lightColor = rarity >= 4 ? cssVar('--fluid-warm-ivory') : cssVar('--white-100');

  return (
    <group>
      <ambientLight intensity={ambientIntensity} />
      <directionalLight 
        position={[40, 80, 100]} 
        intensity={directionalIntensity} 
        color={lightColor}
      />
    </group>
  );
}

/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import { useMemo } from 'react';
import { cssVar } from '../../utils/cssVar';
import { parseColorToRGB } from '../../utils/colorUtils';

export function useFluidConfig() {
  return useMemo(() => {
    const prefersReducedMotion =
      typeof window !== 'undefined' &&
      window.matchMedia('(prefers-reduced-motion: reduce)').matches;
    const isMobile =
      typeof window !== 'undefined' &&
      window.matchMedia('(max-width: 768px)').matches;

    return {
      enabled: !prefersReducedMotion,
      simResolution: isMobile ? 64 : 128,
      dyeResolution: isMobile ? 256 : 512,
      intensity: 0.3,
      colors: {
        primary: cssVar('--fluid-warm-ivory', '#d4c5a9'),
        secondary: cssVar('--fluid-deep-gold', '#b8965a'),
        primaryRGB: parseColorToRGB(cssVar('--fluid-warm-ivory', '#d4c5a9')),
        secondaryRGB: parseColorToRGB(cssVar('--fluid-deep-gold', '#b8965a')),
      },
      maxDpr: isMobile ? 1.0 : 1.5,
    };
  }, []);
}

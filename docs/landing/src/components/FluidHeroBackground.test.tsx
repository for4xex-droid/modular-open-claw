import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen } from '@testing-library/react';
import { FluidHeroBackground } from './FluidHeroBackground';
import { useFluidConfig } from '../hooks/useFluidConfig';

// Mock useFluidConfig
vi.mock('../hooks/useFluidConfig', () => ({
  useFluidConfig: vi.fn(),
}));

// eslint-disable-next-line @typescript-eslint/no-explicit-any
const mockUseFluidConfig = useFluidConfig as any;

const enabledConfig = {
  enabled: true,
  simResolution: 128,
  dyeResolution: 512,
  intensity: 0.3,
  colors: {
    primary: '#d4c5a9',
    secondary: '#b8965a',
    primaryRGB: [0.8, 0.7, 0.6],
    secondaryRGB: [0.7, 0.6, 0.5],
  },
  maxDpr: 1.5,
};

const disabledConfig = { ...enabledConfig, enabled: false };

describe('FluidHeroBackground Component', () => {
  afterEach(() => {
    vi.clearAllMocks();
  });

  it('renders a canvas element with aria-hidden="true" and mix-blend-mode: screen when enabled', () => {
    mockUseFluidConfig.mockReturnValue(enabledConfig);
    const { container } = render(<FluidHeroBackground />);
    
    // Canvas should exist in the rendered output (now a sibling, not inside fluid-container)
    const canvas = container.querySelector('canvas');
    expect(canvas).toBeInTheDocument();
    expect(canvas).toHaveAttribute('aria-hidden', 'true');
    // Verify overlay blend mode
    expect(canvas!.style.mixBlendMode).toBe('screen');
    expect(canvas!.style.pointerEvents).toBe('none');
  });

  it('renders CSS fallback when disabled (e.g. prefers-reduced-motion) without canvas', () => {
    mockUseFluidConfig.mockReturnValue(disabledConfig);
    render(<FluidHeroBackground />);
    
    const container = screen.getByTestId('fluid-container');
    expect(container).toBeInTheDocument();
    
    // Canvas should NOT be rendered when disabled
    const canvas = container.ownerDocument.querySelector('canvas');
    expect(canvas).toBeNull();

    // Fallback gradient overlay should still be visible
    const fallback = screen.getByTestId('fluid-fallback');
    expect(fallback).toBeInTheDocument();
  });

  it('renders a pointer capture overlay layer when enabled', () => {
    mockUseFluidConfig.mockReturnValue(enabledConfig);
    const { container } = render(<FluidHeroBackground />);
    
    // There should be a transparent pointer capture div (non-canvas, with pointer-events: auto)
    const pointerOverlays = container.querySelectorAll('div[aria-hidden="true"]');
    const pointerLayer = Array.from(pointerOverlays).find(
      el => (el as HTMLElement).style.pointerEvents === 'auto'
    );
    expect(pointerLayer).toBeTruthy();
  });
});

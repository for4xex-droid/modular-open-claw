import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { renderHook } from '@testing-library/react';
import { useFluidConfig } from './useFluidConfig';

describe('useFluidConfig', () => {
  const originalMatchMedia = window.matchMedia;

  beforeEach(() => {
    // Mock getComputedStyle to return simulated CSS variable values
    vi.spyOn(window, 'getComputedStyle').mockImplementation(() => {
      return {
        getPropertyValue: (prop: string) => {
          if (prop === '--color-fluid-warm-ivory') return '#d4c5a9';
          if (prop === '--color-fluid-deep-gold') return '#b8965a';
          return '';
        }
        // eslint-disable-next-line @typescript-eslint/no-explicit-any
      } as any;
    });
  });

  afterEach(() => {
    window.matchMedia = originalMatchMedia;
    vi.restoreAllMocks();
  });

  it('should return default config for desktop viewports', () => {
    window.matchMedia = vi.fn().mockImplementation((query) => ({
      matches: false,
      media: query,
      onchange: null,
      addListener: vi.fn(),
      removeListener: vi.fn(),
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
      dispatchEvent: vi.fn(),
    }));

    const { result } = renderHook(() => useFluidConfig());

    expect(result.current.enabled).toBe(true);
    expect(result.current.simResolution).toBe(128);
    expect(result.current.dyeResolution).toBe(512);
    expect(result.current.maxDpr).toBe(1.5);
  });

  it('should disable fluid effect when prefers-reduced-motion is true', () => {
    window.matchMedia = vi.fn().mockImplementation((query) => ({
      matches: query.includes('prefers-reduced-motion'),
      media: query,
      onchange: null,
      addListener: vi.fn(),
      removeListener: vi.fn(),
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
      dispatchEvent: vi.fn(),
    }));

    const { result } = renderHook(() => useFluidConfig());

    expect(result.current.enabled).toBe(false);
  });

  it('should scale down resolution on mobile viewports', () => {
    window.matchMedia = vi.fn().mockImplementation((query) => ({
      matches: query.includes('max-width: 768px'),
      media: query,
      onchange: null,
      addListener: vi.fn(),
      removeListener: vi.fn(),
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
      dispatchEvent: vi.fn(),
    }));

    const { result } = renderHook(() => useFluidConfig());

    expect(result.current.enabled).toBe(true);
    expect(result.current.simResolution).toBe(64);
    expect(result.current.dyeResolution).toBe(256);
    expect(result.current.maxDpr).toBe(1.0);
  });
});

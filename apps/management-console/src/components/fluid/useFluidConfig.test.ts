import { renderHook } from '@testing-library/react';
import { useFluidConfig } from './useFluidConfig';

// Mock window.matchMedia for jsdom environment
Object.defineProperty(window, 'matchMedia', {
  writable: true,
  value: jest.fn().mockImplementation(query => ({
    matches: false,
    media: query,
    onchange: null,
    addListener: jest.fn(),
    removeListener: jest.fn(),
    addEventListener: jest.fn(),
    removeEventListener: jest.fn(),
    dispatchEvent: jest.fn(),
  })),
});

// Mock cssVar to return fake CSS token values
jest.mock('../../utils/cssVar', () => ({
  cssVar: jest.fn((name: string, fallback: string) => fallback)
}));

describe('useFluidConfig', () => {
  it('should return valid config object', () => {
    const { result } = renderHook(() => useFluidConfig());
    
    expect(result.current).toHaveProperty('enabled');
    expect(result.current).toHaveProperty('simResolution');
    expect(result.current).toHaveProperty('dyeResolution');
    expect(result.current.colors).toEqual({
      primary: '#d4c5a9',
      secondary: '#b8965a',
      primaryRGB: [212 / 255, 197 / 255, 169 / 255],
      secondaryRGB: [184 / 255, 150 / 255, 90 / 255]
    });
  });
});

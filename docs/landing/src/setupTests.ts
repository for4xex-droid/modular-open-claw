import '@testing-library/jest-dom';
import { vi } from 'vitest';

// Mock IntersectionObserver for Framer Motion
class IntersectionObserverMock {
  observe = vi.fn();
  disconnect = vi.fn();
  unobserve = vi.fn();
}

Object.defineProperty(window, 'IntersectionObserver', {
  writable: true,
  configurable: true,
  value: IntersectionObserverMock,
});

Object.defineProperty(globalThis, 'IntersectionObserver', {
  writable: true,
  configurable: true,
  value: IntersectionObserverMock,
});

// Mock CSS custom properties for tests to avoid console warnings
if (typeof document !== 'undefined') {
  document.documentElement.style.setProperty('--color-fluid-warm-ivory', '#d4c5a9');
  document.documentElement.style.setProperty('--color-fluid-deep-gold', '#b8965a');
}


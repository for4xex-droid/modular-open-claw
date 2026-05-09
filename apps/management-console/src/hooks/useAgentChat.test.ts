import { renderHook, act } from '@testing-library/react';
import { useAgentChat } from './useAgentChat';

// Mock authenticatedFetch
jest.mock('../lib/auth', () => ({
  authenticatedFetch: jest.fn()
}));

jest.mock('../config', () => ({
  API_BASE: 'http://localhost'
}));

describe('useAgentChat JS Bridge Feedback', () => {
  it('listens for aiome_inject_prompt CustomEvent and updates input', () => {
    const { result } = renderHook(() => useAgentChat());
    
    act(() => {
      window.dispatchEvent(new CustomEvent('aiome_inject_prompt', {
        detail: { prompt: 'Test injected prompt' }
      }));
    });
    
    expect(result.current.input).toBe('Test injected prompt');
  });

  it('triggers sendMessage automatically when aiome_inject_prompt event is fired with autoSend', () => {
    const { result } = renderHook(() => useAgentChat());
    
    act(() => {
      window.dispatchEvent(new CustomEvent('aiome_inject_prompt', {
        detail: { prompt: 'Auto trigger prompt', autoSend: true }
      }));
    });
    
    expect(result.current.history.length).toBeGreaterThan(0);
    expect(result.current.history[0].role).toBe('user');
    expect(result.current.history[0].content).toBe('Auto trigger prompt');
    expect(result.current.status).toBe('THINKING');
  });
});

import { renderHook, act, waitFor } from '@testing-library/react';
import { useAgentChat } from './useAgentChat';

// Mock authenticatedFetch
jest.mock('../lib/auth', () => ({
  authenticatedFetch: jest.fn().mockImplementation(() => Promise.resolve({
    ok: true,
    json: () => Promise.resolve([])
  }))
}));

jest.mock('../config', () => ({
  API_BASE: 'http://localhost'
}));

jest.mock('./useSystemVitality', () => ({
  useSystemVitality: () => ({ lastEvent: null })
}));

describe('useAgentChat JS Bridge Feedback', () => {
  it('listens for aiome_inject_prompt CustomEvent and updates input', async () => {
    const { result } = renderHook(() => useAgentChat());
    
    act(() => {
      window.dispatchEvent(new CustomEvent('aiome_inject_prompt', {
        detail: { prompt: 'Test injected prompt' }
      }));
    });
    
    expect(result.current.input).toBe('Test injected prompt');
    
    // Wait for initial loadHistory to complete
    await waitFor(() => {
      // @ts-expect-error - NodeJS require is not typed in Vite strict mode
      const authMock = require('../lib/auth').authenticatedFetch;
      expect(authMock).toHaveBeenCalled();
    });
  });

  it('triggers sendMessage automatically when aiome_inject_prompt event is fired with autoSend', async () => {
    const { result } = renderHook(() => useAgentChat());
    
    act(() => {
      window.dispatchEvent(new CustomEvent('aiome_inject_prompt', {
        detail: { prompt: 'Auto trigger prompt', autoSend: true }
      }));
    });
    
    expect(result.current.history.length).toBeGreaterThan(0);
    expect(result.current.history[0].role).toBe('user');
    expect(result.current.history[0].content).toBe('Auto trigger prompt');
    
    // Wait for the async sendMessage logic to finish
    await waitFor(() => {
      expect(result.current.status).toBe('IDLE');
    });
  });
});

describe('useAgentChat Slash Command Interception', () => {
  beforeEach(() => {
    jest.clearAllMocks();
  });

  it('intercepts /store command, skips fetch, and injects voiceStore envelope locally', async () => {
    const { result } = renderHook(() => useAgentChat());
    // @ts-expect-error
    const authMock = require('../lib/auth').authenticatedFetch;
    
    // Clear initial loadHistory fetch mock
    authMock.mockClear();

    await act(async () => {
      await result.current.sendMessage('/store');
    });

    // 1. Should not call backend API
    expect(authMock).not.toHaveBeenCalled();
    
    // 2. History should contain user prompt and system a2uiEnvelope
    expect(result.current.history).toHaveLength(2);
    expect(result.current.history[0].role).toBe('user');
    expect(result.current.history[0].content).toBe('/store');
    expect(result.current.history[1].role).toBe('assistant');
    expect(result.current.history[1].a2uiEnvelope).toBeDefined();
    expect(result.current.history[1].a2uiEnvelope?.type).toBe('createSurface');
    // @ts-expect-error - surface components property access check
    expect(result.current.history[1].a2uiEnvelope?.surface?.components?.[0].type).toBe('voiceStore');
  });

  it('intercepts /treasure command, skips fetch, and injects treasureItem envelope locally', async () => {
    const { result } = renderHook(() => useAgentChat());
    // @ts-expect-error
    const authMock = require('../lib/auth').authenticatedFetch;
    
    authMock.mockClear();

    await act(async () => {
      await result.current.sendMessage('/treasure');
    });

    expect(authMock).not.toHaveBeenCalled();
    // @ts-expect-error
    expect(result.current.history[1].a2uiEnvelope?.surface?.components?.[0].type).toBe('treasureItem');
  });

  it('intercepts /lora command, skips fetch, and injects loraMarket envelope locally', async () => {
    const { result } = renderHook(() => useAgentChat());
    // @ts-expect-error
    const authMock = require('../lib/auth').authenticatedFetch;
    
    authMock.mockClear();

    await act(async () => {
      await result.current.sendMessage('/lora');
    });

    expect(authMock).not.toHaveBeenCalled();
    // @ts-expect-error
    expect(result.current.history[1].a2uiEnvelope?.surface?.components?.[0].type).toBe('loraMarket');
  });

  it('intercepts /clear command, skips fetch, and clears chat history', async () => {
    const { result } = renderHook(() => useAgentChat());
    // @ts-expect-error
    const authMock = require('../lib/auth').authenticatedFetch;

    // Send a normal message first
    await act(async () => {
      await result.current.sendMessage('Hello');
    });
    
    authMock.mockClear();

    // Now clear it
    await act(async () => {
      await result.current.sendMessage('/clear');
    });

    expect(authMock).not.toHaveBeenCalled();
    expect(result.current.history).toHaveLength(0);
  });
});

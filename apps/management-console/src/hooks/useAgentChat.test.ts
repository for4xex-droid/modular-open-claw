/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import { renderHook, act, waitFor } from '@testing-library/react';
import { useAgentChat } from './useAgentChat';
// @ts-expect-error Node util module types may not be installed in frontend
import { TextEncoder, TextDecoder } from 'util';

(globalThis as any).TextEncoder = TextEncoder;
(globalThis as any).TextDecoder = TextDecoder as any;

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

// Mock useTtsSse
const mockSpeak = jest.fn();
const mockCancel = jest.fn();
jest.mock('./useTtsSse', () => ({
  useTtsSse: () => ({
    speak: mockSpeak,
    cancel: mockCancel
  })
}));

describe('useAgentChat TTS Integration', () => {
  beforeEach(() => {
    jest.clearAllMocks();
    mockSpeak.mockResolvedValue(undefined);
    (globalThis as any).URL.createObjectURL = jest.fn().mockReturnValue('blob:test');
    (globalThis as any).URL.revokeObjectURL = jest.fn();
    (globalThis as any).Audio = jest.fn().mockImplementation(() => ({
      play: jest.fn().mockResolvedValue(undefined),
      pause: jest.fn(),
      src: '',
      addEventListener: jest.fn((event, cb) => {
        if (event === 'ended') {
          setTimeout(cb, 0);
        }
      }),
      removeEventListener: jest.fn()
    })) as any;
  });

  it('calls useTtsSse.speak instead of fetch when autoTts is enabled and a message is sent', async () => {
    const { result } = renderHook(() => useAgentChat());
    
    // @ts-expect-error
    const authMock = require('../lib/auth').authenticatedFetch;
    authMock.mockClear();

    // Mock successful chat stream response
    authMock.mockResolvedValueOnce({
      ok: true,
      body: {
        getReader: () => {
          let done = false;
          return {
            read: () => {
              if (done) return Promise.resolve({ done: true });
              done = true;
              const encoder = new TextEncoder();
              return Promise.resolve({
                done: false,
                value: encoder.encode("event: text\ndata: Hello from assistant\n\n")
              });
            }
          };
        }
      }
    });

    await act(async () => {
      await result.current.sendMessage('Hello');
    });

    // It should call mockSpeak with the accumulated text
    expect(mockSpeak).toHaveBeenCalledWith('Hello from assistant');
    
    // It should NOT have called fetch for /api/v1/voice/synthesize
    const fetchCalls = authMock.mock.calls;
    const hasVoiceFetch = fetchCalls.some((call: any[]) => call[0].includes('/voice/synthesize'));
    expect(hasVoiceFetch).toBe(false);
  });

  it('falls back to static blob fetch if useTtsSse.speak throws an error', async () => {
    const { result } = renderHook(() => useAgentChat());
    
    // @ts-expect-error
    const authMock = require('../lib/auth').authenticatedFetch;
    authMock.mockClear();

    // Force SSE to fail
    mockSpeak.mockRejectedValueOnce(new Error('SSE Failed'));

    // Mock successful chat stream response (1st fetch)
    authMock.mockResolvedValueOnce({
      ok: true,
      body: {
        getReader: () => {
          let done = false;
          return {
            read: () => {
              if (done) return Promise.resolve({ done: true });
              done = true;
              return Promise.resolve({
                done: false,
                value: new TextEncoder().encode("event: text\ndata: Fallback text\n\n")
              });
            }
          };
        }
      }
    });

    // Mock successful blob fetch for fallback (2nd fetch)
    authMock.mockResolvedValueOnce({
      ok: true,
      blob: () => Promise.resolve(new Blob(['dummy'], { type: 'audio/wav' }))
    });

    await act(async () => {
      await result.current.sendMessage('Trigger');
    });

    // 1. SSE speak should be called
    expect(mockSpeak).toHaveBeenCalledWith('Fallback text');

    // Wait a tick for the fallback promise chain
    await waitFor(() => {
      // 2. Blob fetch fallback should be called
      const fetchCalls = authMock.mock.calls;
      const hasVoiceFetch = fetchCalls.some((call: any[]) => call[0].includes('/voice/synthesize'));
      expect(hasVoiceFetch).toBe(true);
    });
    
    // 3. ObjectURL should be created and revoked (testing the memory leak fix)
    expect((globalThis as any).URL.createObjectURL).toHaveBeenCalled();
    await waitFor(() => {
      expect((globalThis as any).URL.revokeObjectURL).toHaveBeenCalled();
    });
  });
});


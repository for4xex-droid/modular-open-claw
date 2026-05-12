import { renderHook, act } from '@testing-library/react';
import { useTtsSse } from './useTtsSse';
import { fetchEventSource } from '@microsoft/fetch-event-source';

jest.mock('@microsoft/fetch-event-source', () => ({
    fetchEventSource: jest.fn()
}));

jest.mock('../lib/auth', () => ({
    getAuthHeaders: () => ({ 'Authorization': 'Bearer test-token' })
}));

jest.mock('../config', () => ({
    API_BASE: 'http://localhost'
}));

const mockFetchEventSource = fetchEventSource as jest.MockedFunction<typeof fetchEventSource>;

describe('useTtsSse', () => {
    beforeEach(() => {
        jest.clearAllMocks();
    });

    it('should call fetchEventSource with correct params on speak()', async () => {
        const { result } = renderHook(() => useTtsSse());
        
        await act(async () => {
            await result.current.speak('Hello world');
        });

        expect(fetchEventSource).toHaveBeenCalledWith(
            'http://localhost/api/v1/voice/synthesize?stream=true',
            expect.objectContaining({
                method: 'POST',
                headers: {
                    'Authorization': 'Bearer test-token',
                    'Content-Type': 'application/json'
                },
                body: JSON.stringify({ text: 'Hello world' })
            })
        );
    });

    it('should NOT call fetchEventSource when text is empty', async () => {
        const { result } = renderHook(() => useTtsSse());

        await act(async () => {
            await result.current.speak('');
        });

        expect(fetchEventSource).not.toHaveBeenCalled();
    });

    it('should NOT call fetchEventSource when text is whitespace-only', async () => {
        const { result } = renderHook(() => useTtsSse());

        await act(async () => {
            await result.current.speak('   ');
        });

        expect(fetchEventSource).not.toHaveBeenCalled();
    });

    it('cancel() should abort the in-flight controller', async () => {
        // Track the AbortSignal passed to fetchEventSource
        let capturedSignal: AbortSignal | undefined;
        mockFetchEventSource.mockImplementation(async (_url, opts) => {
            capturedSignal = opts?.signal as AbortSignal | undefined;
            // Simulate a long-running stream that never completes
            return new Promise(() => {});
        });

        const { result } = renderHook(() => useTtsSse());

        // Start speak (don't await, it will hang intentionally)
        act(() => {
            result.current.speak('Hello');
        });

        // Verify signal was captured
        expect(capturedSignal).toBeDefined();
        expect(capturedSignal!.aborted).toBe(false);

        // Cancel
        act(() => {
            result.current.cancel();
        });

        expect(capturedSignal!.aborted).toBe(true);
    });

    it('should throw when server sends an error event', async () => {
        mockFetchEventSource.mockImplementation(async (_url, opts) => {
            const options = opts as any;
            // Simulate server sending an error event
            options.onmessage({ event: 'error', data: '{"error":"Voice engine unavailable"}' });
            // fetchEventSource catches the abort and calls onclose
            if (options.onclose) options.onclose();
        });

        const { result } = renderHook(() => useTtsSse());

        let thrownError: Error | null = null;
        await act(async () => {
            try {
                await result.current.speak('Hello');
            } catch (e) {
                thrownError = e as Error;
            }
        });

        expect(thrownError).not.toBeNull();
        expect(thrownError!.message).toBe('Voice engine unavailable');
    });
});


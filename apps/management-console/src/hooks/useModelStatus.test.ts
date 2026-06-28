/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import { renderHook, act } from '@testing-library/react';
import { useModelStatus } from './useModelStatus';
import { authenticatedFetch } from '../lib/auth';
import { fetchEventSource } from '@microsoft/fetch-event-source';

jest.mock('../lib/auth', () => ({
    authenticatedFetch: jest.fn(),
    getAuthHeaders: () => ({ Authorization: 'Bearer test' })
}));

jest.mock('@microsoft/fetch-event-source', () => ({
    fetchEventSource: jest.fn()
}));

jest.mock('../config', () => ({
    API_BASE: 'http://localhost:3000'
}));

describe('useModelStatus', () => {
    let mockFetch: jest.Mock;
    let mockFetchEventSource: jest.Mock;

    beforeEach(() => {
        jest.clearAllMocks();
        mockFetch = authenticatedFetch as jest.Mock;
        mockFetchEventSource = fetchEventSource as jest.Mock;
    });

    it('should initialize with default states', () => {
        const { result } = renderHook(() => useModelStatus());
        
        expect(result.current.status).toBeNull();
        expect(result.current.loading).toBe(true);
        expect(result.current.error).toBeNull();
        expect(result.current.isPulling).toBe(false);
        expect(result.current.pullProgress).toBeNull();
    });

    it('should fetch model status successfully', async () => {
        const mockStatusData = { ollama_ready: true, models: [] };
        mockFetch.mockResolvedValueOnce({
            ok: true,
            json: async () => mockStatusData
        });

        const { result } = renderHook(() => useModelStatus());

        await act(async () => {
            await result.current.checkStatus();
        });

        expect(result.current.status).toEqual(mockStatusData);
        expect(result.current.loading).toBe(false);
        expect(result.current.error).toBeNull();
    });

    it('should handle fetch status error gracefully', async () => {
        mockFetch.mockResolvedValueOnce({
            ok: false,
            status: 500
        });

        const { result } = renderHook(() => useModelStatus());

        await act(async () => {
            await result.current.checkStatus();
        });

        expect(result.current.status).toBeNull();
        expect(result.current.loading).toBe(false);
        expect(result.current.error).toBe('Failed to fetch model status: 500');
    });

    it('should pull model and handle progress/success events', async () => {
        let onMessageCallback: any;
        
        mockFetchEventSource.mockImplementation(async (url, options) => {
            onMessageCallback = options.onmessage;
        });

        // Mock checkStatus fetch call that happens after success
        mockFetch.mockResolvedValue({
            ok: true,
            json: async () => ({ ollama_ready: true, models: [] })
        });

        const { result } = renderHook(() => useModelStatus());

        act(() => {
            result.current.pullModel('test-model');
        });

        expect(result.current.isPulling).toBe(true);
        expect(result.current.pullProgress?.status).toBe('Preparing to download...');

        // Simulate progress event
        await act(async () => {
            onMessageCallback({ 
                event: 'progress', 
                data: JSON.stringify({ status: 'Downloading...', completed: 50, total: 100 }) 
            });
        });

        expect(result.current.pullProgress).toEqual({
            status: 'Downloading...',
            completed: 50,
            total: 100
        });

        // Simulate success progress event
        await act(async () => {
            onMessageCallback({ 
                event: 'progress', 
                data: JSON.stringify({ status: 'success' }) 
            });
        });

        expect(result.current.pullProgress?.status).toBe('Success!');
        expect(result.current.isPulling).toBe(false);
    });

    it('should pull model and handle done event', async () => {
        let onMessageCallback: any;
        
        mockFetchEventSource.mockImplementation(async (url, options) => {
            onMessageCallback = options.onmessage;
        });

        mockFetch.mockResolvedValue({ ok: true, json: async () => ({}) });

        const { result } = renderHook(() => useModelStatus());

        act(() => {
            result.current.pullModel('test-model');
        });

        // Simulate done event
        await act(async () => {
            onMessageCallback({ event: 'done' });
        });

        expect(result.current.pullProgress?.status).toBe('Success!');
        expect(result.current.isPulling).toBe(false);
    });

    it('should handle pull model error event', async () => {
        let onMessageCallback: any;
        
        mockFetchEventSource.mockImplementation(async (url, options) => {
            onMessageCallback = options.onmessage;
        });

        const { result } = renderHook(() => useModelStatus());

        act(() => {
            result.current.pullModel('test-model');
        });

        // Simulate error event
        await act(async () => {
            onMessageCallback({ 
                event: 'error', 
                data: JSON.stringify({ error: 'Model not found' }) 
            });
        });

        expect(result.current.error).toBe('Model not found');
        expect(result.current.isPulling).toBe(false);
    });

    it('should handle fetchEventSource throw', async () => {
        const consoleSpy = jest.spyOn(console, 'error').mockImplementation();
        
        mockFetchEventSource.mockRejectedValueOnce(new Error('Network disconnected'));

        const { result } = renderHook(() => useModelStatus());

        await act(async () => {
            await result.current.pullModel('test-model');
        });

        expect(result.current.error).toBe('Network disconnected');
        expect(result.current.isPulling).toBe(false);
        
        consoleSpy.mockRestore();
    });

    it('should allow cancelling pull', async () => {
        const { result } = renderHook(() => useModelStatus());

        act(() => {
            result.current.pullModel('test-model');
        });

        expect(result.current.isPulling).toBe(true);

        act(() => {
            result.current.cancelPull();
        });

        expect(result.current.isPulling).toBe(false);
        expect(result.current.pullProgress).toBeNull();
    });
});

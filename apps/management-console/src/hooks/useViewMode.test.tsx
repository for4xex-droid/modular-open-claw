/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import React from 'react';
import { renderHook, act, waitFor } from '@testing-library/react';
import { useViewMode, migrateViewMode, ViewModeProvider } from './useViewMode';
import { authenticatedFetch } from '../lib/auth';

jest.mock('../lib/auth', () => ({
    authenticatedFetch: jest.fn()
}));

jest.mock('../config', () => ({
    API_BASE: 'http://localhost:3000'
}));

const wrapper = ({ children }: { children: React.ReactNode }) => (
    <ViewModeProvider>{children}</ViewModeProvider>
);

describe('migrateViewMode', () => {
    it.each([
        ['beginner', 'simple'],
        ['simple', 'simple'],
        ['intermediate', 'cockpit'],
        ['advanced', 'cockpit'],
        ['expert', 'cockpit'],
        ['cockpit', 'cockpit'],
        ['unknown', 'cockpit'],
    ])('maps %s to %s', (input, expected) => {
        expect(migrateViewMode(input)).toBe(expected);
    });
});

describe('useViewMode', () => {
    let mockFetch: jest.Mock;

    beforeEach(() => {
        jest.clearAllMocks();
        localStorage.clear();
        mockFetch = authenticatedFetch as jest.Mock;
    });

    it('should initialize with cockpit mode by default', () => {
        mockFetch.mockResolvedValueOnce({ ok: true, json: async () => [] });
        
        const { result } = renderHook(() => useViewMode(), { wrapper });
        
        expect(result.current.viewMode).toBe('cockpit');
    });

    it('should migrate legacy localStorage values on init', () => {
        localStorage.setItem('aiome_view_mode', 'advanced');
        mockFetch.mockResolvedValueOnce({ ok: true, json: async () => [] });
        
        const { result } = renderHook(() => useViewMode(), { wrapper });
        
        expect(result.current.viewMode).toBe('cockpit');
    });

    it('should fetch view mode from API and migrate legacy values', async () => {
        mockFetch.mockResolvedValueOnce({ 
            ok: true, 
            json: async () => [{ key: 'view_mode', value: 'beginner' }] 
        });
        
        const { result } = renderHook(() => useViewMode(), { wrapper });
        
        await waitFor(() => {
            expect(result.current.viewMode).toBe('simple');
        });
        
        expect(localStorage.getItem('aiome_view_mode')).toBe('simple');
    });

    it('should update view mode and call API', async () => {
        mockFetch.mockResolvedValueOnce({ ok: true, json: async () => [] }); // initial fetch
        mockFetch.mockResolvedValueOnce({ ok: true }); // PUT request
        
        const { result } = renderHook(() => useViewMode(), { wrapper });
        
        await act(async () => {
            await result.current.setViewMode('cockpit');
        });
        
        expect(result.current.viewMode).toBe('cockpit');
        expect(localStorage.getItem('aiome_view_mode')).toBe('cockpit');
        
        expect(mockFetch).toHaveBeenCalledWith(
            'http://localhost:3000/api/v1/settings',
            expect.objectContaining({
                method: 'PUT',
                body: JSON.stringify({ key: 'view_mode', value: 'cockpit', category: 'ui' })
            })
        );
    });

    it('should share state across multiple hook consumers', async () => {
        mockFetch.mockResolvedValue({ ok: true, json: async () => [] });

        const { result } = renderHook(
            () => ({
                a: useViewMode(),
                b: useViewMode(),
            }),
            { wrapper },
        );

        await act(async () => {
            await result.current.a.setViewMode('simple');
        });

        expect(result.current.a.viewMode).toBe('simple');
        expect(result.current.b.viewMode).toBe('simple');

        await act(async () => {
            await result.current.b.setViewMode('cockpit');
        });

        expect(result.current.a.viewMode).toBe('cockpit');
        expect(result.current.b.viewMode).toBe('cockpit');
    });

    it('should throw when used outside ViewModeProvider', () => {
        expect(() => renderHook(() => useViewMode())).toThrow(
            'useViewMode must be used within ViewModeProvider',
        );
    });

    it('should handle fetch errors gracefully', async () => {
        const consoleErrorSpy = jest.spyOn(console, 'error').mockImplementation();
        mockFetch.mockRejectedValueOnce(new Error('Network error'));
        
        const { result } = renderHook(() => useViewMode(), { wrapper });
        
        // Wait a bit to ensure the effect completes
        await new Promise(resolve => setTimeout(resolve, 0));
        
        expect(result.current.viewMode).toBe('cockpit');
        expect(consoleErrorSpy).toHaveBeenCalledWith('Failed to fetch view mode', expect.any(Error));
        
        consoleErrorSpy.mockRestore();
    });

    it('should handle update errors gracefully', async () => {
        const consoleErrorSpy = jest.spyOn(console, 'error').mockImplementation();
        mockFetch.mockResolvedValueOnce({ ok: true, json: async () => [] }); // initial
        mockFetch.mockRejectedValueOnce(new Error('Update error')); // PUT
        
        const { result } = renderHook(() => useViewMode(), { wrapper });
        
        await act(async () => {
            await result.current.setViewMode('cockpit');
        });
        
        expect(result.current.viewMode).toBe('cockpit'); // State updates optimistically
        expect(consoleErrorSpy).toHaveBeenCalledWith('Failed to update view mode', expect.any(Error));
        
        consoleErrorSpy.mockRestore();
    });
});

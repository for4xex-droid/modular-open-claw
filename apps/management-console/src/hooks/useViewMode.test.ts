/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import { renderHook, act, waitFor } from '@testing-library/react';
import { useViewMode } from './useViewMode';
import { authenticatedFetch } from '../lib/auth';

jest.mock('../lib/auth', () => ({
    authenticatedFetch: jest.fn()
}));

jest.mock('../config', () => ({
    API_BASE: 'http://localhost:3000'
}));

describe('useViewMode', () => {
    let mockFetch: jest.Mock;

    beforeEach(() => {
        jest.clearAllMocks();
        localStorage.clear();
        mockFetch = authenticatedFetch as jest.Mock;
    });

    it('should initialize with intermediate mode by default', () => {
        mockFetch.mockResolvedValueOnce({ ok: true, json: async () => [] });
        
        const { result } = renderHook(() => useViewMode());
        
        expect(result.current.viewMode).toBe('intermediate');
    });

    it('should initialize from localStorage if available', () => {
        localStorage.setItem('aiome_view_mode', 'advanced');
        mockFetch.mockResolvedValueOnce({ ok: true, json: async () => [] });
        
        const { result } = renderHook(() => useViewMode());
        
        expect(result.current.viewMode).toBe('advanced');
    });

    it('should fetch view mode from API and update state', async () => {
        mockFetch.mockResolvedValueOnce({ 
            ok: true, 
            json: async () => [{ key: 'view_mode', value: 'beginner' }] 
        });
        
        const { result } = renderHook(() => useViewMode());
        
        await waitFor(() => {
            expect(result.current.viewMode).toBe('beginner');
        });
        
        expect(localStorage.getItem('aiome_view_mode')).toBe('beginner');
    });

    it('should update view mode and call API', async () => {
        mockFetch.mockResolvedValueOnce({ ok: true, json: async () => [] }); // initial fetch
        mockFetch.mockResolvedValueOnce({ ok: true }); // PUT request
        
        const { result } = renderHook(() => useViewMode());
        
        await act(async () => {
            await result.current.setViewMode('advanced');
        });
        
        expect(result.current.viewMode).toBe('advanced');
        expect(localStorage.getItem('aiome_view_mode')).toBe('advanced');
        
        expect(mockFetch).toHaveBeenCalledWith(
            'http://localhost:3000/api/v1/settings',
            expect.objectContaining({
                method: 'PUT',
                body: JSON.stringify({ key: 'view_mode', value: 'advanced', category: 'ui' })
            })
        );
    });

    it('should handle fetch errors gracefully', async () => {
        const consoleErrorSpy = jest.spyOn(console, 'error').mockImplementation();
        mockFetch.mockRejectedValueOnce(new Error('Network error'));
        
        const { result } = renderHook(() => useViewMode());
        
        // Wait a bit to ensure the effect completes
        await new Promise(resolve => setTimeout(resolve, 0));
        
        expect(result.current.viewMode).toBe('intermediate');
        expect(consoleErrorSpy).toHaveBeenCalledWith('Failed to fetch view mode', expect.any(Error));
        
        consoleErrorSpy.mockRestore();
    });

    it('should handle update errors gracefully', async () => {
        const consoleErrorSpy = jest.spyOn(console, 'error').mockImplementation();
        mockFetch.mockResolvedValueOnce({ ok: true, json: async () => [] }); // initial
        mockFetch.mockRejectedValueOnce(new Error('Update error')); // PUT
        
        const { result } = renderHook(() => useViewMode());
        
        await act(async () => {
            await result.current.setViewMode('advanced');
        });
        
        expect(result.current.viewMode).toBe('advanced'); // State updates optimistically
        expect(consoleErrorSpy).toHaveBeenCalledWith('Failed to update view mode', expect.any(Error));
        
        consoleErrorSpy.mockRestore();
    });
});

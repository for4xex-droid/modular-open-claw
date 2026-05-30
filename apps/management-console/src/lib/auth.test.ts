/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

import {
    getAuthToken,
    setAuthToken,
    clearAuthToken,
    isAuthenticated,
    getAuthHeaders,
    authenticatedFetch,
} from './auth';

describe('auth utility', () => {
    beforeEach(() => {
        sessionStorage.clear();
        jest.clearAllMocks();
        // global.fetch のモック
        global.fetch = jest.fn().mockImplementation(() =>
            Promise.resolve({
                ok: true,
                status: 200,
                json: () => Promise.resolve({ success: true }),
            } as unknown as Response)
        );
    });

    describe('Token Management', () => {
        it('should get null when no token is set', () => {
            expect(getAuthToken()).toBeNull();
            expect(isAuthenticated()).toBe(false);
        });

        it('should set, get, and clear auth token', () => {
            const token = 'test-token-12345';
            setAuthToken(token);
            expect(getAuthToken()).toBe(token);
            expect(isAuthenticated()).toBe(true);

            clearAuthToken();
            expect(getAuthToken()).toBeNull();
            expect(isAuthenticated()).toBe(false);
        });
    });

    describe('getAuthHeaders', () => {
        it('should return empty object when no token is present', () => {
            expect(getAuthHeaders()).toEqual({});
        });

        it('should return Authorization header when token is present', () => {
            const token = 'test-token-12345';
            setAuthToken(token);
            expect(getAuthHeaders()).toEqual({
                'Authorization': `Bearer ${token}`,
            });
        });
    });

    describe('authenticatedFetch', () => {
        it('should fetch without Authorization header when no token is present', async () => {
            await authenticatedFetch('/api/test');
            expect(global.fetch).toHaveBeenCalledWith('/api/test', {
                headers: {},
            });
        });

        it('should fetch with Authorization header when token is present', async () => {
            const token = 'secret-token';
            setAuthToken(token);

            await authenticatedFetch('/api/test');
            expect(global.fetch).toHaveBeenCalledWith('/api/test', {
                headers: {
                    'Authorization': `Bearer ${token}`,
                },
            });
        });

        it('should default Content-Type to application/json for body requests', async () => {
            const token = 'secret-token';
            setAuthToken(token);

            await authenticatedFetch('/api/test', {
                method: 'POST',
                body: JSON.stringify({ data: 'value' }),
            });

            expect(global.fetch).toHaveBeenCalledWith('/api/test', {
                method: 'POST',
                body: JSON.stringify({ data: 'value' }),
                headers: {
                    'Authorization': `Bearer ${token}`,
                    'Content-Type': 'application/json',
                },
            });
        });

        it('should NOT overwrite Content-Type if already provided', async () => {
            await authenticatedFetch('/api/test', {
                method: 'POST',
                body: '<xml></xml>',
                headers: {
                    'Content-Type': 'application/xml',
                },
            });

            expect(global.fetch).toHaveBeenCalledWith('/api/test', {
                method: 'POST',
                body: '<xml></xml>',
                headers: {
                    'Content-Type': 'application/xml',
                },
            });
        });

        it('should NOT set Content-Type to application/json for FormData body', async () => {
            const formData = new FormData();
            formData.append('key', 'value');

            await authenticatedFetch('/api/test', {
                method: 'POST',
                body: formData,
            });

            expect(global.fetch).toHaveBeenCalledWith('/api/test', {
                method: 'POST',
                body: formData,
                headers: {}, // FormData はブラウザが境界を設定するため、明示的な Content-Type は付与しない
            });
        });
    });
});

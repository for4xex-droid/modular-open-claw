/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

import { renderHook, act } from '@testing-library/react';
import { useCheckoutSession } from './useCheckoutSession';
import { authenticatedFetch } from '../lib/auth';
import { redirect } from '../lib/navigation';

// mock auth & navigation
jest.mock('../lib/auth', () => ({
    authenticatedFetch: jest.fn(),
}));

jest.mock('../lib/navigation', () => ({
    redirect: jest.fn(),
}));

jest.mock('../config', () => ({
    API_BASE: 'http://localhost:3000',
}));

describe('useCheckoutSession hook', () => {
    beforeEach(() => {
        jest.clearAllMocks();
    });

    it('should initialize with idle state', () => {
        const { result } = renderHook(() => useCheckoutSession('price_123', 'agent_456'));
        expect(result.current.isLoading).toBe(false);
        expect(result.current.error).toBeNull();
    });

    it('should successfully create checkout session and redirect', async () => {
        (authenticatedFetch as jest.Mock).mockResolvedValue({
            ok: true,
            json: () => Promise.resolve({ url: 'https://checkout.stripe.com/pay/cs_test' }),
        });

        const { result } = renderHook(() => useCheckoutSession('price_123', 'agent_456'));

        await act(async () => {
            await result.current.handleCheckout();
        });

        expect(authenticatedFetch).toHaveBeenCalledWith(
            expect.stringContaining('/api/v1/commerce/checkout-session/create'),
            {
                method: 'POST',
                body: JSON.stringify({
                    agent_id: 'agent_456',
                    price_id: 'price_123',
                    success_url: `${window.location.origin}/checkout/success/`,
                    cancel_url: window.location.href,
                }),
            }
        );
        expect(redirect).toHaveBeenCalledWith('https://checkout.stripe.com/pay/cs_test');
        expect(result.current.isLoading).toBe(false);
        expect(result.current.error).toBeNull();
    });

    it('should handle API failure gracefully', async () => {
        (authenticatedFetch as jest.Mock).mockResolvedValue({
            ok: false,
            status: 500,
            statusText: 'Internal Server Error',
        });

        const { result } = renderHook(() => useCheckoutSession('price_123', 'agent_456'));

        await act(async () => {
            await result.current.handleCheckout();
        });

        expect(redirect).not.toHaveBeenCalled();
        expect(result.current.isLoading).toBe(false);
        expect(result.current.error).toBe('Failed to create checkout session');
    });

    it('should handle network error (fetch rejection) gracefully', async () => {
        (authenticatedFetch as jest.Mock).mockRejectedValue(new Error('Network request failed'));

        const { result } = renderHook(() => useCheckoutSession('price_123', 'agent_456'));

        await act(async () => {
            await result.current.handleCheckout();
        });

        expect(redirect).not.toHaveBeenCalled();
        expect(result.current.isLoading).toBe(false);
        expect(result.current.error).toBe('Network request failed');
    });

    it('should handle error without message property', async () => {
        (authenticatedFetch as jest.Mock).mockRejectedValue({});

        const { result } = renderHook(() => useCheckoutSession('price_123', 'agent_456'));

        await act(async () => {
            await result.current.handleCheckout();
        });

        expect(result.current.error).toBe('Failed to create checkout session');
    });

    it('should reject response with empty url string', async () => {
        (authenticatedFetch as jest.Mock).mockResolvedValue({
            ok: true,
            json: () => Promise.resolve({ url: '' }),
        });

        const { result } = renderHook(() => useCheckoutSession('price_123', 'agent_456'));

        await act(async () => {
            await result.current.handleCheckout();
        });

        expect(redirect).not.toHaveBeenCalled();
        expect(result.current.error).toBe('Failed to create checkout session');
    });

    it('should reject response with non-http url (XSS prevention)', async () => {
        (authenticatedFetch as jest.Mock).mockResolvedValue({
            ok: true,
            json: () => Promise.resolve({ url: 'javascript:alert(1)' }),
        });

        const { result } = renderHook(() => useCheckoutSession('price_123', 'agent_456'));

        await act(async () => {
            await result.current.handleCheckout();
        });

        expect(redirect).not.toHaveBeenCalled();
        expect(result.current.error).toBe('Failed to create checkout session');
    });

    it('should successfully create customer portal session and redirect', async () => {
        (authenticatedFetch as jest.Mock).mockResolvedValue({
            ok: true,
            json: () => Promise.resolve({ url: 'https://billing.stripe.com/p/portal_test' }),
        });

        const { result } = renderHook(() => useCheckoutSession('price_123', 'agent_456'));

        await act(async () => {
            await result.current.handlePortal();
        });

        expect(authenticatedFetch).toHaveBeenCalledWith(
            expect.stringContaining('/api/v1/commerce/customer-portal/create'),
            {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json',
                },
                body: JSON.stringify({
                    agent_id: 'agent_456',
                    return_url: window.location.href,
                }),
            }
        );
        expect(redirect).toHaveBeenCalledWith('https://billing.stripe.com/p/portal_test');
        expect(result.current.isPortalLoading).toBe(false);
        expect(result.current.error).toBeNull();
    });

    it('should handle customer portal API failure gracefully', async () => {
        (authenticatedFetch as jest.Mock).mockResolvedValue({
            ok: false,
            status: 500,
        });

        const { result } = renderHook(() => useCheckoutSession('price_123', 'agent_456'));

        await act(async () => {
            await result.current.handlePortal();
        });

        expect(redirect).not.toHaveBeenCalled();
        expect(result.current.isPortalLoading).toBe(false);
        expect(result.current.error).toBe('Failed to create customer portal session');
    });
});

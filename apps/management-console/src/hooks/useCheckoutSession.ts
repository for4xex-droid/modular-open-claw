/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

import { useState, useCallback, useRef } from 'react';
import { authenticatedFetch } from '../lib/auth';
import { redirect } from '../lib/navigation';
import { API_BASE } from '../config';

export const useCheckoutSession = (priceId: string, agentId?: string) => {
    const [isLoading, setIsLoading] = useState(false);
    const [isPortalLoading, setIsPortalLoading] = useState(false);
    const [error, setError] = useState<string | null>(null);
    const pendingRef = useRef(false);

    const checkoutSuccessUrl = `${window.location.origin}/checkout/success`;
    const checkoutCancelUrl = window.location.href;

    const handleCheckout = useCallback(async () => {
        if (pendingRef.current) return;
        pendingRef.current = true;
        setIsLoading(true);
        setError(null);
        try {
            const res = await authenticatedFetch(`${API_BASE}/api/v1/commerce/checkout-session/create`, {
                method: 'POST',
                body: JSON.stringify({
                    agent_id: agentId,
                    price_id: priceId,
                    success_url: checkoutSuccessUrl,
                    cancel_url: checkoutCancelUrl,
                }),
            });

            if (res.ok) {
                const data = await res.json();
                if (typeof data.url === 'string' && data.url.startsWith('http')) {
                    redirect(data.url);
                } else {
                    throw new Error('Failed to create checkout session');
                }
            } else {
                throw new Error('Failed to create checkout session');
            }
        } catch (err: unknown) {
            setError(err instanceof Error ? err.message : 'Failed to create checkout session');
        } finally {
            pendingRef.current = false;
            setIsLoading(false);
        }
    }, [priceId, agentId, checkoutSuccessUrl, checkoutCancelUrl]);

    const handlePortal = useCallback(async () => {
        if (!agentId) return;
        setIsPortalLoading(true);
        setError(null);
        try {
            const res = await authenticatedFetch(`${API_BASE}/api/v1/commerce/customer-portal/create`, {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json',
                },
                body: JSON.stringify({
                    agent_id: agentId,
                    return_url: window.location.href,
                }),
            });

            if (res.ok) {
                const data = await res.json();
                if (typeof data.url === 'string' && data.url.startsWith('http')) {
                    redirect(data.url);
                } else {
                    throw new Error('Failed to create customer portal session');
                }
            } else {
                throw new Error('Failed to create customer portal session');
            }
        } catch (err: unknown) {
            setError(err instanceof Error ? err.message : 'Failed to create customer portal session');
        } finally {
            setIsPortalLoading(false);
        }
    }, [agentId]);

    return {
        handleCheckout,
        handlePortal,
        isLoading,
        isPortalLoading,
        error,
    };
};


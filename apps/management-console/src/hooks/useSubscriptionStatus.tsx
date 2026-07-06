/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import React, { createContext, useCallback, useContext, useEffect, useMemo, useState } from 'react';
import { API_BASE } from '../config';
import { authenticatedFetch } from '../lib/auth';
import { useAgentIdentity } from './useAgentIdentity';

/** Matches generated.ts SubscriptionStatus + auth.rs Pro gate (active | trialing) */
export type SubscriptionStatus =
  | 'active'
  | 'cancelled'
  | 'past_due'
  | 'none'
  | 'trialing'
  | 'unpaid'
  | 'incomplete'
  | 'incomplete_expired';

export interface UseSubscriptionStatusResult {
  status: SubscriptionStatus | null;
  isPro: boolean;
  isLoading: boolean;
  error: string | null;
  refresh: () => Promise<void>;
}

const SubscriptionContext = createContext<UseSubscriptionStatusResult | null>(null);

export const SubscriptionProvider: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  const { agentId } = useAgentIdentity();
  const [status, setStatus] = useState<SubscriptionStatus | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    if (!agentId) {
      setStatus(null);
      setError(null);
      return;
    }
    setIsLoading(true);
    setError(null);
    try {
      const res = await authenticatedFetch(
        `${API_BASE}/api/v1/commerce/subscription/${agentId}`
      );
      if (!res.ok) {
        const text = await res.text();
        setError(text || `Subscription fetch failed (${res.status})`);
        setStatus(null);
        return;
      }
      const data = (await res.json()) as SubscriptionStatus;
      setStatus(data);
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Subscription fetch failed');
      setStatus(null);
    } finally {
      setIsLoading(false);
    }
  }, [agentId]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const isPro = status === 'active' || status === 'trialing';

  const value = useMemo(
    () => ({ status, isPro, isLoading, error, refresh }),
    [status, isPro, isLoading, error, refresh]
  );

  return (
    <SubscriptionContext.Provider value={value}>{children}</SubscriptionContext.Provider>
  );
};

export const useSubscriptionStatus = (): UseSubscriptionStatusResult => {
  const ctx = useContext(SubscriptionContext);
  if (!ctx) {
    throw new Error('useSubscriptionStatus must be used within SubscriptionProvider');
  }
  return ctx;
};

/** Open Pro upgrade modal from anywhere (PlanBadge, LockedOverlay) */
export const openProUpgradeModal = (featureKey?: string): void => {
  if (typeof window !== 'undefined') {
    window.dispatchEvent(
      new CustomEvent('pro-upgrade-modal-open', { detail: { featureKey } })
    );
  }
};

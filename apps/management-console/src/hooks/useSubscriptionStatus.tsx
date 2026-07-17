/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import React, {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useRef,
  useState,
} from 'react';
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
  /** Resolves true when a network fetch ran; false if coalesced/skipped. */
  refresh: () => Promise<boolean>;
}

const SubscriptionContext = createContext<UseSubscriptionStatusResult | null>(null);

export const SubscriptionProvider: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  const { agentId } = useAgentIdentity();
  const [status, setStatus] = useState<SubscriptionStatus | null>(null);
  // Fail-closed: treat as loading until the first refresh settles (avoids Free CTA flash).
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const isLoadingRef = useRef(false);
  const pendingRefreshRef = useRef(false);

  const refresh = useCallback(async (): Promise<boolean> => {
    if (!agentId) {
      setStatus(null);
      setError(null);
      setIsLoading(false);
      isLoadingRef.current = false;
      pendingRefreshRef.current = false;
      return false;
    }
    if (isLoadingRef.current) {
      pendingRefreshRef.current = true;
      return false;
    }

    let fetched = false;
    // Cap follow-up coalesces so a visibility stampede cannot spin forever.
    const maxPasses = 3;
    for (let pass = 0; pass < maxPasses; pass += 1) {
      pendingRefreshRef.current = false;
      isLoadingRef.current = true;
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
        } else {
          const data = (await res.json()) as SubscriptionStatus;
          setStatus(data);
        }
        fetched = true;
      } catch (e) {
        setError(e instanceof Error ? e.message : 'Subscription fetch failed');
        setStatus(null);
        fetched = true;
      } finally {
        isLoadingRef.current = false;
        setIsLoading(false);
      }
      if (!pendingRefreshRef.current) {
        break;
      }
    }
    pendingRefreshRef.current = false;

    return fetched;
  }, [agentId]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  useEffect(() => {
    const onVisibility = () => {
      // Always invoke refresh(); in-flight calls coalesce via pendingRefreshRef.
      if (document.visibilityState === 'visible') {
        void refresh();
      }
    };
    document.addEventListener('visibilitychange', onVisibility);
    return () => document.removeEventListener('visibilitychange', onVisibility);
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

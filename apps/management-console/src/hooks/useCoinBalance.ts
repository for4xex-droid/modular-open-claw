import React, {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useState,
} from 'react';
import { authenticatedFetch } from '../lib/auth';
import { API_BASE } from '../config';
import { useAgentIdentity } from './useAgentIdentity';

const CACHE_MS = 60_000;

export interface UseCoinBalanceResult {
  balance: number;
  isLoading: boolean;
  error: boolean;
  refetch: () => Promise<void>;
  agentId: string | null;
}

const CoinBalanceContext = createContext<UseCoinBalanceResult | null>(null);

export const CoinBalanceProvider: React.FC<{ children: React.ReactNode }> = ({
  children,
}) => {
  const { agentId } = useAgentIdentity();
  const [balance, setBalance] = useState<number>(0);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState(false);
  const [lastFetch, setLastFetch] = useState(0);

  const refetch = useCallback(async () => {
    if (!agentId) return;
    setIsLoading(true);
    setError(false);
    try {
      const res = await authenticatedFetch(
        `${API_BASE}/api/v1/commerce/balance/${agentId}`,
      );
      if (res.ok) {
        const data = await res.json();
        setBalance(typeof data?.balance === 'number' ? data.balance : 0);
        setLastFetch(Date.now());
      } else {
        setError(true);
      }
    } catch {
      setError(true);
    } finally {
      setIsLoading(false);
    }
  }, [agentId]);

  useEffect(() => {
    setLastFetch(0);
    setBalance(0);
    setError(false);
  }, [agentId]);

  useEffect(() => {
    if (!agentId) return;
    if (Date.now() - lastFetch < CACHE_MS && lastFetch > 0) return;
    refetch();
  }, [agentId, lastFetch, refetch]);

  const value = useMemo(
    () => ({ balance, isLoading, error, refetch, agentId }),
    [balance, isLoading, error, refetch, agentId],
  );

  return React.createElement(CoinBalanceContext.Provider, { value }, children);
};

export function useCoinBalance(): UseCoinBalanceResult {
  const ctx = useContext(CoinBalanceContext);
  if (!ctx) {
    throw new Error('useCoinBalance must be used within CoinBalanceProvider');
  }
  return ctx;
}

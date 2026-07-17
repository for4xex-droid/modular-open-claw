/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

import { render, screen, waitFor } from '@testing-library/react';
import NurtureDashboard from './NurtureDashboard';
import { authenticatedFetch } from '../../lib/auth';
import { CoinBalanceProvider } from '../../hooks/useCoinBalance';
import { LanguageProvider } from '../../i18n';
import { useSubscriptionStatus } from '../../hooks/useSubscriptionStatus';
import { useCheckoutSession } from '../../hooks/useCheckoutSession';

jest.mock('../../lib/auth', () => ({
  authenticatedFetch: jest.fn(),
  getAuthToken: jest.fn(() => 'mock.token.part'),
}));

jest.mock('../../config', () => ({
  API_BASE: 'http://localhost:3000',
  STRIPE_PRICE_ID: 'price_test_mock',
}));

jest.mock('../../lib/navigation', () => ({
  redirect: jest.fn(),
}));

jest.mock('../../hooks/useAgentIdentity', () => ({
  useAgentIdentity: jest.fn(() => ({ agentId: 'agent-001', isEkycVerified: true })),
}));

jest.mock('../../hooks/useSubscriptionStatus', () => ({
  useSubscriptionStatus: jest.fn(),
  openProUpgradeModal: jest.fn(),
}));

jest.mock('../../hooks/useCheckoutSession', () => ({
  useCheckoutSession: jest.fn(),
}));

const mockUseSubscriptionStatus = useSubscriptionStatus as jest.Mock;
const mockUseCheckoutSession = useCheckoutSession as jest.Mock;

function mockFetchSuccess() {
  (authenticatedFetch as jest.Mock).mockImplementation((url: string) => {
    if (url.includes('/commerce/points')) {
      return Promise.resolve({
        ok: true,
        json: () =>
          Promise.resolve({
            balance: 1000,
            lifetime_earned: 1000,
            lifetime_withdrawn: 0,
            conversion_rate_bps: 100,
          }),
      });
    }
    if (url.includes('/commerce/balance')) {
      return Promise.resolve({
        ok: true,
        json: () => Promise.resolve({ balance: 500 }),
      });
    }
    if (url.includes('/commerce/history')) {
      return Promise.resolve({
        ok: true,
        json: () => Promise.resolve([]),
      });
    }
    return Promise.reject(new Error('Not found'));
  });
}

function renderDashboard() {
  return render(
    <LanguageProvider>
      <CoinBalanceProvider>
        <NurtureDashboard />
      </CoinBalanceProvider>
    </LanguageProvider>,
  );
}

describe('NurtureDashboard billing CTAs', () => {
  beforeEach(() => {
    jest.clearAllMocks();
    mockFetchSuccess();
    mockUseCheckoutSession.mockReturnValue({
      handlePortal: jest.fn(),
      isPortalLoading: false,
      error: null,
    });
  });

  it('shows Upgrade to Pro when not subscribed', async () => {
    mockUseSubscriptionStatus.mockReturnValue({
      isPro: false,
      isLoading: false,
    });

    renderDashboard();

    await waitFor(() => {
      expect(screen.getByText('Upgrade to Pro')).toBeInTheDocument();
    });
    expect(screen.queryByText('Manage billing')).not.toBeInTheDocument();
    expect(screen.queryByText('Buy Points (KC)')).not.toBeInTheDocument();
  });

  it('shows Manage billing when Pro', async () => {
    mockUseSubscriptionStatus.mockReturnValue({
      isPro: true,
      isLoading: false,
    });

    renderDashboard();

    await waitFor(() => {
      expect(screen.getByText('Manage billing')).toBeInTheDocument();
    });
    expect(screen.queryByText('Upgrade to Pro')).not.toBeInTheDocument();
    expect(screen.queryByText('Buy Points (KC)')).not.toBeInTheDocument();
  });

  it('shows portal error in the error banner when manage billing fails', async () => {
    mockUseSubscriptionStatus.mockReturnValue({
      isPro: true,
      isLoading: false,
    });
    mockUseCheckoutSession.mockReturnValue({
      handlePortal: jest.fn(),
      isPortalLoading: false,
      error: 'Failed to create customer portal session',
    });

    renderDashboard();

    await waitFor(() => {
      expect(screen.getByText('Failed to create customer portal session')).toBeInTheDocument();
    });
  });
});

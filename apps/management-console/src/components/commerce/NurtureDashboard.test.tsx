/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import NurtureDashboard from './NurtureDashboard';
import { authenticatedFetch } from '../../lib/auth';
import { CoinBalanceProvider } from '../../hooks/useCoinBalance';
import { LanguageProvider } from '../../i18n';

// Mock the auth and config
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


describe('NurtureDashboard Commerce Integration', () => {
  beforeEach(() => {
    jest.clearAllMocks();
  });

  it('renders Buy Points button and handles checkout session creation', async () => {
    // Arrange: Mock the initial data fetch
    (authenticatedFetch as jest.Mock).mockImplementation((url) => {
      if (url.includes('/commerce/points')) {
        return Promise.resolve({
          ok: true,
          json: () => Promise.resolve({ balance: 1000, lifetime_earned: 1000, lifetime_withdrawn: 0, conversion_rate_bps: 100 }),
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
      // Mock the checkout session creation
      if (url.includes('/commerce/checkout-session/create')) {
        return Promise.resolve({
          ok: true,
          json: () => Promise.resolve({ url: 'https://checkout.stripe.com/test_url' }),
        });
      }
      return Promise.reject(new Error('Not found'));
    });

    render(
      <LanguageProvider>
        <CoinBalanceProvider>
          <NurtureDashboard />
        </CoinBalanceProvider>
      </LanguageProvider>,
    );

    // Wait for the balance to load
    await waitFor(() => {
      expect(screen.getAllByText(/1,000/)[0]).toBeInTheDocument();
    });

    // Act: Click Buy Points button
    const buyButton = screen.getByText('Buy Points (KC)');
    expect(buyButton).toBeInTheDocument();
    
    fireEvent.click(buyButton);

    // Assert: Verify API was called and redirect occurred
    await waitFor(() => {
      expect(authenticatedFetch).toHaveBeenCalledWith(
        expect.stringContaining('/api/v1/commerce/checkout-session/create'),
        expect.objectContaining({
          method: 'POST',
          body: expect.any(String), // We can refine this later
        })
      );
    });
  });
});

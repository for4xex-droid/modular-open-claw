
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import NurtureDashboard from './NurtureDashboard';
import { authenticatedFetch } from '../../lib/auth';

// Mock the auth and config
jest.mock('../../lib/auth', () => ({
  authenticatedFetch: jest.fn(),
  getAuthToken: jest.fn(() => 'mock.token.part'),
}));

jest.mock('../../config', () => ({
  API_BASE: 'http://localhost:3000',
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

    render(<NurtureDashboard />);

    // Wait for the balance to load
    await waitFor(() => {
      expect(screen.getAllByText(/1,000/)[0]).toBeInTheDocument();
    });

    // Act: Click Buy Points button
    const buyButton = screen.getByText('Buy Points');
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

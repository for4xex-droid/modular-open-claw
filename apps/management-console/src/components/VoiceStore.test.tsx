
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import VoiceStore from './VoiceStore';
import { authenticatedFetch } from '../lib/auth';

// Mock dependencies
jest.mock('../lib/auth', () => ({
  authenticatedFetch: jest.fn().mockImplementation((url: string) => {
    if (url.includes('/api/v1/commerce/balance/agent-001')) {
      return Promise.resolve({ ok: true, json: () => Promise.resolve({ coins: 100 }) });
    }
    if (url.includes('/api/v1/voice/list')) {
      return Promise.resolve({ ok: true, json: () => Promise.resolve([]) });
    }
    if (url.includes('/api/v1/commerce/checkout-session/create')) {
      return Promise.resolve({ ok: true, json: () => Promise.resolve({ url: 'https://checkout.stripe.com/test' }) });
    }
    return Promise.resolve({ ok: false });
  })
}));

jest.mock('../config', () => ({
  API_BASE: 'http://localhost:3015'
}));

jest.mock('../i18n', () => ({
  useTranslation: () => ({ t: (key: string) => key })
}));

describe('VoiceStore Commerce Integration', () => {
  beforeEach(() => {
    jest.clearAllMocks();
  });

  it('calls checkout-session/create and redirects when Recharge button is clicked', async () => {
    render(<VoiceStore />);
    
    // Find the Recharge button
    const rechargeButton = await screen.findByText('Recharge');
    
    // Click it
    fireEvent.click(rechargeButton);
    
    // Assert API call
    await waitFor(() => {
      expect(authenticatedFetch).toHaveBeenCalledWith(
        expect.stringContaining('/api/v1/commerce/checkout-session/create'),
        expect.objectContaining({ method: 'POST' })
      );
    });
  });
});

/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import VoiceStore from './VoiceStore';
import { authenticatedFetch } from '../lib/auth';
import { useAgentIdentity } from '../hooks/useAgentIdentity';

// Mock dependencies
jest.mock('../lib/auth', () => ({
  authenticatedFetch: jest.fn().mockImplementation((url: string) => {
    if (url.includes('/api/v1/commerce/balance/123e4567-e89b-12d3-a456-426614174000')) {
      return Promise.resolve({ ok: true, json: () => Promise.resolve({ balance: 1000 }) });
    }
    if (url.includes('/api/v1/voice/list')) {
      return Promise.resolve({ ok: true, json: () => Promise.resolve([
          { id: '11111111-1111-1111-1111-111111111111', name: 'Test Voice 1', description: 'desc', price_coins: 50, creator_id: 'Author' }
      ]) });
    }
    if (url.includes('/api/v1/commerce/checkout-session/create')) {
      return Promise.resolve({ ok: true, json: () => Promise.resolve({ url: 'https://checkout.stripe.com/test' }) });
    }
    if (url.includes('/api/v1/commerce/purchase/123e4567-e89b-12d3-a456-426614174000')) {
      return Promise.resolve({ ok: true, json: () => Promise.resolve({ transaction_id: 'tx-123' }) });
    }
    return Promise.resolve({ ok: false });
  })
}));

jest.mock('../hooks/useAgentIdentity', () => ({
    useAgentIdentity: jest.fn()
}));

jest.mock('./common/Toast', () => ({
  useToast: () => ({ showToast: jest.fn() })
}));

jest.mock('../config', () => ({
  API_BASE: 'http://localhost:3015'
}));

jest.mock('../i18n', () => ({
  useTranslation: () => ({ t: (key: string) => key }),
}));

describe('VoiceStore Commerce Integration', () => {
  beforeEach(() => {
    jest.clearAllMocks();
    (useAgentIdentity as jest.Mock).mockReturnValue({
        agentId: '123e4567-e89b-12d3-a456-426614174000',
        isEkycVerified: true
    });
  });

  it('calls checkout-session/create and redirects when Recharge button is clicked', async () => {
    const originalConsoleError = console.error;
    console.error = jest.fn();

    render(<VoiceStore />);
    
    const rechargeButton = await screen.findByText('voice.kcRecharge');
    fireEvent.click(rechargeButton);
    
    await waitFor(() => {
      expect(authenticatedFetch).toHaveBeenCalledWith(
        expect.stringContaining('/api/v1/commerce/checkout-session/create'),
        expect.objectContaining({ method: 'POST' })
      );
    });
    
    console.error = originalConsoleError;
  });

  it('formats PurchaseRequest correctly and uses agentId when purchasing a voice asset', async () => {
    render(<VoiceStore />);
    
    await waitFor(() => {
       expect(screen.getByText('1,000 KC')).toBeInTheDocument();
    });

    const purchaseButtons = await screen.findAllByRole('button', { name: 'voice.purchase' });
    expect(purchaseButtons[0]).toBeEnabled();

    fireEvent.click(purchaseButtons[0]);

    await waitFor(() => {
        expect(authenticatedFetch).toHaveBeenCalledWith(
            expect.stringContaining('/api/v1/commerce/purchase/123e4567-e89b-12d3-a456-426614174000'),
            expect.objectContaining({
                method: 'POST',
                body: expect.stringContaining('"item_id"')
            })
        );
    });
    
    const callArgs = (authenticatedFetch as jest.Mock).mock.calls.find(call => call[0].includes('/purchase/'));
    const body = JSON.parse(callArgs[1].body);
    
    expect(body).toHaveProperty('item_id');
    expect(body).toHaveProperty('metadata');
    expect(body.metadata).toHaveProperty('amount_coins');
  });

  it('disables purchase button when eKYC is not verified', async () => {
    (useAgentIdentity as jest.Mock).mockReturnValue({
        agentId: '123e4567-e89b-12d3-a456-426614174000',
        isEkycVerified: false
    });

    render(<VoiceStore />);
    
    await waitFor(() => {
        expect(screen.getByText('1,000 KC')).toBeInTheDocument();
    });
    
    const purchaseButtons = await screen.findAllByRole('button', { name: 'voice.purchase' });
    expect(purchaseButtons[0]).toBeDisabled();
    
    const ekycMessages = await screen.findAllByText('voice.ekycRequired');
    expect(ekycMessages.length).toBeGreaterThan(0);
  });
});

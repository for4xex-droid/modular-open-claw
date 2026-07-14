/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import '@testing-library/jest-dom';
import { AiaaOnboardingWizard } from './AiaaOnboardingWizard';
import { authenticatedFetch } from '../lib/auth';

jest.mock('../lib/auth', () => ({
  authenticatedFetch: jest.fn(),
}));

jest.mock('../config', () => ({
  API_BASE: 'http://localhost',
  STRIPE_PRICE_ID: 'price_gold_monthly',
}));

jest.mock('../i18n', () => ({
  useTranslation: () => ({ t: () => undefined })
}));

const mockIdentity = {
  agentId: '11111111-1111-1111-1111-111111111111' as string | null,
  isEkycVerified: false,
};

jest.mock('../hooks/useAgentIdentity', () => ({
  useAgentIdentity: () => mockIdentity,
}));

describe('AiaaOnboardingWizard', () => {
  beforeEach(() => {
    jest.clearAllMocks();
    mockIdentity.agentId = '11111111-1111-1111-1111-111111111111';
  });

  it('generates a checkout session link with the logged-in agent id', async () => {
    (authenticatedFetch as jest.Mock).mockResolvedValue({
      ok: true,
      json: async () => ({ url: 'https://checkout.stripe.com/pay/cs_test_mock_from_api' })
    });

    render(<AiaaOnboardingWizard />);

    const nameInput = screen.getByPlaceholderText('Client / Company Name');
    fireEvent.change(nameInput, { target: { value: 'Test Corp' } });
    fireEvent.click(screen.getByText(/Next Step/i));

    const generateButton = await screen.findByText('Generate Blueprint & Checkout Link');
    fireEvent.click(generateButton);

    await waitFor(() => {
      expect(screen.getByText('Blueprint Ready!')).toBeInTheDocument();
    });

    expect(screen.getByText('https://checkout.stripe.com/pay/cs_test_mock_from_api')).toBeInTheDocument();
    expect(authenticatedFetch).toHaveBeenCalledWith(
      expect.stringContaining('/api/v1/commerce/checkout-session/create'),
      expect.objectContaining({
        method: 'POST',
        body: expect.stringContaining('11111111-1111-1111-1111-111111111111'),
      })
    );
    expect(authenticatedFetch).toHaveBeenCalledWith(
      expect.any(String),
      expect.objectContaining({
        body: expect.not.stringContaining('00000000-0000-0000-0000-000000000000'),
      })
    );
  });

  it('rejects checkout without agent identity (no zero-UUID)', async () => {
    mockIdentity.agentId = null;

    render(<AiaaOnboardingWizard />);

    const nameInput = screen.getByPlaceholderText('Client / Company Name');
    fireEvent.change(nameInput, { target: { value: 'Test Corp' } });
    fireEvent.click(screen.getByText(/Next Step/i));

    const generateButton = await screen.findByText('Generate Blueprint & Checkout Link');
    fireEvent.click(generateButton);

    await waitFor(() => {
      expect(screen.getByText(/Agent identity is required/i)).toBeInTheDocument();
    });
    expect(authenticatedFetch).not.toHaveBeenCalled();
  });

  it('handles API errors gracefully', async () => {
    (authenticatedFetch as jest.Mock).mockResolvedValue({
      ok: false,
      statusText: 'Internal Server Error'
    });

    render(<AiaaOnboardingWizard />);

    const nameInput = screen.getByPlaceholderText('Client / Company Name');
    fireEvent.change(nameInput, { target: { value: 'Test Corp' } });
    fireEvent.click(screen.getByText(/Next Step/i));

    const generateButton = await screen.findByText('Generate Blueprint & Checkout Link');
    fireEvent.click(generateButton);

    await waitFor(() => {
      expect(screen.getByText(/Error generating checkout link/i)).toBeInTheDocument();
    });
  });
});

import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import '@testing-library/jest-dom';
import { AiaaOnboardingWizard } from './AiaaOnboardingWizard';
import { authenticatedFetch } from '../lib/auth';

jest.mock('../lib/auth', () => ({
  authenticatedFetch: jest.fn(),
}));

describe('AiaaOnboardingWizard', () => {
  beforeEach(() => {
    jest.clearAllMocks();
  });

  it('generates a checkout session link successfully (RED -> GREEN)', async () => {
    (authenticatedFetch as jest.Mock).mockResolvedValue({
      ok: true,
      json: async () => ({ url: 'https://checkout.stripe.com/pay/cs_test_mock_from_api' })
    });

    render(<AiaaOnboardingWizard />);
    
    // Step 1: Discovery Session
    const nameInput = screen.getByPlaceholderText('Client / Company Name');
    fireEvent.change(nameInput, { target: { value: 'Test Corp' } });
    
    const nextButton = screen.getByText(/Next Step/i);
    expect(nextButton).not.toBeDisabled();
    fireEvent.click(nextButton);
    
    // Step 2: Economics & ROI
    const generateButton = await screen.findByText('Generate Blueprint & Checkout Link');
    fireEvent.click(generateButton);
    
    // Wait for Step 3: Blueprint Ready
    await waitFor(() => {
      expect(screen.getByText('Blueprint Ready!')).toBeInTheDocument();
    });
    
    expect(screen.getByText('https://checkout.stripe.com/pay/cs_test_mock_from_api')).toBeInTheDocument();
    
    expect(authenticatedFetch).toHaveBeenCalledWith(
      expect.stringContaining('/api/v1/commerce/checkout-session/create'),
      expect.objectContaining({
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: expect.stringContaining('agent_id'),
      })
    );
  });

  it('handles API errors gracefully', async () => {
    (authenticatedFetch as jest.Mock).mockResolvedValue({
      ok: false,
      statusText: 'Internal Server Error'
    });

    render(<AiaaOnboardingWizard />);
    
    const nameInput = screen.getByPlaceholderText('Client / Company Name');
    fireEvent.change(nameInput, { target: { value: 'Test Corp' } });
    
    const nextButton = screen.getByText(/Next Step/i);
    fireEvent.click(nextButton);
    
    const generateButton = await screen.findByText('Generate Blueprint & Checkout Link');
    fireEvent.click(generateButton);
    
    await waitFor(() => {
      expect(screen.getByText(/Error generating checkout link/i)).toBeInTheDocument();
    });
  });
});

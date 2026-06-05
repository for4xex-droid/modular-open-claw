import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { CTA } from './CTA';
import '../i18n/config';

describe('CTA Component', () => {
  const originalFetch = globalThis.fetch;

  beforeEach(() => {
    // Mock VITE_FORMSPREE_ID env var for form rendering tests
    vi.stubEnv('VITE_FORMSPREE_ID', 'YOUR_FORM_ID');
    // Mock fetch to prevent real HTTP requests during test
    globalThis.fetch = vi.fn().mockResolvedValue({ ok: true });
  });

  afterEach(() => {
    vi.unstubAllEnvs();
    globalThis.fetch = originalFetch;
  });

  it('renders the CTA title and description', () => {
    render(<CTA />);
    expect(screen.getByText('Start building with Aiome today.')).toBeInTheDocument();
    expect(screen.getByText(/Ready to step into the future/i)).toBeInTheDocument();
  });

  it('renders the deploy link with correct destination', () => {
    render(<CTA />);
    const deployLink = screen.getByRole('link', { name: /Deploy Now/i });
    expect(deployLink).toBeInTheDocument();
    expect(deployLink).toHaveAttribute('href', '#quickstart');
  });

  it('renders the email subscription form and handles successful submission', async () => {
    render(<CTA />);
    
    const emailInput = screen.getByPlaceholderText('Enter your work email');
    const submitButton = screen.getByRole('button', { name: 'Get Early Access' });
    const gdprCheckbox = screen.getByLabelText(/I agree to receive updates/i);
    
    expect(emailInput).toBeInTheDocument();
    expect(submitButton).toBeInTheDocument();
    expect(gdprCheckbox).toBeInTheDocument();
    
    expect(gdprCheckbox).not.toBeChecked();
    
    fireEvent.click(gdprCheckbox);
    fireEvent.change(emailInput, { target: { value: 'test@example.com' } });
    fireEvent.click(submitButton);
    
    await waitFor(() => {
      expect(screen.getByText("Thank you! We've added you to the waitlist.")).toBeInTheDocument();
    });

    expect(globalThis.fetch).toHaveBeenCalledWith(
      'https://formspree.io/f/YOUR_FORM_ID',
      expect.objectContaining({ method: 'POST' }),
    );
  });

  it('shows error message when submission fails', async () => {
    const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {});
    (globalThis.fetch as ReturnType<typeof vi.fn>).mockRejectedValueOnce(new Error('Network error'));
    render(<CTA />);

    const emailInput = screen.getByPlaceholderText('Enter your work email');
    const gdprCheckbox = screen.getByLabelText(/I agree to receive updates/i);
    const submitButton = screen.getByRole('button', { name: 'Get Early Access' });

    fireEvent.click(gdprCheckbox);
    fireEvent.change(emailInput, { target: { value: 'fail@example.com' } });
    fireEvent.click(submitButton);

    await waitFor(() => {
      expect(screen.getByText(/Something went wrong/i)).toBeInTheDocument();
    });

    consoleSpy.mockRestore();
  });

  it('does not render the subscription form when VITE_FORMSPREE_ID is not set', () => {
    vi.stubEnv('VITE_FORMSPREE_ID', '');
    render(<CTA />);
    
    expect(screen.getByText('Start building with Aiome today.')).toBeInTheDocument();
    expect(screen.queryByPlaceholderText('Enter your work email')).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Get Early Access' })).not.toBeInTheDocument();
    expect(screen.getByRole('link', { name: /Deploy Now/i })).toBeInTheDocument();
  });
});


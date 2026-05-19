import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import SetupWizard from './SetupWizard';

jest.mock('../i18n', () => ({
  useTranslation: () => ({
    t: (key: string) => null, // return null to exercise fallback strings
    i18n: { changeLanguage: jest.fn() }
  })
}));

jest.mock('../config', () => ({
  API_BASE: 'http://localhost:3015'
}));

jest.mock('../lib/auth', () => ({
  setAuthToken: jest.fn()
}));

global.fetch = jest.fn();

describe('SetupWizard Component', () => {
  beforeEach(() => {
    jest.clearAllMocks();
  });

  it('should complete the full wizard flow end-to-end', async () => {
    (global.fetch as jest.Mock).mockResolvedValue({
      ok: true,
      json: async () => ({ access_token: "test_token" })
    });

    const onCompleteMock = jest.fn();
    render(<SetupWizard onComplete={onCompleteMock} />);

    // Step 0: Intro
    expect(screen.getByText(/Welcome to Aiome/i)).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: /Start Setup/i }));

    // Step 1: TOS
    await waitFor(() => expect(screen.getByRole('heading', { name: /Terms of Service/i })).toBeInTheDocument());
    const nextBtn = screen.getByRole('button', { name: /Next/i });
    expect(nextBtn).toBeDisabled(); // Cannot proceed without accepting TOS
    fireEvent.click(screen.getByLabelText(/I agree to/i));
    expect(nextBtn).not.toBeDisabled();
    fireEvent.click(nextBtn);

    // Step 2: AI Name
    await waitFor(() => expect(screen.getByText(/Name your AI/i)).toBeInTheDocument());
    fireEvent.change(screen.getByPlaceholderText(/e.g. Watchtower/i), { target: { value: 'MyTestAI' } });
    fireEvent.click(screen.getByRole('button', { name: /Next/i }));

    // Step 3: View Mode
    await waitFor(() => expect(screen.getByText(/Choose your experience/i)).toBeInTheDocument());
    fireEvent.click(screen.getByText(/Expert Mode/i));

    // Step 4: Admin Credentials
    await waitFor(() => expect(screen.getByText(/Create Admin/i)).toBeInTheDocument());
    fireEvent.change(screen.getByLabelText(/Email/i), { target: { value: 'admin@example.com' } });
    fireEvent.change(screen.getByLabelText(/^Password/i), { target: { value: 'SecurePass1234!' } });
    fireEvent.change(screen.getByLabelText(/Confirm Password/i), { target: { value: 'SecurePass1234!' } });
    fireEvent.click(screen.getByRole('button', { name: /Initialize System/i }));

    // Step 5: Finalizing
    await waitFor(() => expect(screen.getByText(/Awakening/i)).toBeInTheDocument());

    await waitFor(() => {
      expect(global.fetch).toHaveBeenCalledWith(
        'http://localhost:3015/api/v1/setup/init',
        expect.objectContaining({ method: 'POST' })
      );
      expect(onCompleteMock).toHaveBeenCalled();
    });
  });

  it('should show inline validation for invalid email', async () => {
    render(<SetupWizard onComplete={jest.fn()} />);

    // Navigate to admin step
    fireEvent.click(screen.getByRole('button', { name: /Start Setup/i }));
    await waitFor(() => screen.getByRole('heading', { name: /Terms of Service/i }));
    fireEvent.click(screen.getByLabelText(/I agree to/i));
    fireEvent.click(screen.getByRole('button', { name: /Next/i }));
    await waitFor(() => screen.getByText(/Name your AI/i));
    fireEvent.click(screen.getByRole('button', { name: /Next/i }));
    await waitFor(() => screen.getByText(/Choose your experience/i));
    fireEvent.click(screen.getByText(/Simple Mode/i));

    // Type invalid email
    await waitFor(() => screen.getByText(/Create Admin/i));
    fireEvent.change(screen.getByLabelText(/Email/i), { target: { value: 'notanemail' } });

    expect(screen.getByText(/valid email/i)).toBeInTheDocument();
  });

  it('should show inline validation for password mismatch', async () => {
    render(<SetupWizard onComplete={jest.fn()} />);

    // Navigate to admin step
    fireEvent.click(screen.getByRole('button', { name: /Start Setup/i }));
    await waitFor(() => screen.getByRole('heading', { name: /Terms of Service/i }));
    fireEvent.click(screen.getByLabelText(/I agree to/i));
    fireEvent.click(screen.getByRole('button', { name: /Next/i }));
    await waitFor(() => screen.getByText(/Name your AI/i));
    fireEvent.click(screen.getByRole('button', { name: /Next/i }));
    await waitFor(() => screen.getByText(/Choose your experience/i));
    fireEvent.click(screen.getByText(/Simple Mode/i));

    await waitFor(() => screen.getByText(/Create Admin/i));
    fireEvent.change(screen.getByLabelText(/^Password/i), { target: { value: 'SecurePass1234!' } });
    fireEvent.change(screen.getByLabelText(/Confirm Password/i), { target: { value: 'DifferentPass!' } });

    expect(screen.getByText(/do not match/i)).toBeInTheDocument();
  });

  it('should disable submit button when email is invalid', async () => {
    render(<SetupWizard onComplete={jest.fn()} />);

    // Navigate to admin step
    fireEvent.click(screen.getByRole('button', { name: /Start Setup/i }));
    await waitFor(() => screen.getByRole('heading', { name: /Terms of Service/i }));
    fireEvent.click(screen.getByLabelText(/I agree to/i));
    fireEvent.click(screen.getByRole('button', { name: /Next/i }));
    await waitFor(() => screen.getByText(/Name your AI/i));
    fireEvent.click(screen.getByRole('button', { name: /Next/i }));
    await waitFor(() => screen.getByText(/Choose your experience/i));
    fireEvent.click(screen.getByText(/Simple Mode/i));

    await waitFor(() => screen.getByText(/Create Admin/i));
    const initBtn = screen.getByRole('button', { name: /Initialize System/i });

    // Initially disabled
    expect(initBtn).toBeDisabled();

    // Fill valid email but short password
    fireEvent.change(screen.getByLabelText(/Email/i), { target: { value: 'admin@example.com' } });
    fireEvent.change(screen.getByLabelText(/^Password/i), { target: { value: 'short' } });
    expect(initBtn).toBeDisabled();
  });

  it('should show server error message when setup API returns 400', async () => {
    (global.fetch as jest.Mock).mockResolvedValue({
      ok: false,
      json: async () => ({ message: "Setup has already been completed" })
    });

    render(<SetupWizard onComplete={jest.fn()} />);

    // Navigate to admin step
    fireEvent.click(screen.getByRole('button', { name: /Start Setup/i }));
    await waitFor(() => screen.getByRole('heading', { name: /Terms of Service/i }));
    fireEvent.click(screen.getByLabelText(/I agree to/i));
    fireEvent.click(screen.getByRole('button', { name: /Next/i }));
    await waitFor(() => screen.getByText(/Name your AI/i));
    fireEvent.click(screen.getByRole('button', { name: /Next/i }));
    await waitFor(() => screen.getByText(/Choose your experience/i));
    fireEvent.click(screen.getByText(/Standard Mode/i));

    await waitFor(() => screen.getByText(/Create Admin/i));
    fireEvent.change(screen.getByLabelText(/Email/i), { target: { value: 'admin@example.com' } });
    fireEvent.change(screen.getByLabelText(/^Password/i), { target: { value: 'SecurePass1234!' } });
    fireEvent.change(screen.getByLabelText(/Confirm Password/i), { target: { value: 'SecurePass1234!' } });
    fireEvent.click(screen.getByRole('button', { name: /Initialize System/i }));

    // Should show error and return to credentials step
    await waitFor(() => {
      expect(screen.getByRole('alert')).toBeInTheDocument();
      expect(screen.getByText(/Setup has already been completed/i)).toBeInTheDocument();
    });
  });

  it('should show network error message when fetch rejects', async () => {
    (global.fetch as jest.Mock).mockRejectedValue(new TypeError('Failed to fetch'));

    render(<SetupWizard onComplete={jest.fn()} />);

    // Navigate to admin step
    fireEvent.click(screen.getByRole('button', { name: /Start Setup/i }));
    await waitFor(() => screen.getByRole('heading', { name: /Terms of Service/i }));
    fireEvent.click(screen.getByLabelText(/I agree to/i));
    fireEvent.click(screen.getByRole('button', { name: /Next/i }));
    await waitFor(() => screen.getByText(/Name your AI/i));
    fireEvent.click(screen.getByRole('button', { name: /Next/i }));
    await waitFor(() => screen.getByText(/Choose your experience/i));
    fireEvent.click(screen.getByText(/Expert Mode/i));

    await waitFor(() => screen.getByText(/Create Admin/i));
    fireEvent.change(screen.getByLabelText(/Email/i), { target: { value: 'admin@example.com' } });
    fireEvent.change(screen.getByLabelText(/^Password/i), { target: { value: 'SecurePass1234!' } });
    fireEvent.change(screen.getByLabelText(/Confirm Password/i), { target: { value: 'SecurePass1234!' } });
    fireEvent.click(screen.getByRole('button', { name: /Initialize System/i }));

    // Should show user-friendly network error message
    await waitFor(() => {
      expect(screen.getByRole('alert')).toBeInTheDocument();
      expect(screen.getByText(/Unable to reach the server/i)).toBeInTheDocument();
    });
  });
});

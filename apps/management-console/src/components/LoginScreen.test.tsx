import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import LoginScreen from './LoginScreen';

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

jest.mock('./fluid/FluidBackground', () => () => <div data-testid="fluid-bg" />);

global.fetch = jest.fn();

describe('LoginScreen Component', () => {
  beforeEach(() => {
    jest.clearAllMocks();
  });

  it('renders login form and authenticates on success', async () => {
    (global.fetch as jest.Mock).mockResolvedValue({
      ok: true,
      json: async () => ({ access_token: "test_token" })
    });

    const onAuthMock = jest.fn();
    render(<LoginScreen onAuthenticated={onAuthMock} />);

    // Check UI renders with fallback text
    expect(screen.getByText(/Aiome Identity/i)).toBeInTheDocument();
    
    const input = screen.getByLabelText(/Password/i);
    fireEvent.change(input, { target: { value: 'mysecret' } });
    
    const submitBtn = screen.getByRole('button', { name: /Login/i });
    fireEvent.click(submitBtn);

    await waitFor(() => {
      expect(global.fetch).toHaveBeenCalledWith('http://localhost:3015/api/v1/auth/token', expect.objectContaining({
        method: 'POST',
        body: expect.stringContaining('mysecret')
      }));
      expect(onAuthMock).toHaveBeenCalled();
    });
  });

  it('shows error on failure', async () => {
    (global.fetch as jest.Mock).mockResolvedValue({
      ok: false,
      json: async () => ({ message: "Invalid password" })
    });

    render(<LoginScreen onAuthenticated={jest.fn()} />);

    const input = screen.getByLabelText(/Password/i);
    fireEvent.change(input, { target: { value: 'wrong' } });
    fireEvent.click(screen.getByRole('button', { name: /Login/i }));

    await waitFor(() => {
      expect(screen.getByRole('alert')).toBeInTheDocument();
      expect(screen.getByText(/Invalid password/i)).toBeInTheDocument();
    });
  });

  it('disables submit button when password is empty', () => {
    render(<LoginScreen onAuthenticated={jest.fn()} />);
    const submitBtn = screen.getByRole('button', { name: /Login/i });
    expect(submitBtn).toBeDisabled();
  });

  it('does not submit when password is empty string', () => {
    render(<LoginScreen onAuthenticated={jest.fn()} />);
    
    const input = screen.getByLabelText(/Password/i);
    fireEvent.change(input, { target: { value: '' } });
    fireEvent.click(screen.getByRole('button', { name: /Login/i }));

    // fetch should not be called (empty password)
    expect(global.fetch).not.toHaveBeenCalled();
  });
});

import React from 'react';
import { render, screen, fireEvent } from '@testing-library/react';
import AuthOverlay from './AuthOverlay';
import { useTranslation } from '../i18n';

jest.mock('../config', () => ({
  API_BASE: 'http://localhost:3015'
}));

// Mock i18n
jest.mock('../i18n', () => ({
  useTranslation: () => ({
    t: (key: string) => {
      const keys: Record<string, string> = {
        'auth.title': 'Login',
        'auth.secretKeyLabel': 'Password',
        'auth.synchronize': 'Login',
        'auth.errorInvalidKey': 'Password is incorrect. Please try again.',
      };
      return keys[key] || key;
    }
  })
}));

describe('AuthOverlay - Humanized UI', () => {
  const mockOnAuthenticated = jest.fn();

  beforeEach(() => {
    jest.clearAllMocks();
  });

  it('renders humanized login UI instead of SECRET KEY', () => {
    render(<AuthOverlay onAuthenticated={mockOnAuthenticated} />);
    
    // RED: We expect human-friendly terms, not "SECRET KEY" or "Synchronize"
    expect(screen.getByRole('heading', { name: 'Login' })).toBeInTheDocument();
    
    // We expect the label to be Password, not SECRET KEY
    expect(screen.getByText('Password')).toBeInTheDocument();
    
    const button = screen.getByRole('button', { name: /Login/i });
    expect(button).toBeInTheDocument();
  });

  it('shows humanized error message on failure', async () => {
    // Mock fetch to simulate 401
    global.fetch = jest.fn().mockResolvedValue({
      ok: false,
      status: 401
    }) as jest.Mock;

    render(<AuthOverlay onAuthenticated={mockOnAuthenticated} />);
    
    // AuthOverlay input does not have a placeholder text that we mock to "Enter your password".
    // We find it by its actual placeholder.
    const input = screen.getByPlaceholderText('••••••••••••••••');
    const button = screen.getByRole('button', { name: /Login/i });
    
    fireEvent.change(input, { target: { value: 'wrong-password' } });
    fireEvent.click(button);
    
    // RED: We expect a friendly message, not "Authentication failed (401)"
    const errorMsg = await screen.findByText('Password is incorrect. Please try again.');
    expect(errorMsg).toBeInTheDocument();
  });
});

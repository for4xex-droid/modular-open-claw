/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import { render, screen, waitFor, fireEvent } from '@testing-library/react';
import SkillVault from './SkillVault';
import { authenticatedFetch } from '../lib/auth';
import { useToast } from './common/Toast';

jest.mock('../config', () => ({
  API_BASE: 'http://localhost'
}));

// Mock Auth
jest.mock('../lib/auth', () => ({
  authenticatedFetch: jest.fn()
}));

// Mock i18n
jest.mock('../i18n', () => ({
  useTranslation: () => ({ t: (k: string) => k })
}));

// Mock Toast
jest.mock('./common/Toast', () => ({
  useToast: jest.fn()
}));

describe('SkillVault Component', () => {
  const mockShowToast = jest.fn();

  beforeEach(() => {
    jest.clearAllMocks();
    (useToast as jest.Mock).mockReturnValue({ showToast: mockShowToast });
  });

  it('GREEN: Install Skill button is enabled and triggers api install successfully', async () => {
    (authenticatedFetch as jest.Mock).mockImplementation((url: string) => {
      // M1 fix: install endpoint MUST be matched first (more specific path)
      if (url.endsWith('/api/v1/skills/install')) {
        return Promise.resolve({ ok: true });
      }
      if (url.endsWith('/api/skills')) {
        return Promise.resolve({
          ok: true,
          json: async () => [{
            name: 'Test Marketplace Skill',
            description: 'A test skill from market',
            source: 'marketplace',
            status: 'Available',
            layer: 1,
            tools: []
          }]
        });
      }
      return Promise.resolve({ ok: false });
    });

    render(<SkillVault />);

    const installBtn = await waitFor(() => {
      // i18n mock returns the key string (e.g. 'skill.install'), not defaultValue
      const btn = screen.getByText(/skill\.install/i).closest('button');
      expect(btn).toBeInTheDocument();
      return btn;
    });
    
    expect(installBtn).not.toBeDisabled();
    
    fireEvent.click(installBtn!);

    await waitFor(() => {
      expect(authenticatedFetch).toHaveBeenCalledWith(
        'http://localhost/api/v1/skills/install',
        expect.objectContaining({
          method: 'POST',
          body: JSON.stringify({ name: 'Test Marketplace Skill' })
        })
      );
      expect(mockShowToast).toHaveBeenCalledWith('success', expect.any(String));
    });
  });
});

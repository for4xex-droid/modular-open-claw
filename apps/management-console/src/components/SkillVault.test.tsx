import React from 'react';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import SkillVault from './SkillVault';
import { authenticatedFetch } from '../lib/auth';

// Mock Auth
jest.mock('../lib/auth', () => ({
  authenticatedFetch: jest.fn()
}));

// Mock i18n
jest.mock('../i18n', () => ({
  useTranslation: () => ({ t: (k: string) => k })
}));

describe('SkillVault Component', () => {
  beforeEach(() => {
    jest.clearAllMocks();
    window.alert = jest.fn();
  });

  it('RED: "Install Skill" button should show a coming soon alert when clicked', async () => {
    // Arrange: Mock the API to return a marketplace skill
    (authenticatedFetch as jest.Mock).mockResolvedValue({
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

    render(<SkillVault />);

    // Wait for skills to load
    await waitFor(() => {
      expect(screen.queryByText(/skill.loading/i)).not.toBeInTheDocument();
    });

    // Act
    const installBtn = screen.getByText(/Install Skill/i);
    fireEvent.click(installBtn);

    // Assert: The button should be wired to an onClick that alerts "Coming soon in Phase 3"
    expect(window.alert).toHaveBeenCalledWith('Coming soon in Phase 3');
  });
});

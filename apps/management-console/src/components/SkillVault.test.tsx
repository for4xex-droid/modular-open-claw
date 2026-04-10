import { render, screen, waitFor } from '@testing-library/react';
import SkillVault from './SkillVault';
import { authenticatedFetch } from '../lib/auth';

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

describe('SkillVault Component', () => {
  beforeEach(() => {
    jest.clearAllMocks();
    window.alert = jest.fn();
  });

  it('RED: "Install Skill" button should be disabled and show coming soon title', async () => {
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
    const installBtn = screen.getByText(/Install Skill/i).closest('button');

    // Assert: The button should be disabled and have title
    expect(installBtn).toBeDisabled();
    expect(installBtn).toHaveAttribute('title', 'Coming soon in Phase 3');
  });
});

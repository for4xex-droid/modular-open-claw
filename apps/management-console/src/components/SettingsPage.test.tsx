import { render, screen, fireEvent } from '@testing-library/react';
import SettingsPage from './SettingsPage';

// Mock required contexts
jest.mock('../hooks/AvatarContext', () => ({
  useAvatarCharacter: () => ({
    character: 'female',
    setCharacter: jest.fn(),
    proportion: 'chibi',
    setProportion: jest.fn()
  })
}));

jest.mock('../hooks/useDisplayMode', () => ({
  useDisplayMode: () => ({
    mode: 'vrm',
    setMode: jest.fn()
  })
}));

// Mock translation
jest.mock('../i18n', () => ({
  useTranslation: () => ({
    t: (key: string) => key
  })
}));

// Mock Auth lib
jest.mock('../lib/auth', () => ({
  authenticatedFetch: jest.fn().mockImplementation(() => Promise.resolve({
    ok: true,
    json: () => Promise.resolve([])
  })),
  setAuthToken: jest.fn(),
  clearAuthToken: jest.fn()
}));

// Mock config
jest.mock('../config', () => ({
  API_BASE: 'http://localhost'
}));

// Mock useViewMode
let mockViewMode = 'advanced';
jest.mock('../hooks/useViewMode', () => ({
  useViewMode: () => ({
    viewMode: mockViewMode,
    setViewMode: jest.fn((mode) => { mockViewMode = mode; })
  })
}));

describe('SettingsPage Integrations', () => {
  beforeEach(() => {
    jest.clearAllMocks();
    mockViewMode = 'advanced'; // Default to advanced for existing tests
  });

  it('renders Channel Bridges section with X Bearer Token input', async () => {
    render(<SettingsPage />);
    
    // Wait for the page to load
    await screen.findByText('settings.appearance');
    
    // Assert the new section heading exists
    const bridgesHeading = await screen.findByText('settings.channelBridges');
    expect(bridgesHeading).toBeInTheDocument();

    // Assert the X Bearer Token input is rendered
    const xBearerTokenLabel = screen.getByText('settings.xBearerToken');
    expect(xBearerTokenLabel).toBeInTheDocument();
  });

  it('handles boundary conditions for X Bearer Token', async () => {
    render(<SettingsPage />);
    await screen.findByText('settings.appearance');

    // The component uses getSetting which will fall back to local state
    // Just verify the setting input rendering won't crash and we can type empty boundaries.
    const xBearerInputs = screen.getAllByPlaceholderText('settings.enterApiKey');
    expect(xBearerInputs.length).toBeGreaterThan(0);
    
    // Pick the last one (since the other might be search/api key)
    const tokenInput = xBearerInputs[xBearerInputs.length - 1];
    
    // Simulate updating with empty boundary (null/blank)
    fireEvent.change(tokenInput, { target: { value: '' } });
    fireEvent.blur(tokenInput);
    
    expect(tokenInput).toHaveValue('');

    // Simulate exceedingly long payload
    const overflowToken = 'B'.repeat(500);
    fireEvent.change(tokenInput, { target: { value: overflowToken } });
    fireEvent.blur(tokenInput);
    
    expect(tokenInput).toHaveValue(overflowToken);
  });

  it('hides intermediate and advanced sections in beginner mode', async () => {
    mockViewMode = 'beginner';
    render(<SettingsPage />);
    
    // Should see Appearance and LLM Configuration
    expect(await screen.findByText('settings.appearance')).toBeInTheDocument();
    expect(screen.getByText('settings.llmEngine')).toBeInTheDocument();

    // Should NOT see Commerce, Channel Bridges, Security, Feature Flags, Escrow, MCP
    expect(screen.queryByText('settings.commerceEconomicBase')).not.toBeInTheDocument();
    expect(screen.queryByText('settings.channelBridges')).not.toBeInTheDocument();
    expect(screen.queryByText('settings.securityInfrastructure')).not.toBeInTheDocument();
    expect(screen.queryByText('settings.featureFlags')).not.toBeInTheDocument();
  });

  it('shows intermediate sections but hides advanced sections in intermediate mode', async () => {
    mockViewMode = 'intermediate';
    render(<SettingsPage />);
    
    // Should see Appearance, LLM, Commerce, Channel Bridges, Security
    expect(await screen.findByText('settings.appearance')).toBeInTheDocument();
    expect(screen.getByText('settings.llmEngine')).toBeInTheDocument();
    expect(screen.getByText('settings.commerceEconomicBase')).toBeInTheDocument();
    expect(screen.getByText('settings.channelBridges')).toBeInTheDocument();
    expect(screen.getByText('settings.securityInfrastructure')).toBeInTheDocument();

    // Should NOT see Feature Flags (advanced only)
    expect(screen.queryByText('settings.featureFlags')).not.toBeInTheDocument();
  });

  it('shows all sections in advanced mode', async () => {
    mockViewMode = 'advanced';
    render(<SettingsPage />);
    
    // All sections should be visible
    expect(await screen.findByText('settings.appearance')).toBeInTheDocument();
    expect(screen.getByText('settings.llmEngine')).toBeInTheDocument();
    expect(screen.getByText('settings.commerceEconomicBase')).toBeInTheDocument();
    expect(screen.getByText('settings.channelBridges')).toBeInTheDocument();
    expect(screen.getByText('settings.securityInfrastructure')).toBeInTheDocument();
    expect(screen.getByText('settings.featureFlags')).toBeInTheDocument();
  });
});

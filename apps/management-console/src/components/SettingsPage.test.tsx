/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import SettingsPage from './SettingsPage';

export const mockSetCharacter = jest.fn();
export const mockSetProportion = jest.fn();
export const mockSetMode = jest.fn();

// Mock required contexts
jest.mock('../hooks/AvatarContext', () => ({
  useAvatarCharacter: () => ({
    character: 'female',
    setCharacter: mockSetCharacter,
    proportion: 'chibi',
    setProportion: mockSetProportion
  })
}));

jest.mock('../hooks/useDisplayMode', () => ({
  useDisplayMode: () => ({
    mode: 'vrm',
    setMode: mockSetMode
  })
}));

// Mock translation
export const mockSetLang = jest.fn();
jest.mock('../i18n', () => ({
  useTranslation: () => ({
    t: (key: string) => key
  }),
  useLanguage: () => ({
    lang: 'ja',
    setLang: mockSetLang
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

// Mock Toast
jest.mock('./common/Toast', () => ({
  useToast: () => ({ showToast: jest.fn() })
}));

// Mock EscrowManagementView to prevent async act warnings from it
jest.mock('./EscrowManagementView', () => {
  return function DummyEscrowView() {
    return <div data-testid="escrow-management-view" />;
  };
});

// Mock useViewMode
let mockViewMode: 'simple' | 'cockpit' = 'cockpit';
jest.mock('../hooks/useViewMode', () => ({
  useViewMode: () => ({
    viewMode: mockViewMode,
    setViewMode: jest.fn((mode: 'simple' | 'cockpit') => { mockViewMode = mode; })
  })
}));

const mockInvoke = jest.fn();
jest.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

describe('SettingsPage Integrations', () => {
  beforeEach(() => {
    jest.clearAllMocks();
    mockViewMode = 'cockpit'; // Default to cockpit for existing tests
    delete (window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__;
    mockInvoke.mockReset();
  });

  it('shows Nurture Mode section on Desktop Tauri and applies mode', async () => {
    (window as unknown as { __TAURI_INTERNALS__: object }).__TAURI_INTERNALS__ = {};
    mockInvoke.mockImplementation(async (cmd: string) => {
      if (cmd === 'get_nurture_status') {
        return { mode: 'in_process', status: 'in_process', url: 'http://127.0.0.1:3015' };
      }
      if (cmd === 'set_nurture_mode') {
        return { mode: 'local', status: 'running', url: 'http://localhost:3020' };
      }
      return {};
    });

    render(<SettingsPage />);
    expect(await screen.findByTestId('nurture-mode-section')).toBeInTheDocument();
    expect(await screen.findByTestId('nurture-mode-status')).toHaveTextContent('settings.nurtureModeCurrent');

    fireEvent.click(screen.getByText('settings.nurtureMode_local'));
    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith('set_nurture_mode', { mode: 'local' });
    });
  });

  it('renders Channel Bridges section with X Bearer Token and Search API Key inputs', async () => {
    render(<SettingsPage />);
    
    // Wait for the page to load
    await screen.findByText('settings.appearance');
    
    // Assert the new section heading exists
    const bridgesHeading = await screen.findByText('settings.channelBridges');
    expect(bridgesHeading).toBeInTheDocument();

    // Assert the X Bearer Token input is rendered
    const xBearerTokenLabel = screen.getByText('settings.xBearerToken');
    expect(xBearerTokenLabel).toBeInTheDocument();
    expect(screen.getByText('settings.xBearerTokenNotice')).toBeInTheDocument();

    // OP-026: search_api_key (WebSearch/Serp) — existing i18n keys
    expect(screen.getByText('settings.searchApiKey')).toBeInTheDocument();
    expect(screen.getByText('settings.searchApiKeyNotice')).toBeInTheDocument();
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

    // Wait for the async updateSetting call to resolve
    // @ts-expect-error
    const { authenticatedFetch } = require('../lib/auth');
    await waitFor(() => {
      expect(authenticatedFetch).toHaveBeenCalledWith(
        expect.stringContaining('/api/v1/settings'),
        expect.anything()
      );
    });
  });

  it('hides cockpit-only sections in simple mode', async () => {
    mockViewMode = 'simple';
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

  it('shows all sections in cockpit mode', async () => {
    mockViewMode = 'cockpit';
    render(<SettingsPage />);
    
    // Wait for initial render/fetch to settle
    await screen.findByText('settings.appearance');

    expect(screen.getByText('settings.llmEngine')).toBeInTheDocument();
    expect(screen.getByText('settings.commerceEconomicBase')).toBeInTheDocument();
    expect(screen.getByText('settings.proMonthlyKcAllowance')).toBeInTheDocument();
    expect(screen.getByText('settings.proMonthlyKcAllowanceHelp')).toBeInTheDocument();
    expect(screen.getByText('settings.channelBridges')).toBeInTheDocument();
    expect(screen.getByText('settings.securityInfrastructure')).toBeInTheDocument();
    expect(screen.getByText('settings.featureFlags')).toBeInTheDocument();
  });

  it('hides Pro monthly KC allowance in simple mode', async () => {
    mockViewMode = 'simple';
    render(<SettingsPage />);

    await screen.findByText('settings.appearance');
    expect(screen.queryByText('settings.proMonthlyKcAllowance')).not.toBeInTheDocument();
  });

  it('persists pro_monthly_kc_allowance on blur', async () => {
    mockViewMode = 'cockpit';
    const mockAuthFetch = require('../lib/auth').authenticatedFetch;
    mockAuthFetch.mockImplementation(async (url: string, options: { method?: string }) => {
      if (url.includes('/api/v1/settings') && options?.method === 'PUT') {
        return { ok: true };
      }
      return { ok: true, json: async () => [] };
    });

    render(<SettingsPage />);
    await screen.findByText('settings.proMonthlyKcAllowance');

    // Label → SettingInput root (parent of field-row) → input. Do not use placeholder
    // (shared "0" with monthly spend; 0 = disabled semantics).
    const label = screen.getByText('settings.proMonthlyKcAllowance');
    const allowanceInput = label.closest('div')!.parentElement!.querySelector(
      'input'
    ) as HTMLInputElement;
    expect(allowanceInput).toBeTruthy();
    fireEvent.change(allowanceInput, { target: { value: '2500' } });
    fireEvent.blur(allowanceInput);

    await waitFor(() => {
      expect(mockAuthFetch).toHaveBeenCalledWith(
        expect.stringContaining('/api/v1/settings'),
        expect.objectContaining({
          method: 'PUT',
          body: JSON.stringify({
            key: 'pro_monthly_kc_allowance',
            value: '2500',
            category: 'commerce',
          }),
        })
      );
    });
  });

  it('handles MCP Config Manager interactions', async () => {
    // Mock the auth fetch for MCP config
    const mockAuthFetch = require('../lib/auth').authenticatedFetch;
    mockAuthFetch.mockImplementation(async (url: string, options: any) => {
      if (url.includes('/api/skills/mcp/config')) {
        if (options?.method === 'PUT') {
          return { ok: true };
        }
        return { ok: true, json: async () => ({ mcp_servers: { test: 1 } }) };
      }
      return { ok: true, json: async () => [] };
    });

    mockViewMode = 'cockpit';
    render(<SettingsPage />);

    // Wait for the appearance section to load (ensures main SettingsPage fetch is done)
    await screen.findByText('settings.appearance');

    // Wait for the MCP architecture section to load
    const mcpHeading = await screen.findByText('settings.mcpArchitecture');
    expect(mcpHeading).toBeInTheDocument();

    // Find the textarea and simulate typing
    const textareas = screen.getAllByRole('textbox');
    // The MCP config textarea is the only textarea in this component currently
    const mcpTextarea = textareas.find(ta => ta.tagName.toLowerCase() === 'textarea') as HTMLTextAreaElement;
    expect(mcpTextarea).toBeInTheDocument();
    
    // It should initially fetch and display the mocked JSON
    await waitFor(() => {
      expect(mcpTextarea.value).toContain('"test": 1');
    });

    // Change the value
    fireEvent.change(mcpTextarea, { target: { value: '{"mcp_servers": {"new": true}}' } });
    expect(mcpTextarea.value).toBe('{"mcp_servers": {"new": true}}');

    // Click the save button
    const saveButton = screen.getByText('settings.saveSyncTools');
    fireEvent.click(saveButton);

    // Wait for save message
    await screen.findByText(/settings.reloadedSuccessfully/);
    
    expect(mockAuthFetch).toHaveBeenCalledWith(
      expect.stringContaining('/api/skills/mcp/config'),
      expect.objectContaining({
        method: 'PUT',
        body: '{"mcp_servers": {"new": true}}'
      })
    );
  });

  it('handles MCP Config Manager errors (fetch and save)', async () => {
    // Suppress console.error for this test
    const consoleSpy = jest.spyOn(console, 'error').mockImplementation(() => {});
    
    const mockAuthFetch = require('../lib/auth').authenticatedFetch;
    mockAuthFetch.mockImplementation(async (url: string, options: any) => {
      if (url.includes('/api/skills/mcp/config')) {
        if (options?.method === 'PUT') {
          return { ok: false };
        }
        throw new Error('Fetch failed');
      }
      return { ok: true, json: async () => [] };
    });

    mockViewMode = 'cockpit';
    render(<SettingsPage />);
    await screen.findByText('settings.appearance');

    // Wait for the MCP architecture section
    const mcpHeading = await screen.findByText('settings.mcpArchitecture');
    expect(mcpHeading).toBeInTheDocument();

    // Verify console.error was called for the fetch failure
    expect(consoleSpy).toHaveBeenCalled();
    consoleSpy.mockRestore();

    // Try saving invalid JSON
    const textareas = screen.getAllByRole('textbox');
    const mcpTextarea = textareas.find(ta => ta.tagName.toLowerCase() === 'textarea') as HTMLTextAreaElement;
    
    fireEvent.change(mcpTextarea, { target: { value: 'invalid json' } });
    const saveButton = screen.getByText('settings.saveSyncTools');
    fireEvent.click(saveButton);

    // Expect invalid JSON error
    expect(await screen.findByText(/settings.invalidJson/)).toBeInTheDocument();

    // Now put valid json but API returns error
    fireEvent.change(mcpTextarea, { target: { value: '{"valid": true}' } });
    fireEvent.click(saveButton);

    // Expect save error
    expect(await screen.findByText(/settings.errorSaving/)).toBeInTheDocument();
  });

  it('handles Feature Toggle interactions', async () => {
    // Mock fetch to return some settings
    const mockAuthFetch = require('../lib/auth').authenticatedFetch;
    mockAuthFetch.mockImplementation(async (url: string, options: any) => {
      if (options?.method === 'PUT') return { ok: true };
      return { 
        ok: true, 
        json: async () => [{ key: 'enable_feature_x', value: 'false', category: 'features' }] 
      };
    });

    mockViewMode = 'cockpit';
    render(<SettingsPage />);
    
    // Wait for the settings to load
    await screen.findByText('settings.featureFlags');
    
    // Find the checkbox for the feature flag
    const checkboxes = screen.getAllByRole('checkbox');
    expect(checkboxes.length).toBeGreaterThan(0);
    
    // Click it to toggle
    fireEvent.click(checkboxes[0]);
    
    // Wait for the PUT request
    await waitFor(() => {
      expect(mockAuthFetch).toHaveBeenCalledWith(
        expect.stringContaining('/api/v1/settings'),
        expect.objectContaining({
          method: 'PUT',
          body: expect.stringContaining('"value":"true"')
        })
      );
    });
  });

  it('shows an error when saving setting fails', async () => {
    const mockAuthFetch = require('../lib/auth').authenticatedFetch;
    mockAuthFetch.mockImplementation(async (url: string, options: any) => {
      if (options?.method === 'PUT' && url.includes('/api/v1/settings')) {
        return { ok: false, text: async () => 'Database constraint violation' };
      }
      return { ok: true, json: async () => [] };
    });

    render(<SettingsPage />);
    await screen.findByText('settings.appearance');

    const aiNameInput = screen.getByPlaceholderText('settings.aiNamePlaceholder');
    fireEvent.change(aiNameInput, { target: { value: 'New Name' } });
    fireEvent.blur(aiNameInput);

    // Should show global error
    const errorAlert = await screen.findByText(/Failed to save setting: Database constraint violation/);
    expect(errorAlert).toBeInTheDocument();
  });

  it('handles test connection correctly', async () => {
    const mockAuthFetch = require('../lib/auth').authenticatedFetch;
    mockAuthFetch.mockImplementation(async (url: string, options: any) => {
      if (!options || (options.method === 'GET')) {
        return { ok: true, json: async () => [{ key: 'llm_api_url', value: 'http://success.com', category: 'llm' }] };
      }
      if (options?.method === 'POST') {
        return { ok: true, json: async () => ({ success: true, message: 'Connection successful' }) };
      }
      return { ok: true, json: async () => [] };
    });

    render(<SettingsPage />);
    await screen.findByText('settings.appearance');

    const testButtons = screen.getAllByText('settings.testLlmConnection');
    fireEvent.click(testButtons[0]);
    
    // Verify test connection hit the API with correct service name
    await waitFor(() => {
      expect(mockAuthFetch).toHaveBeenCalledWith(
        expect.stringContaining('/api/v1/settings/test'),
        expect.objectContaining({
          method: 'POST',
          body: expect.stringContaining('"service":"llm"')
        })
      );
    });
  });

  it('handles OllamaModelSelector fetch success and selection', async () => {
    const mockAuthFetch = require('../lib/auth').authenticatedFetch;
    mockAuthFetch.mockImplementation(async (url: string, options: any) => {
      if (url.includes('/api/v1/settings') && !url.includes('/test')) {
        return { 
          ok: true, 
          json: async () => [{ key: 'llm_provider', value: 'ollama', category: 'llm' }] 
        };
      }
      if (url.includes('/api/v1/ollama/models')) {
        return { 
          ok: true, 
          json: async () => ({ models: [{ name: 'llama3:latest' }, { name: 'mistral:7b' }] }) 
        };
      }
      if (options?.method === 'PUT') return { ok: true };
      return { ok: true, json: async () => [] };
    });

    render(<SettingsPage />);
    await screen.findByText('settings.appearance');

    // Wait for the dropdown to populate
    const selects = await screen.findAllByRole('combobox');
    const select = selects[1]; 
    
    await waitFor(() => {
      expect(select.children.length).toBeGreaterThanOrEqual(3);
    });

    fireEvent.change(select, { target: { value: 'llama3:latest' } });
    
    await waitFor(() => {
      expect(mockAuthFetch).toHaveBeenCalledWith(
        expect.stringContaining('/api/v1/settings'),
        expect.objectContaining({
          method: 'PUT',
          body: expect.stringContaining('"value":"llama3:latest"')
        })
      );
    });

    const refreshBtn = screen.getByText('settings.refresh');
    fireEvent.click(refreshBtn);
    
    await waitFor(() => {
      expect(mockAuthFetch).toHaveBeenCalledWith(expect.stringContaining('/api/v1/ollama/models'));
    });
  });

  it('handles OllamaModelSelector fetch errors', async () => {
    const mockAuthFetch = require('../lib/auth').authenticatedFetch;
    mockAuthFetch.mockImplementation(async (url: string, options: any) => {
      if (url.includes('/api/v1/settings') && !url.includes('/test')) {
        return { 
          ok: true, 
          json: async () => [{ key: 'llm_provider', value: 'ollama', category: 'llm' }] 
        };
      }
      if (url.includes('/api/v1/ollama/models')) {
        throw new Error('Network timeout');
      }
      return { ok: true, json: async () => [] };
    });

    render(<SettingsPage />);
    await screen.findByText('settings.appearance');

    const refreshBtn = screen.getByText('settings.refresh');
    fireEvent.click(refreshBtn);

    expect(await screen.findByText(/Connection error:/)).toBeInTheDocument();
  });

  it('handles Appearance and Avatar interactions', async () => {
    mockViewMode = 'cockpit';
    render(<SettingsPage />);
    await screen.findByText('settings.appearance');

    // Avatar Character
    const maleBtn = screen.getByText('settings.male');
    fireEvent.click(maleBtn);
    expect(mockSetCharacter).toHaveBeenCalledWith('male');

    // Avatar Style
    const tallerBtn = screen.getByText('settings.modernTaller');
    fireEvent.click(tallerBtn);
    expect(mockSetProportion).toHaveBeenCalledWith('taller');

    // Display Mode (e.g. lite)
    const liteBtn = screen.getByText(/lite/);
    fireEvent.click(liteBtn);
    expect(mockSetMode).toHaveBeenCalledWith('lite');
    
    // Test View Mode toggle
    const simpleBtn = screen.getByText('settings.viewMode_beginner');
    fireEvent.click(simpleBtn);
    // setViewMode is mocked inline in hooks/useViewMode mock
  });

  it('handles SecurityInfrastructure interactions (ToxicityConfig and OriginManager)', async () => {
    const mockAuthFetch = require('../lib/auth').authenticatedFetch;
    mockAuthFetch.mockImplementation(async (url: string, options: any) => {
      if (!options || (options.method === 'GET' && url.includes('/api/v1/settings'))) {
        return { 
          ok: true, 
          json: async () => [
            { key: 'csam_toxicity_forbidden_words', value: 'badword1,badword2', category: 'security' },
            { key: 'allowed_origins', value: 'http://localhost:3000', category: 'security' }
          ] 
        };
      }
      if (options?.method === 'PUT') return { ok: true };
      return { ok: true, json: async () => [] };
    });

    mockViewMode = 'cockpit';
    render(<SettingsPage />);
    await screen.findByText('settings.appearance');

    // Wait for the Security Infrastructure section
    expect(await screen.findByText('settings.securityInfrastructure')).toBeInTheDocument();

    // ToxicityConfig: Remove a word
    const badword1 = screen.getByText('badword1');
    const removeIcons = badword1.parentElement?.querySelectorAll('svg');
    if (removeIcons && removeIcons.length > 0) {
      fireEvent.click(removeIcons[0]);
    }

    await waitFor(() => {
      expect(mockAuthFetch).toHaveBeenCalledWith(
        expect.stringContaining('/api/v1/settings'),
        expect.objectContaining({
          method: 'PUT',
          body: expect.stringContaining('"value":"badword2"') // badword1 is removed
        })
      );
    });

    // ToxicityConfig: Add a word
    const toxicityInput = screen.getByPlaceholderText('settings.enterBannedWord');
    fireEvent.change(toxicityInput, { target: { value: 'badword3' } });
    fireEvent.keyDown(toxicityInput, { key: 'Enter', code: 'Enter' });
    
    await waitFor(() => {
      expect(mockAuthFetch).toHaveBeenCalledWith(
        expect.stringContaining('/api/v1/settings'),
        expect.objectContaining({
          method: 'PUT',
          body: expect.stringContaining('"value":"badword2,badword3"')
        })
      );
    });

    // OriginManager: Add an origin
    const originInput = screen.getByPlaceholderText('https://example.com');
    fireEvent.change(originInput, { target: { value: 'https://example.com' } });
    const originAddBtn = originInput.nextElementSibling as HTMLElement;
    fireEvent.click(originAddBtn);

    await waitFor(() => {
      expect(mockAuthFetch).toHaveBeenCalledWith(
        expect.stringContaining('/api/v1/settings'),
        expect.objectContaining({
          method: 'PUT',
          body: expect.stringContaining('"value":"http://localhost:3000,https://example.com"')
        })
      );
    });

    // SecretUpdater: Update API Secret
    const secretInput = screen.getByPlaceholderText('settings.enterNewSecret');
    fireEvent.change(secretInput, { target: { value: 'new-secret-123' } });
    fireEvent.keyDown(secretInput, { key: 'Enter', code: 'Enter' });

    await waitFor(() => {
      expect(mockAuthFetch).toHaveBeenCalledWith(
        expect.stringContaining('/api/health'),
        expect.objectContaining({
          headers: { 'Authorization': 'Bearer new-secret-123' }
        })
      );
    });
    // SecretUpdater: Update API Secret Error
    mockAuthFetch.mockImplementationOnce(async (url: string, options: any) => {
      return { ok: false, status: 401 };
    });
    fireEvent.change(secretInput, { target: { value: 'wrong-secret' } });
    fireEvent.keyDown(secretInput, { key: 'Enter', code: 'Enter' });

    expect(await screen.findByText(/settings.authFailed/)).toBeInTheDocument();

    // SecretUpdater: Update API Secret Network Error
    mockAuthFetch.mockImplementationOnce(async (url: string, options: any) => {
      throw new Error('Network error');
    });
    fireEvent.change(secretInput, { target: { value: 'network-error-secret' } });
    fireEvent.keyDown(secretInput, { key: 'Enter', code: 'Enter' });

    expect(await screen.findByText(/settings.connectionFailed/)).toBeInTheDocument();
  });

  it('renders Language Selector UI and triggers change', async () => {
    mockViewMode = 'simple'; // simpleでも表示されることを検証
    render(<SettingsPage />);
    await screen.findByText('settings.appearance');

    // 言語ラベルの存在確認
    expect(screen.getByText('settings.language')).toBeInTheDocument();

    // 🇺🇸 language.en ボタンと 🇯🇵 language.ja ボタンの存在確認
    const enButton = screen.getByText('🇺🇸 language.en');
    const jaButton = screen.getByText('🇯🇵 language.ja');
    expect(enButton).toBeInTheDocument();
    expect(jaButton).toBeInTheDocument();

    // 🇺🇸 language.en をクリックしたら mockSetLang が呼ばれるか検証
    fireEvent.click(enButton);
    expect(mockSetLang).toHaveBeenCalledWith('en');
  });
});

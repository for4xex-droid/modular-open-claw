import { render, screen, waitFor, fireEvent } from '@testing-library/react';
import McpDashboard from './McpDashboard';

// Mock matchMedia for framer-motion
Object.defineProperty(window, 'matchMedia', {
  writable: true,
  value: jest.fn().mockImplementation(query => ({
    matches: false,
    media: query,
    onchange: null,
    addListener: jest.fn(),
    removeListener: jest.fn(),
    addEventListener: jest.fn(),
    removeEventListener: jest.fn(),
    dispatchEvent: jest.fn(),
  })),
});

// Mock i18n — the `t` function resolves with defaultValue if given, otherwise maps known keys
const I18N_KEYS: Record<string, string> = {
  'page.mcpDashboard': 'MCP Server Management',
  'common.refresh': 'Refresh'
};

jest.mock('../i18n', () => ({
  useTranslation: () => ({
    t: (key: string, opts?: { defaultValue?: string }) => {
      if (opts?.defaultValue) return opts.defaultValue;
      return I18N_KEYS[key] || key;
    }
  })
}));

// Mock config
jest.mock('../config', () => ({
  API_BASE: 'http://localhost:3015'
}));

// Mock auth
jest.mock('../lib/auth', () => ({
  authenticatedFetch: jest.fn()
}));

import { authenticatedFetch } from '../lib/auth';
const mockFetch = authenticatedFetch as jest.MockedFunction<typeof authenticatedFetch>;

describe('McpDashboard', () => {
  beforeEach(() => {
    jest.clearAllMocks();
  });

  // ============================================================
  // Rendering Tests
  // ============================================================

  it('renders header with translated title', async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({ mcp_servers: {} })
    } as Response);

    render(<McpDashboard />);
    expect(screen.getByText('MCP Server Management')).toBeInTheDocument();
    expect(screen.getByText('Refresh')).toBeInTheDocument();
  });

  it('shows empty state when no servers exist', async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({ mcp_servers: {} })
    } as Response);

    render(<McpDashboard />);

    await waitFor(() => {
      expect(screen.getByText('No MCP Servers Registered')).toBeInTheDocument();
    });
    expect(screen.getByText('Add a server to grant Aiome new capabilities.')).toBeInTheDocument();
  });

  it('displays STDIO server with command and args', async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({
        mcp_servers: {
          "test-sqlite": {
            transport: "stdio",
            command: "npx",
            args: ["-y", "@modelcontextprotocol/server-sqlite"],
            env: { "DB_PATH": "/tmp/test.db" }
          }
        }
      })
    } as Response);

    render(<McpDashboard />);

    await waitFor(() => {
      expect(screen.getByText('test-sqlite')).toBeInTheDocument();
    });
    expect(screen.getByText('STDIO')).toBeInTheDocument();
    expect(screen.getByText('npx -y @modelcontextprotocol/server-sqlite')).toBeInTheDocument();
    expect(screen.getByText('DB_PATH')).toBeInTheDocument();
  });

  it('displays HTTP server with URL', async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({
        mcp_servers: {
          "stripe-api": {
            transport: "http",
            command: "",
            args: [],
            url: "https://api.stripe.com/mcp",
            headers: { "Authorization": "Bearer $STRIPE_KEY" }
          }
        }
      })
    } as Response);

    render(<McpDashboard />);

    await waitFor(() => {
      expect(screen.getByText('stripe-api')).toBeInTheDocument();
    });
    expect(screen.getByText('HTTP')).toBeInTheDocument();
    expect(screen.getByText('https://api.stripe.com/mcp')).toBeInTheDocument();
  });

  it('masks non-variable env values with ***', async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({
        mcp_servers: {
          "test-srv": {
            command: "test",
            args: [],
            env: { "SECRET_KEY": "actual_secret_value", "REF_VAR": "$MY_VAR" }
          }
        }
      })
    } as Response);

    render(<McpDashboard />);

    await waitFor(() => {
      expect(screen.getByText('test-srv')).toBeInTheDocument();
    });
    expect(screen.getByText('***')).toBeInTheDocument();
    expect(screen.getByText('$MY_VAR')).toBeInTheDocument();
  });

  // ============================================================
  // Error Handling Tests
  // ============================================================

  it('shows error banner when API fails', async () => {
    mockFetch.mockResolvedValueOnce({
      ok: false,
      status: 500
    } as Response);

    render(<McpDashboard />);

    await waitFor(() => {
      expect(screen.getByText('Failed to load MCP configuration')).toBeInTheDocument();
    });
  });

  it('shows error banner on network failure', async () => {
    mockFetch.mockRejectedValueOnce(new Error('Network error'));

    render(<McpDashboard />);

    await waitFor(() => {
      expect(screen.getByText('Connection error. Is the API server running?')).toBeInTheDocument();
    });
  });

  // ============================================================
  // Validation Tests (Security Critical)
  // ============================================================

  it('rejects empty server ID', async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({ mcp_servers: {} })
    } as Response);

    render(<McpDashboard />);

    await waitFor(() => {
      expect(screen.getByText('Add Server')).toBeInTheDocument();
    });

    fireEvent.click(screen.getByText('Add Server'));
    fireEvent.click(screen.getByText('Save & Restart'));

    expect(screen.getByText('Server ID is required.')).toBeInTheDocument();
  });

  it('rejects server ID with path traversal characters', async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({ mcp_servers: {} })
    } as Response);

    render(<McpDashboard />);

    await waitFor(() => {
      expect(screen.getByText('Add Server')).toBeInTheDocument();
    });

    fireEvent.click(screen.getByText('Add Server'));

    const input = screen.getByPlaceholderText('e.g. sqlite-mcp');
    fireEvent.change(input, { target: { value: '../../../etc/passwd' } });

    const cmdInput = screen.getByPlaceholderText('npx');
    fireEvent.change(cmdInput, { target: { value: 'node' } });

    fireEvent.click(screen.getByText('Save & Restart'));

    expect(screen.getByText('Server ID must contain only letters, numbers, hyphens, and underscores (max 64 chars).')).toBeInTheDocument();
  });

  it('rejects duplicate server ID', async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({ mcp_servers: { "existing-server": { command: "test", args: [] } } })
    } as Response);

    render(<McpDashboard />);

    await waitFor(() => {
      expect(screen.getByText('existing-server')).toBeInTheDocument();
    });

    fireEvent.click(screen.getByText('Add Server'));

    const input = screen.getByPlaceholderText('e.g. sqlite-mcp');
    fireEvent.change(input, { target: { value: 'existing-server' } });

    const cmdInput = screen.getByPlaceholderText('npx');
    fireEvent.change(cmdInput, { target: { value: 'node' } });

    fireEvent.click(screen.getByText('Save & Restart'));

    expect(screen.getByText('A server with this ID already exists.')).toBeInTheDocument();
  });

  it('rejects javascript: URL scheme (XSS prevention)', async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({ mcp_servers: {} })
    } as Response);

    render(<McpDashboard />);

    await waitFor(() => {
      expect(screen.getByText('Add Server')).toBeInTheDocument();
    });

    fireEvent.click(screen.getByText('Add Server'));

    // Switch to HTTP transport
    const select = screen.getByDisplayValue(/STDIO/);
    fireEvent.change(select, { target: { value: 'http' } });

    const urlInput = screen.getByPlaceholderText('https://example.com/mcp');
    fireEvent.change(urlInput, { target: { value: 'javascript:alert(1)' } });

    const idInput = screen.getByPlaceholderText('e.g. sqlite-mcp');
    fireEvent.change(idInput, { target: { value: 'evil-server' } });

    fireEvent.click(screen.getByText('Save & Restart'));

    expect(screen.getByText('A valid HTTP or HTTPS URL is required for HTTP transport.')).toBeInTheDocument();
  });

  it('rejects STDIO transport with empty command', async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({ mcp_servers: {} })
    } as Response);

    render(<McpDashboard />);

    await waitFor(() => {
      expect(screen.getByText('Add Server')).toBeInTheDocument();
    });

    fireEvent.click(screen.getByText('Add Server'));

    const idInput = screen.getByPlaceholderText('e.g. sqlite-mcp');
    fireEvent.change(idInput, { target: { value: 'valid-id' } });

    fireEvent.click(screen.getByText('Save & Restart'));

    expect(screen.getByText('Command is required for STDIO transport.')).toBeInTheDocument();
  });

  // ============================================================
  // Successful Add Flow
  // ============================================================

  it('calls API with correct payload on successful add', async () => {
    // Track the saved config to simulate server persistence
    let savedConfig: Record<string, unknown> = {};

    mockFetch.mockImplementation(async (_url: string, opts?: RequestInit) => {
      if (opts?.method === 'POST') {
        // Capture what was sent
        savedConfig = JSON.parse(opts.body as string);
        return { ok: true } as Response;
      }
      // GET — return whatever was last saved (or empty initially)
      const data = Object.keys(savedConfig).length > 0 ? savedConfig : { mcp_servers: {} };
      return { ok: true, json: async () => data } as Response;
    });

    render(<McpDashboard />);

    await waitFor(() => {
      expect(screen.getByText('Add Server')).toBeInTheDocument();
    });

    fireEvent.click(screen.getByText('Add Server'));

    fireEvent.change(screen.getByPlaceholderText('e.g. sqlite-mcp'), { target: { value: 'new-server' } });
    fireEvent.change(screen.getByPlaceholderText('npx'), { target: { value: 'node' } });
    fireEvent.change(screen.getByPlaceholderText('-y, @modelcontextprotocol/server-sqlite'), { target: { value: 'server.js' } });

    // Click save
    fireEvent.click(screen.getByText('Save & Restart'));

    // Wait for the new server to appear in the card list
    await waitFor(() => {
      expect(screen.getByText('new-server')).toBeInTheDocument();
    }, { timeout: 3000 });

    // Verify POST payload was correct
    const postCall = mockFetch.mock.calls.find(
      (call) => call[1]?.method === 'POST'
    );
    expect(postCall).toBeDefined();
    expect(postCall![0]).toBe('http://localhost:3015/api/skills/mcp/config');
    expect(savedConfig).toHaveProperty('mcp_servers');
    const servers = (savedConfig as { mcp_servers: Record<string, { command: string; args: string[] }> }).mcp_servers;
    expect(servers['new-server']).toBeDefined();
    expect(servers['new-server'].command).toBe('node');
    expect(servers['new-server'].args).toEqual(['server.js']);
  });
});

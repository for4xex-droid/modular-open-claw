/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

import { render, screen } from '@testing-library/react';
import AgentConsole from './AgentConsole';

// Mock useAgentChat
jest.mock('../hooks/AgentChatProvider', () => ({
  useAgentChat: () => ({
    history: [],
    input: '',
    isTyping: false,
    streamingText: '',
    status: 'IDLE',
    autoTts: false,
    relevantKarma: null,
    relevantKarmaData: null,
    activeKnowledge: null,
    setInput: jest.fn(),
    sendMessage: jest.fn(),
    setAutoTts: jest.fn(),
    handleFeedback: jest.fn(),
  })
}));

// Mock useTranslation
jest.mock('../i18n', () => ({
  useTranslation: () => ({ t: (key: string) => key })
}));

// Mock useWorkspacePersona
jest.mock('../hooks/useWorkspacePersona', () => ({
  useWorkspacePersona: () => ({ mode: 'agentic' })
}));

// Mock child components that might import config.ts directly
jest.mock('./common/ProofPowerIndicator', () => ({
  ProofPowerIndicator: () => <div data-testid="proof-power" />
}));
jest.mock('./common/TokenSavingsIndicator', () => ({
  TokenSavingsIndicator: () => <div data-testid="token-savings" />
}));
jest.mock('./common/ActivityFeed', () => ({
  ActivityFeed: () => <div data-testid="activity-feed" />
}));
jest.mock('./A2uiRenderer', () => ({
  A2uiRenderer: () => <div data-testid="a2ui-renderer" />
}));
jest.mock('../config', () => ({
  API_BASE: 'http://localhost:3015'
}));
jest.mock('../lib/auth', () => ({
  authenticatedFetch: jest.fn().mockImplementation(() => Promise.resolve({
    ok: true, json: () => Promise.resolve([])
  })),
  getAuthToken: jest.fn().mockReturnValue('test-token'),
}));

// Mock ReactMarkdown
jest.mock('react-markdown', () => {
  return ({ children }: { children: React.ReactNode }) => (
    <div data-testid="react-markdown-mock">{children}</div>
  );
});

jest.mock('rehype-sanitize', () => () => {});


describe('AgentConsole Slash Commands UI', () => {
  beforeAll(() => {
    window.HTMLElement.prototype.scrollIntoView = jest.fn();
  });

  it('shows slash command suggestions when typing /', () => {
    // Arrange
    // We override the useAgentChat mock specifically for this test to simulate typing '/'
    const setInputMock = jest.fn();
    // @ts-expect-error
    jest.spyOn(require('../hooks/AgentChatProvider'), 'useAgentChat').mockReturnValue({
      history: [],
      input: '/',
      isTyping: false,
      streamingText: '',
      status: 'IDLE',
      autoTts: false,
      relevantKarma: null,
      relevantKarmaData: null,
      activeKnowledge: null,
      setInput: setInputMock,
      sendMessage: jest.fn(),
      setAutoTts: jest.fn(),
      handleFeedback: jest.fn(),
    });

    render(<AgentConsole />);

    // Act & Assert
    // The suggest menu should appear
    expect(screen.getByText('Voice Store')).toBeInTheDocument();
    expect(screen.getByText('Treasure Box')).toBeInTheDocument();
    expect(screen.getByText('LoRA Market')).toBeInTheDocument();
  });

  it('renders AI messages using ReactMarkdown', () => {
    // Arrange
    // @ts-expect-error
    jest.spyOn(require('../hooks/AgentChatProvider'), 'useAgentChat').mockReturnValue({
      history: [
        { id: '1', role: 'aiome', content: '# Hello World\nThis is a **markdown** test.', timestamp: Date.now() }
      ],
      input: '',
      isTyping: false,
      streamingText: '',
      status: 'IDLE',
      autoTts: false,
      relevantKarma: null,
      relevantKarmaData: null,
      activeKnowledge: null,
      setInput: jest.fn(),
      sendMessage: jest.fn(),
      setAutoTts: jest.fn(),
      handleFeedback: jest.fn(),
    });

    // Act
    render(<AgentConsole />);

    // Assert
    // If ReactMarkdown is used, we should see our mock with the data-testid
    expect(screen.getByTestId('react-markdown-mock')).toBeInTheDocument();
    expect(screen.getByText(/Hello World/)).toBeInTheDocument();
    expect(screen.getByText(/This is a \*\*markdown\*\* test/)).toBeInTheDocument();
  });

  it('calculates blueprint ROI based on actual ledger audit tasks (TDD)', async () => {
    // Arrange
    // Override mocks for this test
    // @ts-expect-error
    jest.spyOn(require('../hooks/useWorkspacePersona'), 'useWorkspacePersona').mockReturnValue({
      mode: 'agency'
    });

    const mockArtifacts = [
      { id: 'bp-1', category: 'Blueprint', name: 'Auto Backup' },
      { id: 'bp-2', category: 'blueprint', name: 'Sync Tasks' },
      { id: 'art-3', category: 'Report', name: 'Some Report' } // Non-blueprint
    ];

    const mockLedger = [
      { id: 1, record_id: 'bp-1', table_name: 'artifacts', operation: 'EXECUTE', timestamp: '2026-06-10' },
      { id: 2, record_id: 'bp-1', table_name: 'artifacts', operation: 'EXECUTE', timestamp: '2026-06-10' },
      { id: 3, record_id: 'other', table_name: 'artifacts', operation: 'EXECUTE', timestamp: '2026-06-10' }
    ];

    const mockFetch = require('../lib/auth').authenticatedFetch as jest.Mock;
    mockFetch.mockImplementation((url: string) => {
      if (url.includes('/api/artifacts')) {
        return Promise.resolve({ ok: true, json: () => Promise.resolve(mockArtifacts) });
      }
      if (url.includes('/api/v1/audit/ledger')) {
        return Promise.resolve({ ok: true, json: () => Promise.resolve(mockLedger) });
      }
      return Promise.resolve({ ok: true, json: () => Promise.resolve([]) });
    });

    // Act
    render(<AgentConsole />);

    // Switch to Automations tab
    const automationsBtn = screen.getByText('agent.automationsTab');
    automationsBtn.click();

    // Assert overall stats
    expect(await screen.findByText('3')).toBeInTheDocument(); // Tasks Executed
    expect(await screen.findByText('$15')).toBeInTheDocument(); // Estimated Savings (3 * $5)
    expect(await screen.findByText('2')).toBeInTheDocument(); // Active Blueprints (bp-1 and bp-2)

    // Assert ROI calculations
    // bp-1 (Auto Backup) has 2 executions: $100 + 2 * $5 = $110
    expect(await screen.findByText('+$110/mo')).toBeInTheDocument();
    // bp-2 (Sync Tasks) has 0 executions: $100 + 0 * $5 = $100
    expect(await screen.findByText('+$100/mo')).toBeInTheDocument();
  });

  it('sets fallback stats on API fetch failure to prevent infinite retries', async () => {
    // Arrange
    // @ts-expect-error
    jest.spyOn(require('../hooks/useWorkspacePersona'), 'useWorkspacePersona').mockReturnValue({
      mode: 'agency'
    });

    const mockFetch = require('../lib/auth').authenticatedFetch as jest.Mock;
    mockFetch.mockImplementation(() => Promise.reject(new Error('Network Error')));

    // Act
    render(<AgentConsole />);

    // Switch to Automations tab
    const automationsBtn = screen.getByText('agent.automationsTab');
    automationsBtn.click();

    // Assert fallback empty UI is rendered instead of loading forever
    expect(await screen.findByText('agent.noBlueprintInstances')).toBeInTheDocument();
  });
});


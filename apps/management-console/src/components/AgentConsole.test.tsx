
import { render, screen } from '@testing-library/react';
import AgentConsole from './AgentConsole';

// Mock useAgentChat
jest.mock('../hooks/useAgentChat', () => ({
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

describe('AgentConsole Slash Commands UI', () => {
  beforeAll(() => {
    window.HTMLElement.prototype.scrollIntoView = jest.fn();
  });

  it('shows slash command suggestions when typing /', () => {
    // Arrange
    // We override the useAgentChat mock specifically for this test to simulate typing '/'
    const setInputMock = jest.fn();
    // @ts-expect-error
    jest.spyOn(require('../hooks/useAgentChat'), 'useAgentChat').mockReturnValue({
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
});

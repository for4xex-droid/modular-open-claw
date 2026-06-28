/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import { render, screen, waitFor } from '@testing-library/react';
import PromptStatsView from './PromptStatsView';
import { authenticatedFetch } from '../lib/auth';

jest.mock('../config', () => ({
  API_BASE: 'http://localhost:8080'
}));

jest.mock('../lib/auth', () => ({
  authenticatedFetch: jest.fn()
}));

jest.mock('../i18n', () => ({
  useTranslation: () => ({
    t: (key: string) => key
  })
}));

describe('PromptStatsView', () => {
  beforeEach(() => {
    jest.clearAllMocks();
  });

  it('renders and displays stats data', async () => {
    const mockStats = {
      providers: [
        {
          provider: 'Ollama',
          model: 'llama3:latest',
          total_calls: 42,
          average_latency_ms: 120.5,
          total_cost_usd: 0.0,
          cache_hit_rate: 15.0
        }
      ]
    };

    (authenticatedFetch as jest.Mock).mockResolvedValue({
      ok: true,
      json: () => Promise.resolve(mockStats)
    });

    render(<PromptStatsView />);

    expect(screen.getByText('promptStats.title')).toBeInTheDocument();
    
    await waitFor(() => {
      expect(screen.getByText('Ollama - llama3:latest')).toBeInTheDocument();
      expect(screen.getByText('42')).toBeInTheDocument();
      expect(screen.getByText('120.5 ms')).toBeInTheDocument();
    });
  });

  it('displays error on failed fetch', async () => {
    (authenticatedFetch as jest.Mock).mockResolvedValue({
      ok: false
    });

    render(<PromptStatsView />);

    await waitFor(() => {
      expect(screen.getByText('promptStats.loadFailed')).toBeInTheDocument();
    });
  });
});

/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import { render, screen, waitFor, act } from '@testing-library/react';
import SeoPulseView from './SeoPulseView';
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

jest.mock('../hooks/useSystemVitality', () => ({
  useSystemVitality: () => ({
    events: []
  })
}));

jest.mock('./ui/LoadingState', () => ({
  LoadingState: () => <div data-testid="loading-state">loading</div>,
}));

describe('SeoPulseView', () => {
  beforeEach(() => {
    jest.clearAllMocks();
    global.fetch = jest.fn();
  });

  it('renders and displays GEO status and history', async () => {
    // Mock global fetch for bootstrap status
    (global.fetch as jest.Mock).mockResolvedValue({
      ok: true,
      json: () => Promise.resolve({
        sidecar_status: [{ name: 'geo-optimizer', status: 'ok' }]
      })
    });

    // Mock authenticatedFetch for quality gate history
    (authenticatedFetch as jest.Mock).mockResolvedValue({
      ok: true,
      json: () => Promise.resolve([
        {
          id: '1',
          job_id: 'job-1',
          score: 85,
          passed: true,
          conductor: 'seo-audit-v1',
          created_at: new Date().toISOString()
        }
      ])
    });

    render(<SeoPulseView />);

    expect(screen.getByText('GEO Pulse')).toBeInTheDocument();

    await waitFor(() => {
      expect(screen.getByText('ok')).toBeInTheDocument();
      expect(screen.getByText('seo-audit-v1')).toBeInTheDocument();
      expect(screen.getByText('Score: 85')).toBeInTheDocument();
    });
  });

  it('updates viseme display on custom event', async () => {
    (global.fetch as jest.Mock).mockResolvedValue({
      ok: true,
      json: () => Promise.resolve({ sidecar_status: [] })
    });
    (authenticatedFetch as jest.Mock).mockResolvedValue({
      ok: true,
      json: () => Promise.resolve([])
    });

    render(<SeoPulseView />);

    await waitFor(() => {
      expect(screen.queryByTestId('loading-state')).not.toBeInTheDocument();
    });

    expect(screen.getByText('SIL')).toBeInTheDocument();

    // Trigger custom event
    act(() => {
      window.dispatchEvent(
        new CustomEvent('aiome_viseme_played', { detail: { viseme: 'AA' } })
      );
    });

    expect(screen.getByText('AA')).toBeInTheDocument();
  });
});

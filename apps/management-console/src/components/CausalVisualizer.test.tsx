/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import CausalVisualizer from './CausalVisualizer';
import { authenticatedFetch } from '../lib/auth';
import {
  A2UI_NAVIGATE_EVENT,
  CAUSAL_JOB_ID_STORAGE_KEY,
  dispatchA2uiNavigate,
} from '../lib/a2uiTabs';

jest.mock('../config', () => ({
  API_BASE: 'http://localhost:8080'
}));

jest.mock('../lib/auth', () => ({
  authenticatedFetch: jest.fn()
}));

jest.mock('vis-network', () => ({
  Network: jest.fn().mockImplementation(() => ({
    on: jest.fn(),
    destroy: jest.fn(),
    getScale: () => 1,
    moveTo: jest.fn(),
    fit: jest.fn()
  }))
}));

jest.mock('vis-data', () => ({
  DataSet: jest.fn().mockImplementation((items) => items)
}));

jest.mock('../i18n', () => {
  const t = (key: string) => key;
  return {
    useTranslation: () => ({ t }),
  };
});

const mockFetch = authenticatedFetch as jest.MockedFunction<typeof authenticatedFetch>;

describe('CausalVisualizer', () => {
  beforeEach(() => {
    jest.clearAllMocks();
    sessionStorage.clear();
  });

  it('renders search input and title', () => {
    render(<CausalVisualizer />);
    expect(screen.getByText('causal.title')).toBeInTheDocument();
    expect(screen.getByPlaceholderText('causal.jobIdPlaceholder')).toBeInTheDocument();
  });

  it('shows empty state message when no job is loaded', () => {
    render(<CausalVisualizer />);
    expect(screen.getByText('causal.enterJobId')).toBeInTheDocument();
  });

  it('updates job id input', () => {
    render(<CausalVisualizer />);
    const input = screen.getByPlaceholderText('causal.jobIdPlaceholder') as HTMLInputElement;
    fireEvent.change(input, { target: { value: 'job-123' } });
    expect(input.value).toBe('job-123');
  });

  it('places controls overlay inside the relative-positioned graph area to prevent title overlap', () => {
    render(<CausalVisualizer />);
    const graphArea = screen.getByTestId('causal-graph-area');
    const controlsOverlay = screen.getByTestId('causal-controls-overlay');
    expect(graphArea).toContainElement(controlsOverlay);
  });

  it('Positive: fetches trajectory on valid Enter and clears empty state', async () => {
    mockFetch.mockImplementation((url: string) => {
      if (url.includes('/diagnosis')) {
        return Promise.resolve({ ok: false } as Response);
      }
      return Promise.resolve({
        ok: true,
        json: async () => ({
          nodes: [
            {
              id: '1',
              step: {
                step_id: 1,
                action: 'plan',
                input: {},
                output: {},
                timestamp: '2026-07-22T00:00:00Z',
                step_category: 'Planning',
              },
            },
          ],
          edges: [],
        }),
      } as Response);
    });

    render(<CausalVisualizer />);
    const input = screen.getByPlaceholderText('causal.jobIdPlaceholder');
    fireEvent.change(input, { target: { value: 'job_abc' } });
    fireEvent.keyDown(input, { key: 'Enter' });

    await waitFor(() => {
      expect(mockFetch).toHaveBeenCalledWith('http://localhost:8080/api/v1/trajectory/job_abc');
    });
    await waitFor(() => {
      expect(screen.queryByText('causal.enterJobId')).not.toBeInTheDocument();
    });
  });

  it('Negative: invalid job id shows causal.invalidJobId and does not fetch', async () => {
    render(<CausalVisualizer />);
    const input = screen.getByPlaceholderText('causal.jobIdPlaceholder');
    fireEvent.change(input, { target: { value: 'bad id!' } });
    fireEvent.keyDown(input, { key: 'Enter' });

    expect(await screen.findByText('causal.invalidJobId')).toBeInTheDocument();
    expect(mockFetch).not.toHaveBeenCalled();
  });

  it('Negative: non-ok trajectory response shows causal.fetchFailed', async () => {
    mockFetch.mockResolvedValue({ ok: false, status: 404 } as Response);

    render(<CausalVisualizer />);
    const input = screen.getByPlaceholderText('causal.jobIdPlaceholder');
    fireEvent.change(input, { target: { value: 'missing_job' } });
    fireEvent.keyDown(input, { key: 'Enter' });

    expect(await screen.findByText('causal.fetchFailed')).toBeInTheDocument();
  });

  it('deep-link: a2ui-navigate with jobId triggers fetch', async () => {
    mockFetch.mockImplementation((url: string) => {
      if (url.includes('/diagnosis')) {
        return Promise.resolve({ ok: false } as Response);
      }
      return Promise.resolve({
        ok: true,
        json: async () => ({ nodes: [], edges: [] }),
      } as Response);
    });

    render(<CausalVisualizer />);
    window.dispatchEvent(
      new CustomEvent(A2UI_NAVIGATE_EVENT, { detail: { tab: 'causal', jobId: 'from_event' } })
    );

    await waitFor(() => {
      expect(mockFetch).toHaveBeenCalledWith('http://localhost:8080/api/v1/trajectory/from_event');
    });
  });

  it('deep-link: sessionStorage stash is consumed on mount', async () => {
    sessionStorage.setItem(CAUSAL_JOB_ID_STORAGE_KEY, 'from_storage');
    mockFetch.mockImplementation((url: string) => {
      if (url.includes('/diagnosis')) {
        return Promise.resolve({ ok: false } as Response);
      }
      return Promise.resolve({
        ok: true,
        json: async () => ({ nodes: [], edges: [] }),
      } as Response);
    });

    render(<CausalVisualizer />);

    await waitFor(() => {
      expect(mockFetch).toHaveBeenCalledWith('http://localhost:8080/api/v1/trajectory/from_storage');
    });
    expect(sessionStorage.getItem(CAUSAL_JOB_ID_STORAGE_KEY)).toBeNull();
  });

  it('dispatchA2uiNavigate stashes jobId for dual-mount handoff', () => {
    dispatchA2uiNavigate({ tab: 'causal', jobId: 'handoff' });
    expect(sessionStorage.getItem(CAUSAL_JOB_ID_STORAGE_KEY)).toBe('handoff');
  });
});

/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

import { render, screen, waitFor } from '@testing-library/react';
import '@testing-library/jest-dom';
import { SubscriptionProvider, useSubscriptionStatus } from './useSubscriptionStatus';
import { authenticatedFetch } from '../lib/auth';

jest.unmock('./useSubscriptionStatus');

jest.mock('./useAgentIdentity', () => ({
  useAgentIdentity: jest.fn(() => ({ agentId: 'agent-001' })),
}));

jest.mock('../lib/auth', () => ({
  authenticatedFetch: jest.fn(),
}));

jest.mock('../config', () => ({
  API_BASE: 'http://localhost:3000',
}));

function Probe() {
  const { status, isLoading } = useSubscriptionStatus();
  return (
    <div>
      <div data-testid="status">{status ?? 'null'}</div>
      <div data-testid="loading">{isLoading ? 'yes' : 'no'}</div>
    </div>
  );
}

describe('SubscriptionProvider visibility refresh', () => {
  beforeEach(() => {
    jest.clearAllMocks();
    (authenticatedFetch as jest.Mock).mockResolvedValue({
      ok: true,
      json: async () => 'active',
    });
  });

  it('starts fail-closed (isLoading=true) until the first fetch settles', async () => {
    let resolveFetch!: (value: { ok: boolean; json: () => Promise<string> }) => void;
    (authenticatedFetch as jest.Mock).mockImplementation(
      () =>
        new Promise((resolve) => {
          resolveFetch = resolve;
        }),
    );

    render(
      <SubscriptionProvider>
        <Probe />
      </SubscriptionProvider>,
    );

    expect(screen.getByTestId('loading')).toHaveTextContent('yes');

    resolveFetch({
      ok: true,
      json: async () => 'none',
    });

    await waitFor(() => {
      expect(screen.getByTestId('loading')).toHaveTextContent('no');
      expect(screen.getByTestId('status')).toHaveTextContent('none');
    });
  });

  it('refreshes subscription when tab becomes visible', async () => {
    Object.defineProperty(document, 'visibilityState', {
      configurable: true,
      get: () => 'visible',
    });

    render(
      <SubscriptionProvider>
        <Probe />
      </SubscriptionProvider>,
    );

    await waitFor(() => {
      expect(authenticatedFetch).toHaveBeenCalledTimes(1);
    });

    document.dispatchEvent(new Event('visibilitychange'));

    await waitFor(() => {
      expect(authenticatedFetch).toHaveBeenCalledTimes(2);
    });
  });

  it('coalesces overlapping refresh into a follow-up fetch', async () => {
    Object.defineProperty(document, 'visibilityState', {
      configurable: true,
      get: () => 'visible',
    });

    let resolveFirst!: (value: { ok: boolean; json: () => Promise<string>; text: () => Promise<string> }) => void;
    const first = new Promise<{ ok: boolean; json: () => Promise<string>; text: () => Promise<string> }>((resolve) => {
      resolveFirst = resolve;
    });
    (authenticatedFetch as jest.Mock)
      .mockImplementationOnce(() => first)
      .mockResolvedValueOnce({
        ok: true,
        json: async () => 'trialing',
        text: async () => '',
      });

    render(
      <SubscriptionProvider>
        <Probe />
      </SubscriptionProvider>,
    );

    await waitFor(() => {
      expect(authenticatedFetch).toHaveBeenCalledTimes(1);
    });

    document.dispatchEvent(new Event('visibilitychange'));
    expect(authenticatedFetch).toHaveBeenCalledTimes(1);

    resolveFirst({
      ok: true,
      json: async () => 'active',
      text: async () => '',
    });

    await waitFor(() => {
      expect(authenticatedFetch).toHaveBeenCalledTimes(2);
    });
  });
});

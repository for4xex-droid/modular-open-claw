/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

import { render, waitFor, act } from '@testing-library/react';
import '@testing-library/jest-dom';
import { CheckoutSuccess } from './CheckoutSuccess';
import { useSubscriptionStatus } from '../../hooks/useSubscriptionStatus';

jest.mock('../../hooks/useSubscriptionStatus', () => ({
  useSubscriptionStatus: jest.fn(),
}));

jest.mock('../../i18n', () => ({
  useTranslation: () => ({
    t: (key: string) => key,
  }),
}));

const mockUseSubscriptionStatus = useSubscriptionStatus as jest.Mock;

describe('CheckoutSuccess short polling', () => {
  beforeEach(() => {
    jest.useFakeTimers();
    jest.clearAllMocks();
  });

  afterEach(() => {
    jest.useRealTimers();
  });

  it('polls refresh up to 3 times after mount while not Pro', async () => {
    const refresh = jest.fn().mockResolvedValue(true);
    mockUseSubscriptionStatus.mockReturnValue({
      status: 'none',
      isPro: false,
      isLoading: false,
      error: null,
      refresh,
    });

    render(<CheckoutSuccess onGoHome={jest.fn()} />);

    // Mount refresh
    await waitFor(() => {
      expect(refresh).toHaveBeenCalledTimes(1);
    });

    await act(async () => {
      jest.advanceTimersByTime(2000);
    });
    await waitFor(() => {
      expect(refresh).toHaveBeenCalledTimes(2);
    });

    await act(async () => {
      jest.advanceTimersByTime(2000);
    });
    await waitFor(() => {
      expect(refresh).toHaveBeenCalledTimes(3);
    });

    await act(async () => {
      jest.advanceTimersByTime(2000);
    });
    await waitFor(() => {
      expect(refresh).toHaveBeenCalledTimes(4);
    });

    await act(async () => {
      jest.advanceTimersByTime(4000);
    });
    expect(refresh).toHaveBeenCalledTimes(4);
  });

  it('stops polling once isPro becomes true', async () => {
    const refresh = jest.fn().mockResolvedValue(true);
    mockUseSubscriptionStatus.mockReturnValue({
      status: 'none',
      isPro: false,
      isLoading: false,
      error: null,
      refresh,
    });

    const { rerender } = render(<CheckoutSuccess onGoHome={jest.fn()} />);

    await waitFor(() => {
      expect(refresh).toHaveBeenCalledTimes(1);
    });

    mockUseSubscriptionStatus.mockReturnValue({
      status: 'active',
      isPro: true,
      isLoading: false,
      error: null,
      refresh,
    });
    rerender(<CheckoutSuccess onGoHome={jest.fn()} />);

    await act(async () => {
      jest.advanceTimersByTime(6000);
    });

    // Remount effect may call refresh once more; no 2s poll chain after Pro.
    expect(refresh.mock.calls.length).toBeLessThanOrEqual(2);
  });
});

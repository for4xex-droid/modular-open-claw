/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

import { render, screen } from '@testing-library/react';
import '@testing-library/jest-dom';
import { LockedOverlay } from './LockedOverlay';
import { useSubscriptionStatus } from '../../hooks/useSubscriptionStatus';

jest.mock('../../hooks/useSubscriptionStatus', () => ({
  useSubscriptionStatus: jest.fn(),
  openProUpgradeModal: jest.fn(),
}));

jest.mock('../../i18n', () => ({
  useTranslation: () => ({
    t: (key: string) => key,
  }),
}));

const mockUseSubscriptionStatus = useSubscriptionStatus as jest.Mock;

describe('LockedOverlay subscription gating', () => {
  it('keeps feature blocked while subscription is loading (fail-closed)', () => {
    mockUseSubscriptionStatus.mockReturnValue({ isPro: false, isLoading: true });

    render(
      <LockedOverlay featureNameKey="pro.featureBuzz">
        <button type="button">secret-action</button>
      </LockedOverlay>,
    );

    expect(screen.queryByRole('button', { name: 'pro.unlockHint' })).not.toBeInTheDocument();
    expect(screen.getByLabelText('common.loading')).toBeInTheDocument();
  });

  it('renders children unlocked when Pro', () => {
    mockUseSubscriptionStatus.mockReturnValue({ isPro: true, isLoading: false });

    render(
      <LockedOverlay featureNameKey="pro.featureBuzz">
        <button type="button">secret-action</button>
      </LockedOverlay>,
    );

    expect(screen.getByRole('button', { name: 'secret-action' })).toBeInTheDocument();
    expect(screen.queryByLabelText('common.loading')).not.toBeInTheDocument();
  });

  it('shows upgrade CTA when Free and loaded', () => {
    mockUseSubscriptionStatus.mockReturnValue({ isPro: false, isLoading: false });

    render(
      <LockedOverlay featureNameKey="pro.featureBuzz">
        <button type="button">secret-action</button>
      </LockedOverlay>,
    );

    expect(screen.getByRole('button', { name: 'pro.unlockHint' })).toBeInTheDocument();
  });
});

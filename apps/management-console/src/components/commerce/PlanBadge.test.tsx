/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

import { render, screen, fireEvent } from '@testing-library/react';
import '@testing-library/jest-dom';
import { PlanBadge } from './PlanBadge';
import { useSubscriptionStatus, openProUpgradeModal } from '../../hooks/useSubscriptionStatus';
import { useCheckoutSession } from '../../hooks/useCheckoutSession';

jest.mock('../../hooks/useSubscriptionStatus', () => ({
  useSubscriptionStatus: jest.fn(),
  openProUpgradeModal: jest.fn(),
}));

jest.mock('../../hooks/useCheckoutSession', () => ({
  useCheckoutSession: jest.fn(),
}));

jest.mock('../../hooks/useAgentIdentity', () => ({
  useAgentIdentity: jest.fn(() => ({ agentId: 'agent-001' })),
}));

jest.mock('../../i18n', () => ({
  useTranslation: () => ({
    t: (key: string) => {
      const map: Record<string, string> = {
        'pro.badgeFree': 'Free · Upgrade',
        'pro.badgeFreeTooltip': 'Click to explore Pro',
        'pro.badgeProTooltip': 'Pro active',
      };
      return map[key] ?? key;
    },
  }),
}));

jest.mock('../../config', () => ({
  STRIPE_PRICE_ID: 'price_test',
}));

const mockUseSubscriptionStatus = useSubscriptionStatus as jest.Mock;
const mockUseCheckoutSession = useCheckoutSession as jest.Mock;

describe('PlanBadge', () => {
  beforeEach(() => {
    jest.clearAllMocks();
    mockUseCheckoutSession.mockReturnValue({
      handlePortal: jest.fn(),
      isPortalLoading: false,
    });
  });

  it('opens upgrade modal when Free badge clicked', () => {
    mockUseSubscriptionStatus.mockReturnValue({ isPro: false, isLoading: false });

    render(<PlanBadge />);
    fireEvent.click(screen.getByText('Free · Upgrade'));

    expect(openProUpgradeModal).toHaveBeenCalled();
  });

  it('opens customer portal when Pro badge clicked', () => {
    const handlePortal = jest.fn();
    mockUseSubscriptionStatus.mockReturnValue({ isPro: true, isLoading: false });
    mockUseCheckoutSession.mockReturnValue({ handlePortal, isPortalLoading: false });

    render(<PlanBadge />);
    fireEvent.click(screen.getByText('Pro'));

    expect(handlePortal).toHaveBeenCalled();
  });
});

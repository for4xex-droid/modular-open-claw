/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

import React from 'react';
import { render, screen, fireEvent } from '@testing-library/react';
import '@testing-library/jest-dom';
import { ProUpgradeModal } from './ProUpgradeModal';
import { useCheckoutSession } from '../../hooks/useCheckoutSession';

// mock custom hook
jest.mock('../../hooks/useCheckoutSession');

jest.mock('../../i18n', () => ({
    useTranslation: () => ({
        t: (key: string, opts?: { feature?: string }) => {
            const map: Record<string, string> = {
                'pro.modalTitle': 'Unlock Aiome Pro',
                'pro.modalSubtitle': 'Supercharge your AI OS.',
                'pro.cancel': 'Cancel',
                'pro.upgrade': 'Upgrade to Pro',
                'pro.closeModal': 'Close modal',
                'pro.pricePeriod': '/ month',
                'pro.trialBadge': 'Cancel anytime',
                'pro.renewalNotice': '1 month · $19.99 USD/mo · auto-renews',
                'pro.featureBuzz': 'Buzz',
                'pro.featureBuzzDesc': 'Buzz desc',
                'pro.featureAgentSense': 'AgentSense',
                'pro.featureAgentSenseDesc': 'AgentSense desc',
                'pro.featureLora': 'LoRA',
                'pro.featureLoraDesc': 'LoRA desc',
                'pro.featureGift': 'Gifts',
                'pro.featureGiftDesc': 'Gifts desc',
                'pro.featureTts': 'TTS',
                'pro.featureTtsDesc': 'TTS desc',
                'pro.triggeredBy': `${opts?.feature ?? ''} requires Pro`,
            };
            return map[key] ?? key;
        },
    }),
}));

jest.mock('../../config', () => ({
    API_BASE: 'http://localhost:3000',
}));

const mockUseCheckoutSession = useCheckoutSession as jest.Mock;

describe('ProUpgradeModal component', () => {
    let handleCheckoutMock: jest.Mock;

    beforeEach(() => {
        jest.clearAllMocks();
        handleCheckoutMock = jest.fn();
        mockUseCheckoutSession.mockReturnValue({
            handleCheckout: handleCheckoutMock,
            isLoading: false,
            error: null,
        });
    });

    it('should not render anything by default', () => {
        render(<ProUpgradeModal priceId="price_123" />);
        expect(screen.queryByText('Unlock Aiome Pro')).not.toBeInTheDocument();
    });

    it('should open modal when stripe-402-payment-required event is dispatched', async () => {
        render(<ProUpgradeModal priceId="price_123" />);

        fireEvent(window, new CustomEvent('stripe-402-payment-required'));

        expect(screen.getByText('Unlock Aiome Pro')).toBeInTheDocument();
    });

    it('should call handleCheckout when upgrade button is clicked', async () => {
        render(<ProUpgradeModal priceId="price_123" />);
        
        // Open the modal
        fireEvent(window, new CustomEvent('stripe-402-payment-required'));

        const upgradeButton = screen.getByText('Upgrade to Pro');
        fireEvent.click(upgradeButton);

        expect(handleCheckoutMock).toHaveBeenCalled();
    });

    it('should close when cancel button is clicked', () => {
        render(<ProUpgradeModal priceId="price_123" />);
        
        // Open
        fireEvent(window, new CustomEvent('stripe-402-payment-required'));
        expect(screen.getByText('Unlock Aiome Pro')).toBeInTheDocument();

        // Close
        const cancelButton = screen.getByText('Cancel');
        fireEvent.click(cancelButton);

        expect(screen.queryByText('Unlock Aiome Pro')).not.toBeInTheDocument();
    });

    it('should close when Escape key is pressed', () => {
        render(<ProUpgradeModal priceId="price_123" />);

        // Open
        fireEvent(window, new CustomEvent('stripe-402-payment-required'));
        expect(screen.getByText('Unlock Aiome Pro')).toBeInTheDocument();

        // Press Escape (Modal listens on document)
        fireEvent.keyDown(document, { key: 'Escape' });

        expect(screen.queryByText('Unlock Aiome Pro')).not.toBeInTheDocument();
    });
});

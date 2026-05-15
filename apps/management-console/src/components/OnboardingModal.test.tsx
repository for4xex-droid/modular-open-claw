import { render, screen, fireEvent } from '@testing-library/react';
import OnboardingModal from './OnboardingModal';
import { AvatarCharacterProvider } from '../hooks/AvatarContext';

// Mock config for import.meta compatibility
jest.mock('../config', () => ({
    API_BASE: 'http://localhost:3000'
}));

// Mock i18n
jest.mock('../i18n', () => ({
    useTranslation: () => ({
        t: (key: string) => key
    })
}));

// Mock framer-motion to avoid animation delays
jest.mock('framer-motion', () => ({
    ...jest.requireActual('framer-motion'),
    AnimatePresence: ({ children }: any) => <>{children}</>
}));

// Mock ModelSetupStep to easily skip it
jest.mock('./ModelSetupStep', () => ({
    ModelSetupStep: ({ onSkip }: any) => (
        <button onClick={onSkip}>Skip LLM Setup</button>
    )
}));

describe('OnboardingModal Component i18n', () => {
    it('should use localized keys for onboarding steps', () => {
        render(
            <AvatarCharacterProvider>
                <OnboardingModal isOpen={true} onClose={() => {}} />
            </AvatarCharacterProvider>
        );
        
        // Step 0
        expect(screen.getByText('onboarding.welcome')).toBeInTheDocument();
        
        // Navigate to Step 1
        fireEvent.click(screen.getByText('onboarding.next'));
        
        // Step 1
        expect(screen.getByText('onboarding.setupTitle')).toBeInTheDocument();
        expect(screen.getByPlaceholderText('onboarding.namePlaceholder')).toBeInTheDocument();
    });

    it('should require ToS agreement before finalizing', () => {
        render(
            <AvatarCharacterProvider>
                <OnboardingModal isOpen={true} onClose={() => {}} />
            </AvatarCharacterProvider>
        );

        // Navigate to the final step
        const nextButton = () => screen.queryByRole('button', { name: /onboarding\.next/i });
        
        for (let i = 0; i < 6; i++) {
            const btn = nextButton();
            if (btn) {
                fireEvent.click(btn);
            } else {
                // Must be on LLM Setup step, look for our mock skip button
                const skipBtn = screen.queryByRole('button', { name: /Skip LLM Setup/i });
                if (skipBtn) {
                    fireEvent.click(skipBtn);
                } else {
                    // Must be the Experience Selection step which also skips
                    const expBtn = screen.queryByRole('button', { name: /onboarding\.beginner/i });
                    if (expBtn) {
                        fireEvent.click(expBtn);
                    }
                }
            }
        }

        // Now we should be on the last step (Awaken)
        const awakenButton = screen.getByRole('button', { name: /onboarding\.awaken/i });
        expect(awakenButton).toBeInTheDocument();
        
        // It should be disabled initially
        expect(awakenButton).toBeDisabled();

        // Check the ToS checkbox
        const tosCheckbox = screen.getByRole('checkbox');
        fireEvent.click(tosCheckbox);

        // Now it should be enabled
        expect(awakenButton).not.toBeDisabled();
    });
});

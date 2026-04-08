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
});

import { render, screen, act } from '@testing-library/react';
import SystemBirth from './SystemBirth';

// Mock i18n
jest.mock('../i18n', () => ({
    useTranslation: () => ({
        t: (key: string) => key // Return key name as value for testing
    })
}));

describe('SystemBirth Component i18n', () => {
    it('should use localized keys for system status', async () => {
        jest.useFakeTimers();
        render(<SystemBirth onComplete={() => {}} />);

        // Phase 2: Status messages should appear
        act(() => {
            jest.advanceTimersByTime(2500);
        });

        // Current implementation uses "CALIBRATING NEURAL CHRONICLE..."
        // Expected i18n version should use 'system.calibrating' or similar
        expect(screen.getByText('system.calibrating')).toBeInTheDocument();
        expect(screen.getByText('system.genesisProtocol')).toBeInTheDocument();

        jest.useRealTimers();
    });
});

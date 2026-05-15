import React from 'react';
import { render, screen, fireEvent, waitFor, act } from '@testing-library/react';
import LoraTrainingView from './LoraTrainingView';
import { authenticatedFetch } from '../lib/auth';

jest.mock('../lib/auth', () => ({
    authenticatedFetch: jest.fn()
}));

jest.mock('../config', () => ({
    API_BASE: 'http://localhost:3000'
}));

jest.mock('../i18n', () => ({
    useTranslation: () => ({ t: (key: string) => key })
}));

jest.mock('framer-motion', () => ({
    motion: {
        div: ({ children, ...props }: any) => <div {...props}>{children}</div>
    },
    AnimatePresence: ({ children }: any) => <>{children}</>
}));

jest.mock('lucide-react', () => ({
    Settings: () => <div data-testid="icon-settings"></div>,
    Play: () => <div data-testid="icon-play"></div>,
    Activity: () => <div data-testid="icon-activity"></div>,
    BrainCircuit: () => <div data-testid="icon-brain"></div>,
    Database: () => <div data-testid="icon-database"></div>,
    RefreshCw: () => <div data-testid="icon-refresh"></div>,
    Network: () => <div data-testid="icon-network"></div>
}));

describe('LoraTrainingView Component', () => {
    let mockFetch: jest.Mock;

    beforeEach(() => {
        jest.clearAllMocks();
        mockFetch = authenticatedFetch as jest.Mock;
        jest.useFakeTimers();
    });

    afterEach(() => {
        jest.useRealTimers();
    });

    it('renders form and default values correctly', () => {
        render(<LoraTrainingView />);
        
        expect(screen.getByText('lora.title')).toBeTruthy();
        expect(screen.getByText('lora.noActiveSession')).toBeTruthy();
        
        expect(screen.getByDisplayValue('3')).toBeTruthy();
        expect(screen.getByDisplayValue('0.0001')).toBeTruthy();
        expect(screen.getByDisplayValue('16')).toBeTruthy();
        expect(screen.getByDisplayValue('4')).toBeTruthy();
    });

    it('button is disabled if datasetId is empty', () => {
        render(<LoraTrainingView />);
        
        const startBtn = screen.getByText('lora.startTraining');
        expect(startBtn).toBeDisabled();
    });

    it('successfully starts training and shows telemetry panel', async () => {
        mockFetch.mockResolvedValueOnce({
            ok: true,
            json: async () => ({ job_id: 'job-123' })
        });
        // Mock the immediate status check
        mockFetch.mockResolvedValueOnce({
            ok: true,
            json: async () => ({ status: 'Training' })
        });

        render(<LoraTrainingView />);
        
        const datasetInput = screen.getByPlaceholderText('e.g. core-skills-v2');
        fireEvent.change(datasetInput, { target: { value: 'my-dataset' } });

        const startBtn = screen.getByText('lora.startTraining');
        
        await act(async () => {
            fireEvent.click(startBtn);
        });

        expect(mockFetch).toHaveBeenCalledWith(
            'http://localhost:3000/api/v1/lora/train',
            expect.objectContaining({
                method: 'POST',
                body: expect.stringContaining('my-dataset')
            })
        );

        expect(screen.getByText('job-123')).toBeTruthy();
        expect(screen.getByText('Training')).toBeTruthy();
        expect(screen.queryByText('lora.noActiveSession')).toBeNull();
    });

    it('handles network error on start', async () => {
        mockFetch.mockRejectedValueOnce(new Error('Network Error'));

        render(<LoraTrainingView />);
        
        const datasetInput = screen.getByPlaceholderText('e.g. core-skills-v2');
        fireEvent.change(datasetInput, { target: { value: 'my-dataset' } });

        const startBtn = screen.getByText('lora.startTraining');
        
        await act(async () => {
            fireEvent.click(startBtn);
        });

        expect(screen.getByText('Network Error')).toBeTruthy();
    });

    it('polls status until completed', async () => {
        // Start training
        mockFetch.mockResolvedValueOnce({
            ok: true,
            json: async () => ({ job_id: 'job-123' })
        });
        
        // Initial check
        mockFetch.mockResolvedValueOnce({
            ok: true,
            json: async () => ({ status: 'Training' })
        });

        render(<LoraTrainingView />);
        
        const datasetInput = screen.getByPlaceholderText('e.g. core-skills-v2');
        fireEvent.change(datasetInput, { target: { value: 'my-dataset' } });

        await act(async () => {
            fireEvent.click(screen.getByText('lora.startTraining'));
        });

        // Advance timer for first poll
        mockFetch.mockResolvedValueOnce({
            ok: true,
            json: async () => ({ status: 'Completed' })
        });

        await act(async () => {
            jest.advanceTimersByTime(3000);
        });

        expect(screen.getByText('Completed')).toBeTruthy();
        expect(screen.getByText('> [SUCCESS] Model weights successfully exported to Vault.')).toBeTruthy();

        // Advance timer again to ensure polling stopped
        await act(async () => {
            jest.advanceTimersByTime(3000);
        });

        // 1 for start, 1 for initial check, 1 for poll = 3
        expect(mockFetch).toHaveBeenCalledTimes(3);
    });

    it('handles fetch error during polling', async () => {
        // Start training
        mockFetch.mockResolvedValueOnce({
            ok: true,
            json: async () => ({ job_id: 'job-123' })
        });
        
        // Initial check fails with 500
        mockFetch.mockResolvedValueOnce({
            ok: false,
            status: 500
        });

        render(<LoraTrainingView />);
        
        const datasetInput = screen.getByPlaceholderText('e.g. core-skills-v2');
        fireEvent.change(datasetInput, { target: { value: 'my-dataset' } });

        await act(async () => {
            fireEvent.click(screen.getByText('lora.startTraining'));
        });

        expect(screen.getByText('lora.errorFetchStatus')).toBeTruthy();
    });
});

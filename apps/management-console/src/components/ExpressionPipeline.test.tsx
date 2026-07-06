/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import React from 'react';
import { render, screen, fireEvent, waitFor, act } from '@testing-library/react';
import ExpressionPipeline from './ExpressionPipeline';
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

jest.mock('./common/Toast', () => ({
    useToast: () => ({ showToast: jest.fn() })
}));

jest.mock('./ui/LoadingState', () => ({
    LoadingState: () => <div data-testid="loading-state">loading</div>,
}));

jest.mock('./ui/EmptyState', () => ({
    EmptyState: ({ titleKey }: { titleKey: string }) => <div data-testid="empty-state">{titleKey}</div>,
}));

jest.mock('framer-motion', () => ({
    motion: {
        div: ({ children, ...props }: any) => <div {...props}>{children}</div>
    },
    AnimatePresence: ({ children }: any) => <>{children}</>
}));

jest.mock('lucide-react', () => ({
    Sparkles: () => <div data-testid="icon-sparkles"></div>,
    RefreshCw: () => <div data-testid="icon-refresh"></div>,
    History: () => <div data-testid="icon-history"></div>,
    Activity: () => <div data-testid="icon-activity"></div>,
    BrainCircuit: () => <div data-testid="icon-brain"></div>,
    ShieldCheck: () => <div data-testid="icon-shield"></div>,
    ToggleLeft: () => <div data-testid="icon-toggle-left"></div>,
    ToggleRight: () => <div data-testid="icon-toggle-right"></div>,
    MessageCircle: () => <div data-testid="icon-message"></div>,
    Clock: () => <div data-testid="icon-clock"></div>,
    AlertTriangle: () => <div data-testid="icon-alert"></div>
}));

describe('ExpressionPipeline Component', () => {
    let mockFetch: jest.Mock;

    beforeEach(() => {
        jest.clearAllMocks();
        mockFetch = authenticatedFetch as jest.Mock;
        jest.useFakeTimers();
    });

    afterEach(() => {
        jest.useRealTimers();
    });

    const mockStatus = {
        status: 'idle',
        auto_expression: false,
        pending_expressions: 0,
        last_insight: 'I am reflecting on recent interactions.',
        message_ja: ''
    };

    const mockExpressions = [
        {
            id: 'expr-1234',
            content: 'Hello world! This is my first expression.',
            emotion: 'joy',
            karma_refs: ['ref1', 'ref2'],
            created_at: '2026-05-14T10:00:00Z'
        }
    ];

    it('renders empty state initially', async () => {
        mockFetch.mockImplementation((url) => {
            if (url.includes('/api/expression/status')) {
                return Promise.resolve({ ok: true, json: async () => mockStatus });
            }
            return Promise.resolve({ ok: true, json: async () => [] });
        });

        render(<ExpressionPipeline />);
        
        await waitFor(() => {
            expect(screen.getByText('expression.title')).toBeTruthy();
        });
        
        await waitFor(() => {
            expect(screen.getByText('expression.noExpressions')).toBeTruthy();
        });
    });

    it('fetches and displays status and expressions on mount', async () => {
        // mock twice for the two initial fetches: fetchStatus, fetchExpressions
        mockFetch.mockImplementation((url) => {
            if (url.includes('/api/expression/status')) {
                return Promise.resolve({ ok: true, json: async () => mockStatus });
            }
            if (url.includes('/api/expression/list')) {
                return Promise.resolve({ ok: true, json: async () => mockExpressions });
            }
            return Promise.resolve({ ok: true, json: async () => ({}) });
        });

        render(<ExpressionPipeline />);

        await waitFor(() => {
            expect(screen.getByText(/"I am reflecting on recent interactions."/)).toBeTruthy();
            expect(screen.getByText('IDLE')).toBeTruthy();
            expect(screen.getByText('Hello world! This is my first expression.')).toBeTruthy();
            expect(screen.getByText('JOY')).toBeTruthy();
            expect(screen.getByText('expr-123')).toBeTruthy();
        });
    });

    it('toggles auto expression mode successfully', async () => {
        const localMockStatus = {
            status: 'idle',
            auto_expression: false,
            pending_expressions: 0,
            last_insight: 'I am reflecting on recent interactions.',
            message_ja: ''
        };

        mockFetch.mockImplementation((url, options) => {
            if (url.includes('/api/expression/status')) {
                return Promise.resolve({ ok: true, json: async () => ({ ...localMockStatus }) });
            }
            if (url.includes('/api/expression/list')) {
                return Promise.resolve({ ok: true, json: async () => mockExpressions });
            }
            if (url.includes('/api/expression/auto') && options?.method === 'POST') {
                // Update status for the next fetch
                localMockStatus.auto_expression = true;
                return Promise.resolve({ ok: true });
            }
            return Promise.resolve({ ok: true, json: async () => ({}) });
        });

        render(<ExpressionPipeline />);

        await waitFor(() => {
            expect(screen.getByText(/"I am reflecting on recent interactions."/)).toBeTruthy();
            expect(screen.getByText(/OFF/)).toBeTruthy(); // Initial state
        });

        const toggleBtn = screen.getByText(/expression.autonomousMode/);
        
        await act(async () => {
            fireEvent.click(toggleBtn);
        });

        expect(mockFetch).toHaveBeenCalledWith(
            'http://localhost:3000/api/expression/auto',
            expect.objectContaining({
                method: 'POST',
                body: JSON.stringify({ enabled: true })
            })
        );

        await waitFor(() => {
            expect(screen.getByText(/ON/)).toBeTruthy();
        });
    });

    it('handles manual generation', async () => {
        mockFetch.mockImplementation((url, options) => {
            if (url.includes('/api/expression/status')) {
                return Promise.resolve({ ok: true, json: async () => mockStatus });
            }
            if (url.includes('/api/expression/list')) {
                return Promise.resolve({ ok: true, json: async () => mockExpressions });
            }
            if (url.includes('/api/expression/generate') && options?.method === 'POST') {
                return Promise.resolve({ ok: true });
            }
            return Promise.resolve({ ok: true, json: async () => ({}) });
        });

        render(<ExpressionPipeline />);

        await waitFor(() => {
            expect(screen.getByText('expression.generate')).toBeTruthy();
        });

        const generateBtn = screen.getByText('expression.generate');
        
        await act(async () => {
            fireEvent.click(generateBtn);
        });

        expect(mockFetch).toHaveBeenCalledWith(
            'http://localhost:3000/api/expression/generate',
            expect.objectContaining({ method: 'POST' })
        );

        // It should fetch list and status again
        await waitFor(() => {
            expect(mockFetch).toHaveBeenCalledTimes(5); // 2 init + 1 gen + 2 refetch
        });
    });

    it('handles API errors gracefully', async () => {
        const consoleSpy = jest.spyOn(console, 'error').mockImplementation();
        mockFetch.mockRejectedValue(new Error('Network error'));

        render(<ExpressionPipeline />);

        await waitFor(() => {
            expect(screen.getByText('common.networkError')).toBeTruthy();
        });

        expect(consoleSpy).toHaveBeenCalledWith('Failed to fetch expression status', expect.any(Error));
        expect(consoleSpy).toHaveBeenCalledWith('Failed to fetch expressions', expect.any(Error));

        consoleSpy.mockRestore();
    });
    
    it('polls status at intervals', async () => {
        mockFetch.mockImplementation((url) => {
            if (url.includes('/api/expression/status')) {
                return Promise.resolve({ ok: true, json: async () => mockStatus });
            }
            if (url.includes('/api/expression/list')) {
                return Promise.resolve({ ok: true, json: async () => mockExpressions });
            }
            return Promise.resolve({ ok: true, json: async () => ({}) });
        });

        render(<ExpressionPipeline />);

        await waitFor(() => {
            expect(mockFetch).toHaveBeenCalledTimes(2); // Initial
        });

        await act(async () => {
            jest.advanceTimersByTime(30000);
        });

        expect(mockFetch).toHaveBeenCalledTimes(3); // One additional status fetch
    });
});

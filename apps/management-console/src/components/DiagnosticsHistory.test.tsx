/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import React from 'react';
import { render, screen, fireEvent, waitFor, act } from '@testing-library/react';
import DiagnosticsHistory from './DiagnosticsHistory';
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

// Mock framer-motion to avoid animation issues in Jest
jest.mock('framer-motion', () => ({
    motion: {
        div: ({ children, ...props }: any) => <div {...props}>{children}</div>
    }
}));

// Mock lucide-react icons
jest.mock('lucide-react', () => ({
    Activity: () => <div data-testid="icon-activity"></div>,
    History: () => <div data-testid="icon-history"></div>,
    AlertTriangle: () => <div data-testid="icon-alert"></div>,
    Clock: () => <div data-testid="icon-clock"></div>,
    Database: () => <div data-testid="icon-database"></div>,
    ChevronRight: () => <div data-testid="icon-chevron"></div>,
    RefreshCw: () => <div data-testid="icon-refresh"></div>,
    Hash: () => <div data-testid="icon-hash"></div>
}));

describe('DiagnosticsHistory Component', () => {
    let mockFetch: jest.Mock;

    beforeEach(() => {
        jest.clearAllMocks();
        mockFetch = authenticatedFetch as jest.Mock;
    });

    const mockDiagnostics = [
        {
            id: 1,
            job_id: 'job-12345678',
            root_cause: 'Network timeout',
            self_repair_hint: 'Retried connection',
            failure_category: 'network',
            timestamp: '2026-05-14T10:00:00Z'
        },
        {
            id: 2,
            job_id: 'job-abcdefgh',
            root_cause: 'Invalid token',
            self_repair_hint: null,
            failure_category: 'security',
            timestamp: null
        }
    ];

    const mockLedger = [
        {
            id: 1,
            table_name: 'users',
            operation: 'INSERT',
            record_id: 'user-1',
            current_hash: 'abcdef1234567890abcdef',
            timestamp: '2026-05-14T11:00:00Z'
        },
        {
            id: 2,
            table_name: 'sessions',
            operation: 'DELETE',
            record_id: 'sess-1',
            current_hash: '0987654321fedcba',
            timestamp: null
        }
    ];

    it('renders loading state initially', async () => {
        // Prevent act warning by returning a pending promise
        let resolveFetch: any;
        mockFetch.mockImplementation(() => new Promise(resolve => resolveFetch = resolve));

        render(<DiagnosticsHistory />);
        
        expect(screen.getByTestId('icon-refresh')).toBeTruthy();
        expect(screen.getByText('diagnostics.syncing')).toBeTruthy();

        // Resolve fetch to clean up
        await act(async () => {
            resolveFetch({ ok: true, json: async () => [] });
        });
    });

    it('fetches and renders diagnostics data', async () => {
        mockFetch.mockResolvedValueOnce({
            ok: true,
            json: async () => mockDiagnostics
        });

        render(<DiagnosticsHistory />);

        await waitFor(() => {
            expect(screen.queryByTestId('icon-refresh')).toBeNull();
        });

        expect(screen.getByText('job-1234')).toBeTruthy(); // job_id slice
        expect(screen.getByText('job-abcd')).toBeTruthy();
        expect(screen.getByText('Network timeout')).toBeTruthy();
        expect(screen.getByText('Invalid token')).toBeTruthy();
        expect(screen.getByText('Retried connection')).toBeTruthy();
        expect(screen.getByText('diagnostics.unknown')).toBeTruthy(); // For null timestamp
    });

    it('switches to ledger tab and renders ledger data', async () => {
        mockFetch.mockResolvedValueOnce({ ok: true, json: async () => [] }); // Initial load
        mockFetch.mockResolvedValueOnce({
            ok: true,
            json: async () => mockLedger
        });

        render(<DiagnosticsHistory />);

        await waitFor(() => {
            expect(screen.queryByTestId('icon-refresh')).toBeNull();
        });

        fireEvent.click(screen.getByText('diagnostics.tabLedger'));

        await waitFor(() => {
            expect(mockFetch).toHaveBeenCalledWith('http://localhost:3000/api/v1/audit/ledger');
        });

        await waitFor(() => {
            expect(screen.getByText('users')).toBeTruthy();
            expect(screen.getByText('sessions')).toBeTruthy();
            expect(screen.getByText('INSERT')).toBeTruthy();
            expect(screen.getByText('DELETE')).toBeTruthy();
            expect(screen.getByText('ID: user-1')).toBeTruthy();
            expect(screen.getByText('--:--')).toBeTruthy(); // For null timestamp
        });
    });

    it('handles load more pagination correctly', async () => {
        // Generate 25 items
        const manyDiagnostics = Array.from({ length: 25 }, (_, i) => ({
            id: i,
            job_id: `job-${i}0000000`,
            root_cause: `Cause ${i}`,
            self_repair_hint: null,
            failure_category: 'runtime',
            timestamp: '2026-05-14T10:00:00Z'
        }));

        mockFetch.mockResolvedValueOnce({
            ok: true,
            json: async () => manyDiagnostics
        });

        render(<DiagnosticsHistory />);

        await waitFor(() => {
            expect(screen.getByText('Cause 0')).toBeTruthy();
        });

        // Only 20 should be rendered initially
        expect(screen.queryByText('Cause 20')).toBeNull();
        
        const loadMoreBtn = screen.getByText('LOAD MORE');
        expect(loadMoreBtn).toBeTruthy();

        fireEvent.click(loadMoreBtn);

        await waitFor(() => {
            expect(screen.getByText('Cause 20')).toBeTruthy();
        });
    });

    it('handles fetch errors gracefully', async () => {
        const consoleSpy = jest.spyOn(console, 'error').mockImplementation();
        mockFetch.mockRejectedValueOnce(new Error('API Down'));

        render(<DiagnosticsHistory />);

        await waitFor(() => {
            expect(screen.queryByTestId('icon-refresh')).toBeNull();
        });

        // Should render empty list without crashing
        expect(consoleSpy).toHaveBeenCalledWith('Failed to fetch diagnostics', expect.any(Error));
        consoleSpy.mockRestore();
    });
});

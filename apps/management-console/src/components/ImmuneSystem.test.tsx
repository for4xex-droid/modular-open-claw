/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import React from 'react';
import { render, screen, fireEvent, waitFor, act } from '@testing-library/react';
import ImmuneSystem from './ImmuneSystem';
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

jest.mock('./common/ConfirmModal', () => {
    return function MockConfirmModal(props: any) {
        if (!props.isOpen) return null;
        return (
            <div data-testid="confirm-modal">
                <h2>{props.title}</h2>
                <button onClick={props.onConfirm}>{props.confirmText}</button>
                <button onClick={props.onCancel}>Cancel</button>
            </div>
        );
    };
});

// Mock framer-motion to avoid animation issues
jest.mock('framer-motion', () => ({
    motion: {
        div: ({ children, ...props }: any) => <div {...props}>{children}</div>
    },
    AnimatePresence: ({ children }: any) => <>{children}</>
}));

// Mock lucide-react icons
jest.mock('lucide-react', () => ({
    Shield: () => <div data-testid="icon-shield"></div>,
    AlertTriangle: () => <div data-testid="icon-alert"></div>,
    Search: () => <div data-testid="icon-search"></div>,
    Filter: () => <div data-testid="icon-filter"></div>,
    Lock: () => <div data-testid="icon-lock"></div>,
    Plus: () => <div data-testid="icon-plus"></div>,
    X: () => <div data-testid="icon-x"></div>,
    Activity: () => <div data-testid="icon-activity"></div>
}));

describe('ImmuneSystem Component', () => {
    let mockFetch: jest.Mock;

    beforeEach(() => {
        jest.clearAllMocks();
        mockFetch = authenticatedFetch as jest.Mock;
    });

    const mockRules = [
        {
            id: 'rule-1',
            pattern: '/etc/passwd',
            severity: 90,
            action: 'BLOCK',
            approval_status: 'Approved',
            created_at: '2026-05-14T10:00:00Z'
        },
        {
            id: 'rule-2',
            pattern: 'SELECT * FROM users',
            severity: 60,
            action: 'WARN',
            approval_status: 'Pending',
            created_at: '2026-05-14T10:00:00Z'
        }
    ];

    const mockQuarantined = [
        {
            id: 'asset-1',
            asset_name: 'malicious_script.sh',
            image_hash: '1234567890abcdef1234567890abcdef',
            reason: 'Suspicious payload',
            status: 'QUARANTINED',
            uploaded_at: '2026-05-14T10:00:00Z'
        }
    ];

    const mockAegisStatus = {
        stats: {
            total_incidents_7d: 5,
            unresolved: 1,
            distinct_skills: 2,
            top_failing_skill: 'CodeGenerator'
        },
        open_incidents: [
            {
                id: 'inc-1',
                skill_name: 'CodeGenerator',
                status: 'Open',
                input_payload: 'System bypass payload...',
                created_at: '2026-05-14T10:00:00Z'
            }
        ]
    };

    it('renders and fetches data correctly', async () => {
        mockFetch.mockImplementation((url) => {
            if (url.includes('/api/synergy/rules')) return Promise.resolve({ ok: true, json: async () => mockRules });
            if (url.includes('/api/v1/audit/quarantine')) return Promise.resolve({ ok: true, json: async () => mockQuarantined });
            if (url.includes('/api/v1/watchtower')) return Promise.resolve({ ok: true, json: async () => mockAegisStatus });
            return Promise.resolve({ ok: true, json: async () => [] });
        });

        render(<ImmuneSystem />);

        expect(screen.getByText('immune.title')).toBeTruthy();

        // Should be on RULES tab by default
        await waitFor(() => {
            expect(screen.getByText('/etc/passwd')).toBeTruthy();
            expect(screen.getByText('CRITICAL')).toBeTruthy();
            expect(screen.getByText('SELECT * FROM users')).toBeTruthy();
            expect(screen.getByText('HIGH')).toBeTruthy();
        });
    });

    it('switches to QUARANTINE tab', async () => {
        mockFetch.mockImplementation((url) => {
            if (url.includes('/api/synergy/rules')) return Promise.resolve({ ok: true, json: async () => [] });
            if (url.includes('/api/v1/audit/quarantine')) return Promise.resolve({ ok: true, json: async () => mockQuarantined });
            if (url.includes('/api/v1/watchtower')) return Promise.resolve({ ok: true, json: async () => mockAegisStatus });
            return Promise.resolve({ ok: true, json: async () => [] });
        });

        render(<ImmuneSystem />);

        fireEvent.click(screen.getByText('immune.tabQuarantine'));

        await waitFor(() => {
            expect(screen.getByText('malicious_script.sh')).toBeTruthy();
            expect(screen.getByText('Suspicious payload')).toBeTruthy();
        });
    });

    it('switches to AEGIS tab', async () => {
        mockFetch.mockImplementation((url) => {
            if (url.includes('/api/synergy/rules')) return Promise.resolve({ ok: true, json: async () => [] });
            if (url.includes('/api/v1/audit/quarantine')) return Promise.resolve({ ok: true, json: async () => [] });
            if (url.includes('/api/v1/watchtower')) return Promise.resolve({ ok: true, json: async () => mockAegisStatus });
            return Promise.resolve({ ok: true, json: async () => [] });
        });

        render(<ImmuneSystem />);

        fireEvent.click(screen.getByText('immune.tabAegis'));

        await waitFor(() => {
            expect(screen.getAllByText(/CodeGenerator/).length).toBeGreaterThan(0);
            expect(screen.getAllByText(/OPEN/).length).toBeGreaterThan(0);
            // Verify the stat label is rendered alongside the value
            expect(screen.getByText('immune.totalIncidents7d')).toBeTruthy();
            expect(screen.getByText('immune.unresolved')).toBeTruthy();
        });
    });

    it('adds a new rule successfully', async () => {
        mockFetch.mockImplementation((url, options) => {
            if (url.includes('/api/synergy/rules') && options?.method === 'POST') {
                return Promise.resolve({ ok: true });
            }
            return Promise.resolve({ ok: true, json: async () => [] });
        });

        render(<ImmuneSystem />);

        // Click Add Rule button
        fireEvent.click(screen.getByText('immune.forgeNewRule'));

        await waitFor(() => {
            expect(screen.getByPlaceholderText('e.g. /etc/passwd')).toBeTruthy();
        });

        // Fill form
        fireEvent.change(screen.getByPlaceholderText('e.g. /etc/passwd'), { target: { value: 'test pattern' } });
        
        // Save
        fireEvent.click(screen.getByText('immune.activateRule'));

        await waitFor(() => {
            expect(mockFetch).toHaveBeenCalledWith(
                'http://localhost:3000/api/synergy/rules',
                expect.objectContaining({
                    method: 'POST',
                    body: expect.stringContaining('test pattern')
                })
            );
        });
    });

    it('edits an existing rule successfully', async () => {
        mockFetch.mockImplementation((url, options) => {
            if (url.includes('/api/synergy/rules') && options?.method === 'PUT') {
                return Promise.resolve({ ok: true });
            }
            if (url.includes('/api/synergy/rules') && !options) {
                return Promise.resolve({ ok: true, json: async () => mockRules });
            }
            return Promise.resolve({ ok: true, json: async () => [] });
        });

        render(<ImmuneSystem />);

        await waitFor(() => {
            expect(screen.getByText('/etc/passwd')).toBeTruthy();
        });

        // Click first edit button
        const editButtons = screen.getAllByText('immune.editButton');
        fireEvent.click(editButtons[0]);

        await waitFor(() => {
            expect(screen.getByDisplayValue('/etc/passwd')).toBeTruthy();
        });

        // Change value
        fireEvent.change(screen.getByDisplayValue('/etc/passwd'), { target: { value: 'new pattern' } });

        // Save
        fireEvent.click(screen.getByText('immune.updateRule'));

        await waitFor(() => {
            expect(mockFetch).toHaveBeenCalledWith(
                'http://localhost:3000/api/synergy/rules',
                expect.objectContaining({
                    method: 'PUT',
                    body: expect.stringContaining('new pattern')
                })
            );
        });
    });

    it('deletes a rule successfully', async () => {
        mockFetch.mockImplementation((url, options) => {
            if (url.includes('/api/synergy/rules/rule-1') && options?.method === 'DELETE') {
                return Promise.resolve({ ok: true });
            }
            if (url.includes('/api/synergy/rules') && !options) {
                return Promise.resolve({ ok: true, json: async () => mockRules });
            }
            return Promise.resolve({ ok: true, json: async () => [] });
        });

        render(<ImmuneSystem />);

        await waitFor(() => {
            expect(screen.getByText('/etc/passwd')).toBeTruthy();
        });

        // Click first delete button
        const deleteButtons = screen.getAllByText('immune.deleteButton');
        fireEvent.click(deleteButtons[0]);

        // Confirm modal opens
        await waitFor(() => {
            expect(screen.getByText('immune.deleteRuleTitle')).toBeTruthy();
        });

        // Click confirm
        fireEvent.click(screen.getByText('immune.confirmDelete'));

        await waitFor(() => {
            expect(mockFetch).toHaveBeenCalledWith(
                'http://localhost:3000/api/synergy/rules/rule-1',
                expect.objectContaining({ method: 'DELETE' })
            );
        });
    });

    it('releases a quarantined asset successfully', async () => {
        mockFetch.mockImplementation((url, options) => {
            if (url.includes('/api/v1/audit/quarantine/asset-1/release') && options?.method === 'POST') {
                return Promise.resolve({ ok: true });
            }
            if (url.includes('/api/v1/audit/quarantine') && !options) {
                return Promise.resolve({ ok: true, json: async () => mockQuarantined });
            }
            return Promise.resolve({ ok: true, json: async () => [] });
        });

        render(<ImmuneSystem />);

        fireEvent.click(screen.getByText('immune.tabQuarantine'));

        await waitFor(() => {
            expect(screen.getByText('immune.releaseException')).toBeTruthy();
        });

        fireEvent.click(screen.getByText('immune.releaseException'));

        // Confirm modal
        await waitFor(() => {
            expect(screen.getByText('immune.releaseAssetTitle')).toBeTruthy();
        });

        fireEvent.click(screen.getByText('immune.confirmRelease'));

        await waitFor(() => {
            expect(mockFetch).toHaveBeenCalledWith(
                'http://localhost:3000/api/v1/audit/quarantine/asset-1/release',
                expect.objectContaining({ method: 'POST' })
            );
        });
    });

    it('handles empty state properly', async () => {
        mockFetch.mockResolvedValue({ ok: true, json: async () => [] });
        render(<ImmuneSystem />);

        await waitFor(() => {
            expect(screen.getByText('immune.noActiveRules')).toBeTruthy();
        });

        fireEvent.click(screen.getByText('immune.tabQuarantine'));

        await waitFor(() => {
            expect(screen.getByText('immune.quarantineClean')).toBeTruthy();
        });
    });

    it('handles API error (network failure) gracefully', async () => {
        const consoleSpy = jest.spyOn(console, 'error').mockImplementation(() => {});

        mockFetch.mockRejectedValue(new Error('Connection refused'));

        render(<ImmuneSystem />);

        await waitFor(() => {
            expect(consoleSpy).toHaveBeenCalledWith(
                'Failed to fetch immune rules',
                expect.any(Error)
            );
        });

        // Component should still render without crashing
        expect(screen.getByText('immune.title')).toBeTruthy();

        consoleSpy.mockRestore();
    });

    it('handles non-ok API response without crashing', async () => {
        mockFetch.mockResolvedValue({ ok: false, status: 500 });

        render(<ImmuneSystem />);

        // Should show empty state since data couldn't be loaded
        await waitFor(() => {
            expect(screen.getByText('immune.noActiveRules')).toBeTruthy();
        });
    });
});

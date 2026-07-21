/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

import React from 'react';
import { render, screen, waitFor, fireEvent } from '@testing-library/react';
import '@testing-library/jest-dom';
import Timeline from './Timeline';
import { authenticatedFetch } from '../lib/auth';
import { CAUSAL_JOB_ID_STORAGE_KEY } from '../lib/a2uiTabs';

jest.mock('../lib/auth', () => ({
    authenticatedFetch: jest.fn(),
}));

jest.mock('../config', () => ({
    API_BASE: 'http://localhost'
}));

jest.mock('../i18n', () => ({
    useTranslation: () => ({
        t: (key: string, options?: any) => {
            const defaults: Record<string, string> = {
                'timeline.title': 'Timeline Chronicles',
                'timeline.chronicles': 'CHRONICLES',
                'timeline.syncing': 'Synchronizing matrix...',
                'timeline.noRecords': 'No records found in sovereign ledger',
                'timeline.localMemory': 'Local Memory',
                'timeline.federatedMemory': 'Federated Memory',
                'timeline.evolutionStep': 'Evolution Step',
                'timeline.openCausal': 'Trace',
            };
            return options?.defaultValue || defaults[key] || key;
        }
    })
}));

describe('Timeline', () => {
    beforeEach(() => {
        jest.clearAllMocks();
        sessionStorage.clear();
    });

    it('should show loading indicator on mount', () => {
        // APIリクエストがペンディング状態であることをシミュレート
        (authenticatedFetch as jest.Mock).mockReturnValue(new Promise(() => {}));

        render(<Timeline />);
        expect(screen.getByText('Synchronizing matrix...')).toBeInTheDocument();
    });

    it('should render merged and sorted timeline events correctly', async () => {
        const mockHealth = { node_id: 'node-local-111' };
        const mockKarma = [
            {
                id: 'k1',
                created_at: '2026-05-30T10:00:00Z',
                node_id: 'node-local-111',
                karma_type: 'Technical',
                job_id: '101',
                lesson: 'Always keep your scopes locked.',
                inspiration: 'Rule adherence yields perfect precision.'
            },
            {
                id: 'k2',
                created_at: '2026-05-30T12:00:00Z',
                node_id: 'node-federated-222',
                karma_type: 'Relational',
                job_id: '102',
                lesson: 'P2P consensus reached cleanly.'
            }
        ];
        const mockEvolution = [
            {
                id: 'e1',
                created_at: '2026-05-30T11:00:00Z',
                event_type: 'Upgrade',
                description: 'Core logic evolved to version 1.5.'
            }
        ];

        (authenticatedFetch as jest.Mock).mockImplementation((url: string) => {
            if (url.includes('/api/health')) {
                return Promise.resolve({ ok: true, json: async () => mockHealth });
            }
            if (url.includes('/api/synergy/karma')) {
                return Promise.resolve({ ok: true, json: async () => mockKarma });
            }
            if (url.includes('/api/system/evolution')) {
                return Promise.resolve({ ok: true, json: async () => mockEvolution });
            }
            return Promise.resolve({ ok: false });
        });

        render(<Timeline />);

        // Loading が終了するのを待つ
        await waitFor(() => {
            expect(screen.queryByText('Synchronizing matrix...')).not.toBeInTheDocument();
        });

        // 合計の記数表示
        expect(screen.getByText(/3 CHRONICLES/i)).toBeInTheDocument();

        // 時系列ソートの順序検証 (created_at 降順)
        // 1位: 12:00:00Z -> k2 (Federated Memory)
        // 2位: 11:00:00Z -> e1 (Evolution Step)
        // 3位: 10:00:00Z -> k1 (Local Memory)
        const lessons = screen.getAllByText(/Always keep your scopes locked|P2P consensus reached cleanly|Core logic evolved to version 1.5/);
        expect(lessons).toHaveLength(3);
        
        // 降順ソート確認
        expect(lessons[0]).toHaveTextContent('P2P consensus reached cleanly');
        expect(lessons[1]).toHaveTextContent('Core logic evolved to version 1.5.');
        expect(lessons[2]).toHaveTextContent('Always keep your scopes locked.');

        // 各種バッジの検証
        expect(screen.getByText('Local Memory')).toBeInTheDocument();
        expect(screen.getByText('Federated Memory')).toBeInTheDocument();
        expect(screen.getByText('Evolution Step')).toBeInTheDocument();

        // 外部連携インスピレーションの表示検証
        expect(screen.getByText('Rule adherence yields perfect precision.')).toBeInTheDocument();

        // OP-022: job_id 付き karma 行に Causal 導線
        expect(screen.getByTestId('timeline-open-causal-101')).toBeInTheDocument();
        expect(screen.getByTestId('timeline-open-causal-102')).toBeInTheDocument();
    });

    it('dispatches a2ui-navigate with jobId when Trace is clicked', async () => {
        const mockHealth = { node_id: 'node-local-111' };
        const mockKarma = [
            {
                id: 'k1',
                created_at: '2026-05-30T10:00:00Z',
                node_id: 'node-local-111',
                karma_type: 'Technical',
                job_id: 'job_trace_1',
                lesson: 'Trace me',
            },
        ];

        (authenticatedFetch as jest.Mock).mockImplementation((url: string) => {
            if (url.includes('/api/health')) {
                return Promise.resolve({ ok: true, json: async () => mockHealth });
            }
            if (url.includes('/api/synergy/karma')) {
                return Promise.resolve({ ok: true, json: async () => mockKarma });
            }
            if (url.includes('/api/system/evolution')) {
                return Promise.resolve({ ok: true, json: async () => [] });
            }
            return Promise.resolve({ ok: false });
        });

        const navListener = jest.fn();
        window.addEventListener('a2ui-navigate', navListener);

        render(<Timeline />);
        await waitFor(() => {
            expect(screen.getByTestId('timeline-open-causal-job_trace_1')).toBeInTheDocument();
        });

        fireEvent.click(screen.getByTestId('timeline-open-causal-job_trace_1'));

        expect(sessionStorage.getItem(CAUSAL_JOB_ID_STORAGE_KEY)).toBe('job_trace_1');
        expect(navListener).toHaveBeenCalled();
        const evt = navListener.mock.calls[0][0] as CustomEvent;
        expect(evt.detail).toEqual({ tab: 'causal', jobId: 'job_trace_1' });

        window.removeEventListener('a2ui-navigate', navListener);
    });

    it('should render empty state when no events exist', async () => {
        (authenticatedFetch as jest.Mock).mockResolvedValue({
            ok: true,
            json: async () => []
        });

        render(<Timeline />);

        await waitFor(() => {
            expect(screen.queryByText('Synchronizing matrix...')).not.toBeInTheDocument();
        });

        expect(screen.getByText('No records found in sovereign ledger')).toBeInTheDocument();
    });

    it('should handle API errors gracefully without crashing', async () => {
        (authenticatedFetch as jest.Mock).mockResolvedValue({
            ok: false,
            statusText: 'Internal Server Error'
        });

        render(<Timeline />);

        await waitFor(() => {
            expect(screen.queryByText('Synchronizing matrix...')).not.toBeInTheDocument();
        });

        // エラー時も空のタイムライン表示として機能すること
        expect(screen.getByText('No records found in sovereign ledger')).toBeInTheDocument();
    });
});

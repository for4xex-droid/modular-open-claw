/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import CharacterPanel from './CharacterPanel';
import { authenticatedFetch } from '../../lib/auth';
import { API_BASE } from '../../config';

jest.mock('../../lib/auth', () => ({
    authenticatedFetch: jest.fn()
}));

jest.mock('../../config', () => ({
    API_BASE: 'http://localhost:3015'
}));

// Mock translation
jest.mock('../../i18n', () => ({
  useTranslation: () => ({ t: (key: string) => key })
}));

jest.mock('../common/Toast', () => ({
  useToast: () => ({ showToast: jest.fn() })
}));

// Mock the nested components
jest.mock('../../lib/vrm/VrmRenderer', () => () => <div data-testid="vrm-renderer" />);
jest.mock('../../lib/inx/InxRenderer', () => () => <div data-testid="inx-renderer" />);
jest.mock('../../lib/glb/GlbRenderer', () => () => <div data-testid="glb-renderer" />);
jest.mock('../character/EkycStatusBadge', () => ({
    EkycStatusBadge: ({ status }: any) => <div data-testid="ekyc-badge">{String(status)}</div>
}));
jest.mock('../character/SoulStatusBadge', () => ({
    SoulStatusBadge: () => <div data-testid="soul-badge" />
}));

describe('CharacterPanel', () => {
    const mockStats = { level: 5, exp: 2500, resonance: 80, creativity: 0, fatigue: 0 };
    const defaultProps = {
        stats: mockStats,
        onOpenViewer: jest.fn(),
        isViewerOpen: false,
        modelUrl: 'test.vrm',
        avatarState: 'idle' as const,
        mode: 'vrm' as const
    };

    beforeEach(() => {
        jest.clearAllMocks();
        // Mock default successful fetch responses
        (authenticatedFetch as jest.Mock).mockImplementation(async (url: string) => {
            if (url.includes('/ekyc/status')) {
                return { ok: true, json: async () => ({ verified: false }) };
            }
            if (url.includes('/soul/status')) {
                return { ok: true, json: async () => ({ state: 'Awake', level: 5 }) };
            }
            if (url.includes('/ekyc/session')) {
                return { ok: true, json: async () => ({ session_url: 'https://verify.stripe.com/test' }) };
            }
            return { ok: false };
        });

        // Mock window.open
        window.open = jest.fn();
    });

    it('should use authenticatedFetch to get status on mount', async () => {
        render(<CharacterPanel {...defaultProps} />);

        await waitFor(() => {
            expect(authenticatedFetch).toHaveBeenCalledWith(`${API_BASE}/api/v1/ekyc/status`);
            expect(authenticatedFetch).toHaveBeenCalledWith(`${API_BASE}/api/v1/soul/status`);
        });
    });

    it('should display "本人確認を開始する" button when ekycStatus is false (unverified)', async () => {
        render(<CharacterPanel {...defaultProps} />);

        await waitFor(() => {
            expect(screen.getByTestId('ekyc-badge')).toHaveTextContent('false');
        });

        expect(screen.getByText('ekyc.startVerification')).toBeInTheDocument();
    });

    it('should NOT display verification button when ekycStatus is true (verified)', async () => {
        (authenticatedFetch as jest.Mock).mockImplementation(async (url: string) => {
            if (url.includes('/ekyc/status')) {
                return { ok: true, json: async () => ({ verified: true }) };
            }
            if (url.includes('/soul/status')) {
                return { ok: true, json: async () => ({ state: 'Awake', level: 5 }) };
            }
            return { ok: false };
        });

        render(<CharacterPanel {...defaultProps} />);

        await waitFor(() => {
            expect(screen.getByTestId('ekyc-badge')).toHaveTextContent('true');
        });

        expect(screen.queryByText('ekyc.startVerification')).not.toBeInTheDocument();
    });

    it('should create ekyc session and redirect when verification button is clicked', async () => {
        render(<CharacterPanel {...defaultProps} />);

        const verifyBtn = await screen.findByText('ekyc.startVerification');
        fireEvent.click(verifyBtn);

        await waitFor(() => {
            expect(authenticatedFetch).toHaveBeenCalledWith(`${API_BASE}/api/v1/ekyc/session`, { method: 'POST' });
            expect(window.open).toHaveBeenCalledWith('https://verify.stripe.com/test', '_blank', 'noopener,noreferrer');
        });
    });

    // U0-B2: 失敗時はトースト連発ではなくパネル内エラー+再試行を表示する
    it('should show in-panel error with retry when status fetches fail, and recover on retry', async () => {
        (authenticatedFetch as jest.Mock).mockImplementation(async () => ({ ok: false }));

        render(<CharacterPanel {...defaultProps} />);

        expect(await screen.findByText('character.loadFailed')).toBeInTheDocument();
        const retryBtn = screen.getByText('error.retry');

        // 再試行で成功したらエラー表示が消える
        (authenticatedFetch as jest.Mock).mockImplementation(async (url: string) => {
            if (url.includes('/ekyc/status')) {
                return { ok: true, json: async () => ({ verified: true }) };
            }
            if (url.includes('/soul/status')) {
                return { ok: true, json: async () => ({ state: 'Awake', level: 5 }) };
            }
            return { ok: false };
        });
        fireEvent.click(retryBtn);

        await waitFor(() => {
            expect(screen.getByTestId('ekyc-badge')).toHaveTextContent('true');
        });
        expect(screen.queryByText('character.loadFailed')).not.toBeInTheDocument();
    });
});

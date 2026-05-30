/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

import React from 'react';
import { render, screen, act } from '@testing-library/react';
import '@testing-library/jest-dom';
import { SoTProgressBar } from './SoTProgressBar';
import { useSystemVitality } from '../hooks/useSystemVitality';

jest.mock('../hooks/useSystemVitality', () => ({
    useSystemVitality: jest.fn(),
}));

jest.mock('../i18n', () => ({
    useTranslation: () => ({
        t: (key: string, options?: any) => {
            const defaults: Record<string, string> = {
                'sot.active': 'Active Session',
                'sot.round': 'Round',
                'sot.thinking': 'thinking',
                'sot.completed': 'completed',
                'sot.latestScores': 'LATEST SCORES',
                'sot.convergedEarly': 'Converged Early',
                'sot.abstentions': 'Abstentions'
            };
            return options?.defaultValue || defaults[key] || key;
        }
    })
}));

describe('SoTProgressBar', () => {
    beforeEach(() => {
        jest.clearAllMocks();
        jest.useFakeTimers();
    });

    afterEach(() => {
        jest.useRealTimers();
    });

    it('should render null when there are no SoT events', () => {
        (useSystemVitality as jest.Mock).mockReturnValue({
            events: []
        });

        const { container } = render(<SoTProgressBar />);
        expect(container.firstChild).toBeNull();
    });

    it('should render active session on SessionStart event', () => {
        (useSystemVitality as jest.Mock).mockReturnValue({
            events: [
                {
                    type: 'sot_progress',
                    data: {
                        type: 'SessionStart',
                        data: {
                            session_id: 'session-123'
                        }
                    }
                }
            ]
        });

        render(<SoTProgressBar />);

        expect(screen.getByText('Active Session')).toBeInTheDocument();
        expect(screen.getByText('Round 0')).toBeInTheDocument();
    });

    it('should display active roles, scores, and protocol during session progress', () => {
        // 注: コンポーネント内のパーサはイベント配列を .reverse() して処理するため、
        // 逆順ループ内で activeSession が作成された後に他のイベントが評価されるよう、
        // 時間軸で SessionStart -> ProtocolSelected -> Score -> RoleStart (作成契機) の順に配置します。
        (useSystemVitality as jest.Mock).mockReturnValue({
            events: [
                {
                    type: 'sot_progress',
                    data: {
                        type: 'SessionStart',
                        data: { session_id: 'session-123' }
                    }
                },
                {
                    type: 'sot_progress',
                    data: {
                        type: 'ProtocolSelected',
                        data: { session_id: 'session-123', protocol: 'Delphi-v2' }
                    }
                },
                {
                    type: 'sot_progress',
                    data: {
                        type: 'Score',
                        data: {
                            session_id: 'session-123',
                            scores: [['Precision', 4.5], ['Safety', 5.0]]
                        }
                    }
                },
                {
                    type: 'sot_progress',
                    data: {
                        type: 'RoleStart',
                        data: { session_id: 'session-123', round: 1, role: 'Critic' }
                    }
                }
            ]
        });

        render(<SoTProgressBar />);

        // プロトコルが表示されること
        expect(screen.getByText('Delphi-v2')).toBeInTheDocument();

        // ラウンド数が更新されること
        expect(screen.getByText('Round 1')).toBeInTheDocument();

        // ロールが表示されること
        expect(screen.getByText(/Critic thinking/i)).toBeInTheDocument();

        // スコアが表示されること
        expect(screen.getByText('Precision: 4.5/5')).toBeInTheDocument();
        expect(screen.getByText('Safety: 5/5')).toBeInTheDocument();
    });

    it('should display outcome when session ends, and dismiss after 5 seconds', () => {
        // 注: 逆順ループで先に SessionEnd を処理させる前に、より時間軸で「後」に位置する
        // RoleStart によって activeSession を初期化させておく必要があります。
        (useSystemVitality as jest.Mock).mockReturnValue({
            events: [
                {
                    type: 'sot_progress',
                    data: {
                        type: 'SessionStart',
                        data: { session_id: 'session-123' }
                    }
                },
                {
                    type: 'sot_progress',
                    data: {
                        type: 'SessionEnd',
                        data: { session_id: 'session-123', outcome: 'ConvergedEarly' }
                    }
                },
                {
                    type: 'sot_progress',
                    data: {
                        type: 'RoleStart',
                        data: { session_id: 'session-123', round: 1, role: 'Critic' }
                    }
                }
            ]
        });

        render(<SoTProgressBar />);

        // セッション終了ステータス (Converged Early) が表示されること
        expect(screen.getByText(/Converged Early/i)).toBeInTheDocument();

        // 4秒経過してもまだ表示されていること
        act(() => {
            jest.advanceTimersByTime(4000);
        });
        expect(screen.getByText(/Converged Early/i)).toBeInTheDocument();

        // 5秒経過すると非表示 (dismissed) になること
        act(() => {
            jest.advanceTimersByTime(1000);
        });
        expect(screen.queryByText(/Converged Early/i)).not.toBeInTheDocument();
    });
});

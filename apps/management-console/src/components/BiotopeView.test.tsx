/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

import React from 'react';
import { render, screen } from '@testing-library/react';
import '@testing-library/jest-dom';
import BiotopeView from './BiotopeView';

jest.mock('./TreasureBox', () => ({
    TreasureBox: () => <div data-testid="treasure-box">TreasureBox Mock</div>
}));

jest.mock('./common/TokenSavingsIndicator', () => ({
    TokenSavingsIndicator: ({ savedChars }: any) => <div data-testid="token-savings">Saved: {savedChars} Mock</div>
}));

jest.mock('../i18n', () => ({
    useTranslation: () => ({
        t: (key: string, options?: any) => {
            const defaults: Record<string, string> = {
                'biotope.liveVitality': 'Live Vitality',
                'sidebar.level': 'Level',
                'biotope.resonance': 'Resonance',
                'biotope.creativity': 'Creativity',
                'biotope.neuralFatigue': 'Neural Fatigue',
                'biotope.techExperience': 'Tech Exp',
                'biotope.chroniclePulse': 'Chronicle Pulse',
                'biotope.monitoringActivity': 'Monitoring Activity',
                'biotope.synergyHeartbeat': 'Synergy Heartbeat',
                'biotope.stable': 'Stable',
                'biotope.weak': 'Weak'
            };
            if (key === 'biotope.ascension') {
                return `Ascension ${options?.n || 0}`;
            }
            return options?.defaultValue || defaults[key] || key;
        }
    })
}));

describe('BiotopeView', () => {
    const mockStats = {
        level: 15,
        resonance: 85,
        creativity: 92,
        fatigue: 30,
        exp: 150,
        persona: 'default',
        health: 100,
        last_active: '2026-05-30T00:00:00Z',
    };

    it('should render all stats and subcomponents successfully', () => {
        render(
            <BiotopeView
                stats={mockStats}
                isConnected={true}
                recentEvents={[]}
                sessionSavedChars={450}
            />
        );

        // 基本的なスタッツ表示の検証
        expect(screen.getByText('Live Vitality')).toBeInTheDocument();
        expect(screen.getByText(/Level 15/i)).toBeInTheDocument();
        expect(screen.getByText(/Ascension 1/i)).toBeInTheDocument(); // Math.floor(15/10) = 1

        // メーター類
        expect(screen.getByText('Resonance')).toBeInTheDocument();
        expect(screen.getByText('85%')).toBeInTheDocument();
        expect(screen.getByText('Creativity')).toBeInTheDocument();
        expect(screen.getByText('92%')).toBeInTheDocument();
        expect(screen.getByText('Neural Fatigue')).toBeInTheDocument();
        expect(screen.getByText('30%')).toBeInTheDocument();

        // 経験値
        expect(screen.getByText(/15 Tech Exp/i)).toBeInTheDocument(); // Math.floor(150/10) = 15

        // 子コンポーネントモックの描画検証
        expect(screen.getByTestId('treasure-box')).toBeInTheDocument();
        expect(screen.getByTestId('token-savings')).toHaveTextContent('Saved: 450 Mock');

        // ハートビート
        expect(screen.getByText('Stable')).toBeInTheDocument();
    });

    it('should show "Weak" heartbeat state when disconnected', () => {
        render(
            <BiotopeView
                stats={mockStats}
                isConnected={false}
                recentEvents={[]}
            />
        );

        expect(screen.getByText('Weak')).toBeInTheDocument();
        expect(screen.queryByText('Stable')).not.toBeInTheDocument();
    });

    it('should display chronic empty state and populating events properly', () => {
        const { rerender } = render(
            <BiotopeView
                stats={mockStats}
                isConnected={true}
                recentEvents={[]}
                sessionSavedChars={450}
            />
        );

        // 空イベント状態の監視メッセージ
        expect(screen.getByText('Monitoring Activity')).toBeInTheDocument();

        // イベントありの状態で再レンダリング
        const mockEvents = [
            {
                id: 'e1',
                type: 'evolution',
                title: 'Level Up',
                desc: 'Agent level increased to 15!',
                color: 'var(--accent-purple)',
                icon: '✨',
                timestamp: 123456789
            },
            {
                id: 'e2',
                type: 'cortex',
                title: 'Thought Distillation',
                desc: 'Distilled 12 high-quality insights.',
                color: 'var(--accent-cyan)',
                icon: '🧠',
                timestamp: 123456790
            }
        ];

        rerender(
            <BiotopeView
                stats={mockStats}
                isConnected={true}
                recentEvents={mockEvents}
                sessionSavedChars={450}
            />
        );

        expect(screen.queryByText('Monitoring Activity')).not.toBeInTheDocument();

        // 精密一致ではなく正規表現による部分一致検索を行う
        expect(screen.getByText(/Level Up/i)).toBeInTheDocument();
        expect(screen.getByText(/Agent level increased to 15!/i)).toBeInTheDocument();
        expect(screen.getByText(/Thought Distillation/i)).toBeInTheDocument();
        expect(screen.getByText(/Distilled 12 high-quality insights/i)).toBeInTheDocument();
    });
});

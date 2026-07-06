/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import { render, screen, fireEvent } from '@testing-library/react';
import { A2uiRenderer } from './A2uiRenderer';
import { a2uiSurfaceStore } from '../lib/a2uiSurfaceStore';


jest.mock('../config', () => ({
  API_BASE: 'http://localhost:3015',
  initApiBase: jest.fn().mockResolvedValue(undefined),
  APP_VERSION: 'v1.0.2'
}));

jest.mock('./common/Toast', () => ({
  useToast: () => ({ showToast: jest.fn() })
}));

jest.mock('../i18n', () => ({
  useTranslation: () => ({ t: () => undefined })
}));

jest.mock('../hooks/useAgentIdentity', () => ({
  useAgentIdentity: () => ({ agentId: 'agent-001', isEkycVerified: false }),
}));

jest.mock('../lib/auth', () => ({
  authenticatedFetch: jest.fn().mockResolvedValue({
    ok: true,
    json: async () => ({ balance: 4200 }),
  }),
}));

describe('A2uiRenderer - Generative UI Components', () => {
    beforeEach(() => {
        a2uiSurfaceStore.clear();
    });

    it('renders a progress bar', () => {
        const env: any = {
            id: '1', timestamp: 1, version: '1.0', signature: 'sig',
            type: 'createSurface',
            surface: {
                id: 'surf-1',
                components: [{
                    type: 'progressBar',
                    props: { progress: 75, label: 'Deploying' }
                }]
            }
        };
        render(<A2uiRenderer envelope={env} />);
        expect(screen.getByText('Deploying')).toBeInTheDocument();
        expect(screen.getByRole('progressbar')).toBeInTheDocument();
        expect(screen.getByRole('progressbar')).toHaveAttribute('aria-valuenow', '75');
    });

    it('renders an alert', () => {
        const env: any = {
            id: '2', timestamp: 1, version: '1.0', signature: 'sig',
            type: 'createSurface',
            surface: {
                id: 'surf-2',
                components: [{
                    type: 'alert',
                    props: { severity: 'warning', message: 'Disk space low' }
                }]
            }
        };
        render(<A2uiRenderer envelope={env} />);
        expect(screen.getByText('Disk space low')).toBeInTheDocument();
    });

    it('renders a card', () => {
        const env: any = {
            id: '3', timestamp: 1, version: '1.0', signature: 'sig',
            type: 'createSurface',
            surface: {
                id: 'surf-3',
                components: [{
                    type: 'card',
                    props: { title: 'User Info', content: 'Details here' }
                }]
            }
        };
        render(<A2uiRenderer envelope={env} />);
        expect(screen.getByText('User Info')).toBeInTheDocument();
        expect(screen.getByText('Details here')).toBeInTheDocument();
    });

    it('renders a card with nav button children', () => {
        const env: any = {
            type: 'createSurface',
            surface: {
                id: 'surf-nav',
                components: [{
                    type: 'card',
                    props: { title: 'Logs', content: 'Open audit' },
                    children: [{
                        type: 'button',
                        props: { label: 'View Logs', action: 'navigate:audit' },
                        children: [],
                    }],
                }],
            },
        };
        const dispatchSpy = jest.spyOn(window, 'dispatchEvent');
        render(<A2uiRenderer envelope={env} />);
        fireEvent.click(screen.getByText('View Logs'));
        expect(dispatchSpy).toHaveBeenCalledWith(
            expect.objectContaining({ type: 'a2ui-navigate', detail: { tab: 'audit' } })
        );
        dispatchSpy.mockRestore();
    });

    it('renders a code block', () => {
        const env: any = {
            id: '4', timestamp: 1, version: '1.0', signature: 'sig',
            type: 'createSurface',
            surface: {
                id: 'surf-4',
                components: [{
                    type: 'codeBlock',
                    props: { code: 'console.log("hello");', language: 'javascript' }
                }]
            }
        };
        render(<A2uiRenderer envelope={env} />);
        expect(screen.getByText('console.log("hello");')).toBeInTheDocument();
    });

    it('renders a chart', () => {
        const env: any = {
            id: '5', timestamp: 1, version: '1.0', signature: 'sig',
            type: 'createSurface',
            surface: {
                id: 'surf-5',
                components: [{
                    type: 'chart',
                    props: { title: 'CPU Usage', metrics: [{ label: 'Core 1', value: 50 }] }
                }]
            }
        };
        render(<A2uiRenderer envelope={env} />);
        expect(screen.getByText('CPU Usage')).toBeInTheDocument();
        expect(screen.getByText('Core 1')).toBeInTheDocument();
    });

    it('renders VoiceStore component', () => {
        const env: any = {
            id: '6', timestamp: 1, version: '1.0', signature: 'sig',
            type: 'createSurface',
            surface: {
                id: 'surf-6',
                components: [{
                    type: 'voiceStore',
                    props: {}
                }]
            }
        };
        render(<A2uiRenderer envelope={env} />);
        expect(screen.getByText('Creator Registry & Voice Store')).toBeInTheDocument();
    });

    it('renders LoraMarket component', () => {
        const env: any = {
            id: '7', timestamp: 1, version: '1.0', signature: 'sig',
            type: 'createSurface',
            surface: {
                id: 'surf-7',
                components: [{
                    type: 'loraMarket',
                    props: {}
                }]
            }
        };
        render(<A2uiRenderer envelope={env} />);
        expect(screen.getByText('Configure and launch self-supervised domain adaptation. The Autotuner will optimize hyperparameters based on the target dataset.')).toBeInTheDocument();
    });

    it('renders walletWidget with KC balance', async () => {
        const env: any = {
            type: 'createSurface',
            surface: {
                id: 'surf-wallet',
                components: [{ type: 'walletWidget', props: { label: 'My KC' }, children: [] }],
            },
        };
        render(<A2uiRenderer envelope={env} />);
        expect(await screen.findByText('4,200 KC')).toBeInTheDocument();
    });

    it('renders marketplaceItem', () => {
        const env: any = {
            type: 'createSurface',
            surface: {
                id: 'surf-market',
                components: [{
                    type: 'marketplaceItem',
                    props: { title: 'Voice Pack', price: 500, currency: 'KC', description: 'Premium voice' },
                    children: [],
                }],
            },
        };
        render(<A2uiRenderer envelope={env} />);
        expect(screen.getByText('Voice Pack')).toBeInTheDocument();
        expect(screen.getByText('500 KC')).toBeInTheDocument();
        expect(screen.getByText('View Store')).toBeInTheDocument();
    });

    it('updateComponents replaces surface components', () => {
        const create: any = {
            type: 'createSurface',
            surface: {
                id: 'surf-upd',
                components: [{ type: 'text', props: { content: 'Before' }, children: [] }],
            },
        };
        const { rerender } = render(<A2uiRenderer envelope={create} />);
        expect(screen.getByText('Before')).toBeInTheDocument();

        rerender(<A2uiRenderer envelope={{
            type: 'updateComponents',
            surfaceId: 'surf-upd',
            components: [{ type: 'text', props: { content: 'After' }, children: [] }],
        }} />);
        rerender(<A2uiRenderer envelope={create} />);
        expect(screen.getByText('After')).toBeInTheDocument();
        expect(screen.queryByText('Before')).not.toBeInTheDocument();
    });

    it('deleteSurface removes rendered surface', () => {
        const create: any = {
            type: 'createSurface',
            surface: {
                id: 'surf-del',
                components: [{ type: 'text', props: { content: 'Gone soon' }, children: [] }],
            },
        };
        const { rerender } = render(<A2uiRenderer envelope={create} />);
        expect(screen.getByText('Gone soon')).toBeInTheDocument();

        rerender(<A2uiRenderer envelope={{ type: 'deleteSurface', surfaceId: 'surf-del' }} />);
        rerender(<A2uiRenderer envelope={create} />);
        expect(screen.queryByText('Gone soon')).not.toBeInTheDocument();
    });
});

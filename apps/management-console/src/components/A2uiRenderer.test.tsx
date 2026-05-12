import { render, screen } from '@testing-library/react';
import { A2uiRenderer } from './A2uiRenderer';


jest.mock('../config', () => ({
  API_BASE: 'http://localhost:3015',
  initApiBase: jest.fn().mockResolvedValue(undefined),
  APP_VERSION: 'v1.0.2'
}));

describe('A2uiRenderer - Generative UI Components', () => {
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
        // Assuming i18n mock returns the key itself or the component has this text
        expect(screen.getByText('Configure and launch self-supervised domain adaptation. The Autotuner will optimize hyperparameters based on the target dataset.')).toBeInTheDocument();
    });
});

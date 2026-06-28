/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import { render, screen, act } from '@testing-library/react';
import * as TokenHealthModule from './hooks/useTokenHealth';

jest.mock('./config', () => ({
  API_BASE: 'http://localhost:3015',
  initApiBase: jest.fn().mockResolvedValue(undefined),
  APP_VERSION: 'v1.0.2'
}));

import App from './App';

// Mock scrollIntoView which is not implemented in JSDOM
window.HTMLElement.prototype.scrollIntoView = jest.fn();


// Mock framer-motion to skip animation delays using a Proxy to handle any motion.elem
jest.mock('framer-motion', () => {
    const React = jest.requireActual('react');
    return {
        __esModule: true,
        motion: new Proxy({}, {
            get: (_target: object, prop: string) => {
                const MotionComponent = ({ children, initial, animate, exit, transition, whileHover, whileTap, layoutId, ...props }: Record<string, unknown>) =>
                    React.createElement(prop, props, children as React.ReactNode);
                MotionComponent.displayName = `motion.${prop}`;
                return MotionComponent;
            }
        }),
        AnimatePresence: ({ children }: { children: React.ReactNode }) => children,
        useSpring: (v: unknown) => v,
        useTransform: (v: unknown, fn?: (val: unknown) => unknown) => fn ? fn(v) : v,
        useMotionValue: (v: unknown) => v
    };
});

// Mock all internal contexts and lazily loaded components
jest.mock('./hooks/useTokenHealth', () => ({
  __esModule: true,
  useTokenHealth: jest.fn()
}));
jest.mock('./i18n', () => ({
  __esModule: true,
  useTranslation: () => ({ t: (k: string) => k }),
  useLanguage: () => ({ lang: 'en', setLang: jest.fn() })
}));
const mockUseSystemVitality = jest.fn();
jest.mock('./hooks/useSystemVitality', () => ({
  __esModule: true,
  useSystemVitality: () => mockUseSystemVitality()
}));
jest.mock('./hooks/useAvatarState', () => ({
  __esModule: true,
  useAvatarState: () => 'idle'
}));
jest.mock('./hooks/useDisplayMode', () => ({
  __esModule: true,
  useDisplayMode: () => ({ mode: 'lite', setMode: jest.fn() })
}));
jest.mock('./components/home/CharacterPanel', () => ({
  __esModule: true,
  default: () => <div data-testid="character-panel-mock">mock</div>
}));

jest.mock('./components/TaskApprovalOverlay', () => ({
  __esModule: true,
  default: () => <div data-testid="task-approval-mock">mock</div>
}));

jest.mock('./components/SetupWizard', () => ({
  __esModule: true,
  default: () => <div data-testid="setup-wizard-mock">SetupWizard Mock</div>
}));
jest.mock('./components/LoginScreen', () => ({
  __esModule: true,
  default: () => <div data-testid="login-screen-mock">LoginScreen Mock</div>
}));

jest.mock('./components/WorkflowBuilder', () => ({
  __esModule: true,
  default: () => <div data-testid="workflow-builder-mock">WorkflowBuilder Mock</div>
}));

jest.mock('./components/SetupWizard', () => ({
  __esModule: true,
  default: () => <div data-testid="setup-wizard-mock">SetupWizard Mock</div>
}));

jest.mock('./components/BiotopeView', () => ({
  __esModule: true,
  default: ({ recentEvents }: any) => (
    <div data-testid="biotope-view-mock">
      {recentEvents.map((event: any) => (
        <div key={event.id}>{event.title}</div>
      ))}
    </div>
  )
}));

jest.mock('./hooks/useTreasure', () => ({
  __esModule: true,
  useTreasure: () => ({ items: [], loading: false, claimDrop: jest.fn() })
}));

jest.mock('./hooks/useViewMode', () => ({
  __esModule: true,
  useViewMode: () => ({ viewMode: 'advanced' })
}));
jest.mock('./lib/auth', () => ({
  __esModule: true,
  isAuthenticated: () => true,
  authenticatedFetch: jest.fn(() => Promise.resolve({ ok: true, json: () => Promise.resolve([]) }))
}));
jest.mock('./hooks/AvatarContext', () => ({
  __esModule: true,
  AvatarProvider: ({ children }: any) => <>{children}</>,
  useAvatarCharacter: () => ({ character: 'female', setCharacter: jest.fn(), proportion: 'taller', setProportion: jest.fn(), getAssetPath: jest.fn().mockReturnValue('mock-path') })
}));

// Type-safe reference to the mocked hook
const mockUseTokenHealth = TokenHealthModule.useTokenHealth as jest.MockedFunction<typeof TokenHealthModule.useTokenHealth>;

describe('App - Global Token Health', () => {
    const originalFetch = window.fetch;

    beforeEach(() => {
        jest.clearAllMocks();
        mockUseSystemVitality.mockReturnValue({
            events: [],
            lastEvent: null,
            connectionStatus: 'connected',
            toggleConnection: jest.fn(),
            lastPingMs: 0
        });
        window.fetch = jest.fn(() => Promise.resolve({
            ok: true,
            json: () => Promise.resolve({ mode: 'normal' })
        })) as jest.Mock;
    });

    afterEach(() => {
        window.fetch = originalFetch;
        // Always restore isAuthenticated to default (true) for test isolation
        const authModule = jest.requireMock('./lib/auth') as { isAuthenticated: jest.Mock };
        authModule.isAuthenticated = jest.fn(() => true);
    });

    it('should display global token expiration alert when isExpired is true', async () => {
        // Arrange
        mockUseTokenHealth.mockReturnValue({
            isExpired: true,
            lastChecked: null,
            checkHealth: jest.fn(),
            dismiss: jest.fn()
        });

        // Act
        render(<App />);

        // Wait to finish render and assert
        await screen.findByText('session.expired');
        expect(screen.getByText('session.expired')).toBeInTheDocument();

        // Wait for async component mounts and fetches to finish
        await act(async () => {
            await new Promise(resolve => setTimeout(resolve, 0));
        });
    });

    it('should not display global token expiration alert when isExpired is false', async () => {
        // Arrange
        mockUseTokenHealth.mockReturnValue({
            isExpired: false,
            lastChecked: null,
            checkHealth: jest.fn(),
            dismiss: jest.fn()
        });

        // Act
        render(<App />);

        // Assert
        expect(screen.queryByText('session.expired')).not.toBeInTheDocument();

        // Wait for async component mounts and fetches to finish
        await act(async () => {
            await new Promise(resolve => setTimeout(resolve, 0));
        });
    });

    it('should render SetupWizard when backend returns mode: setup', async () => {
        // Arrange
        window.fetch = jest.fn(() => Promise.resolve({
            ok: true,
            json: () => Promise.resolve({ mode: 'setup' })
        })) as jest.Mock;

        mockUseTokenHealth.mockReturnValue({
            isExpired: false,
            lastChecked: null,
            checkHealth: jest.fn(),
            dismiss: jest.fn()
        });

        // Act
        render(<App />);

        // Wait to finish render
        await act(async () => {
            await new Promise(resolve => setTimeout(resolve, 10));
        });

        // Assert
        expect(screen.getByTestId('setup-wizard-mock')).toBeInTheDocument();
    });

    it('should render normal mode when backend returns mode: normal', async () => {
        // Arrange
        window.fetch = jest.fn(() => Promise.resolve({
            ok: true,
            json: () => Promise.resolve({ mode: 'normal' })
        })) as jest.Mock;

        mockUseTokenHealth.mockReturnValue({
            isExpired: false,
            lastChecked: null,
            checkHealth: jest.fn(),
            dismiss: jest.fn()
        });

        // Act
        render(<App />);

        // Wait to finish render
        await act(async () => {
            await new Promise(resolve => setTimeout(resolve, 10));
        });

        // Assert
        expect(screen.queryByTestId('setup-wizard-mock')).not.toBeInTheDocument();
        expect(screen.queryByTestId('login-screen-mock')).not.toBeInTheDocument();
    });

    it('should render LoginScreen when mode is normal and user is unauthenticated', async () => {
        // Arrange: override isAuthenticated to return false
        const authModule = jest.requireMock('./lib/auth') as { isAuthenticated: jest.Mock };
        authModule.isAuthenticated = jest.fn(() => false);

        window.fetch = jest.fn(() => Promise.resolve({
            ok: true,
            json: () => Promise.resolve({ mode: 'normal' })
        })) as jest.Mock;

        mockUseTokenHealth.mockReturnValue({
            isExpired: false,
            lastChecked: null,
            checkHealth: jest.fn(),
            dismiss: jest.fn()
        });

        // Act
        render(<App />);

        // Wait to finish render
        await act(async () => {
            await new Promise(resolve => setTimeout(resolve, 10));
        });

        // Assert
        expect(screen.getByTestId('login-screen-mock')).toBeInTheDocument();
        expect(screen.queryByTestId('setup-wizard-mock')).not.toBeInTheDocument();
    });

    it('should fall back to Normal mode when bootstrap fetch fails', async () => {
        // Arrange: simulate a network error
        window.fetch = jest.fn(() => Promise.reject(new Error('Network error'))) as jest.Mock;

        mockUseTokenHealth.mockReturnValue({
            isExpired: false,
            lastChecked: null,
            checkHealth: jest.fn(),
            dismiss: jest.fn()
        });

        // Act
        render(<App />);

        // Wait for error handler to run
        await act(async () => {
            await new Promise(resolve => setTimeout(resolve, 10));
        });

        // Assert: should not be stuck on loading, and should not show SetupWizard
        expect(screen.queryByTestId('setup-wizard-mock')).not.toBeInTheDocument();
    });
});

describe('App - SSE Biome Events', () => {
    const originalFetch = window.fetch;

    beforeEach(() => {
        jest.clearAllMocks();
        mockUseSystemVitality.mockReturnValue({
            events: [],
            lastEvent: null,
            connectionStatus: 'connected',
            toggleConnection: jest.fn(),
            lastPingMs: 0
        });
        window.fetch = jest.fn(() => Promise.resolve({
            ok: true,
            json: () => Promise.resolve({ mode: 'normal' })
        })) as jest.Mock;
    });

    afterEach(() => {
        window.fetch = originalFetch;
    });

    it('should process biome_evolution event and add it to recent events log', async () => {
        // Arrange
        mockUseSystemVitality.mockReturnValue({
            events: [],
            lastEvent: {
                type: 'biome_evolution',
                data: { generation: 20, rarity: 'Legendary', message: 'Specimen mutated!' }
            },
            connectionStatus: 'connected',
            toggleConnection: jest.fn(),
            lastPingMs: 0
        });

        mockUseTokenHealth.mockReturnValue({
            isExpired: false,
            lastChecked: null,
            checkHealth: jest.fn(),
            dismiss: jest.fn()
        });

        // Act
        render(<App />);

        // Switch to biotope/dashboard tab where recent events log is displayed
        const dashboardTab = await screen.findByText('nav.biotope');
        act(() => {
            dashboardTab.click();
        });

        // Assert
        await screen.findByText('event.biomeEvolution');
        expect(screen.getByText('event.biomeEvolution')).toBeInTheDocument();
    });

    it('should process crisis_prediction event and add it to recent events log', async () => {
        // Arrange
        mockUseSystemVitality.mockReturnValue({
            events: [],
            lastEvent: {
                type: 'crisis_prediction',
                data: { crisis_type: 'meteor', seconds_remaining: 1800, description: 'Meteor storm detected' }
            },
            connectionStatus: 'connected',
            toggleConnection: jest.fn(),
            lastPingMs: 0
        });

        mockUseTokenHealth.mockReturnValue({
            isExpired: false,
            lastChecked: null,
            checkHealth: jest.fn(),
            dismiss: jest.fn()
        });

        // Act
        render(<App />);

        // Switch to biotope/dashboard tab where recent events log is displayed
        const dashboardTab = await screen.findByText('nav.biotope');
        act(() => {
            dashboardTab.click();
        });

        // Assert
        await screen.findByText('event.crisisPrediction');
        expect(screen.getByText('event.crisisPrediction')).toBeInTheDocument();
    });

    it('should navigate to workflow-builder and render WorkflowBuilder component', async () => {
        // Arrange
        mockUseTokenHealth.mockReturnValue({
            isExpired: false,
            lastChecked: null,
            checkHealth: jest.fn(),
            dismiss: jest.fn()
        });

        // Act
        render(<App />);

        // Wait to finish render
        await act(async () => {
            await new Promise(resolve => setTimeout(resolve, 10));
        });

        const workflowTab = await screen.findByText('nav.workflowBuilder');
        expect(workflowTab).toBeInTheDocument();

        act(() => {
            workflowTab.click();
        });

        // Assert
        await screen.findByTestId('workflow-builder-mock');
        expect(screen.getByTestId('workflow-builder-mock')).toBeInTheDocument();
        expect(screen.getByText('page.workflowBuilder')).toBeInTheDocument();
    });
});

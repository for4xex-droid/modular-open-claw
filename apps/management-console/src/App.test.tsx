import { render, screen, act } from '@testing-library/react';
import * as TokenHealthModule from './hooks/useTokenHealth';

jest.mock('./config', () => ({
  API_BASE: 'http://localhost:3015',
  initApiBase: jest.fn().mockResolvedValue(undefined),
  APP_VERSION: 'v1.0.2'
}));

import App from './App';

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
jest.mock('./hooks/useSystemVitality', () => ({
  __esModule: true,
  useSystemVitality: () => ({ events: [], lastEvent: null, connectionStatus: 'connected', toggleConnection: jest.fn(), lastPingMs: 0 })
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

jest.mock('./components/BiotopeView', () => ({
  __esModule: true,
  default: () => <div data-testid="biotope-view-mock">mock</div>
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
  useAvatarCharacter: () => ({ character: 'female', setCharacter: jest.fn(), proportion: 'taller', setProportion: jest.fn() })
}));

// Type-safe reference to the mocked hook
const mockUseTokenHealth = TokenHealthModule.useTokenHealth as jest.MockedFunction<typeof TokenHealthModule.useTokenHealth>;

describe('App - Global Token Health', () => {
    const originalFetch = window.fetch;

    beforeEach(() => {
        jest.clearAllMocks();
        window.fetch = jest.fn(() => Promise.resolve({
            ok: true,
            json: () => Promise.resolve({ verified: false })
        })) as jest.Mock;
    });

    afterEach(() => {
        window.fetch = originalFetch;
    });

    it('RED: should display global token expiration alert when isExpired is true', async () => {
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
        await screen.findByText(/Session expired/i);
        expect(screen.getByText(/Session expired/i)).toBeInTheDocument();

        // Wait for async component mounts and fetches to finish
        await act(async () => {
            await new Promise(resolve => setTimeout(resolve, 0));
        });
    });

    it('GREEN: should not display global token expiration alert when isExpired is false', async () => {
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
        expect(screen.queryByText(/Session expired/i)).not.toBeInTheDocument();

        // Wait for async component mounts and fetches to finish
        await act(async () => {
            await new Promise(resolve => setTimeout(resolve, 0));
        });
    });
});

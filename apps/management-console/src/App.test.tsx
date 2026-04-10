import React from 'react';
import { render, screen, act } from '@testing-library/react';
import App from './App';
import * as TokenHealthModule from './hooks/useTokenHealth';

// Mock all internal contexts and lazily loaded components
jest.mock('./hooks/useTokenHealth', () => ({
  useTokenHealth: jest.fn()
}));
jest.mock('./i18n', () => ({
  useTranslation: () => ({ t: (k: string) => k }),
  useLanguage: () => ({ lang: 'en', setLang: jest.fn() })
}));
jest.mock('./hooks/useSystemVitality', () => ({
  useSystemVitality: () => ({ events: [], lastEvent: null, connectionStatus: 'connected', toggleConnection: jest.fn(), lastPingMs: 0 })
}));
jest.mock('./hooks/useAvatarState', () => ({
  useAvatarState: () => 'idle'
}));
jest.mock('./hooks/useDisplayMode', () => ({
  useDisplayMode: () => ({ mode: 'lite', setMode: jest.fn() })
}));
jest.mock('./hooks/useViewMode', () => ({
  useViewMode: () => ({ viewMode: 'advanced' })
}));
jest.mock('./lib/auth', () => ({
  isAuthenticated: () => true
}));
jest.mock('./hooks/AvatarContext', () => ({
  AvatarProvider: ({ children }: any) => <>{children}</>,
  useAvatarCharacter: () => ({ character: 'female', setCharacter: jest.fn(), proportion: 'taller', setProportion: jest.fn() })
}));

describe('App - Global Token Health', () => {
    beforeEach(() => {
        jest.clearAllMocks();
    });

    it('RED: should display global token expiration alert when isExpired is true', () => {
        // Arrange
        // @ts-ignore
        TokenHealthModule.useTokenHealth.mockReturnValue({
            isExpired: true,
            checkHealth: jest.fn(),
            dismiss: jest.fn()
        });

        // Act
        render(<App />);

        // Assert
        expect(screen.getByText(/Session expired/i)).toBeInTheDocument();
    });

    it('GREEN: should not display global token expiration alert when isExpired is false', () => {
        // Arrange
        // @ts-ignore
        TokenHealthModule.useTokenHealth.mockReturnValue({
            isExpired: false,
            checkHealth: jest.fn(),
            dismiss: jest.fn()
        });

        // Act
        render(<App />);

        // Assert
        expect(screen.queryByText(/Session expired/i)).not.toBeInTheDocument();
    });
});

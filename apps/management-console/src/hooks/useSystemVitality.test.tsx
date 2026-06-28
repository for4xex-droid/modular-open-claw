/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import React from 'react';
import { render, act, screen, fireEvent } from '@testing-library/react';
import { SystemVitalityProvider, useSystemVitality } from './useSystemVitality';
import { fetchEventSource } from '@microsoft/fetch-event-source';

jest.mock('@microsoft/fetch-event-source', () => ({
    fetchEventSource: jest.fn()
}));

jest.mock('../config', () => ({
    API_BASE: 'http://localhost:3000'
}));

jest.mock('../lib/auth', () => ({
    getAuthHeaders: () => ({ Authorization: 'Bearer test' })
}));

const TestComponent = () => {
    const { events, connectionStatus, toggleConnection, lastPingMs } = useSystemVitality();
    
    return (
        <div>
            <div data-testid="status">{connectionStatus}</div>
            <div data-testid="ping">{lastPingMs !== null ? lastPingMs : 'none'}</div>
            <div data-testid="event-count">{events.length}</div>
            <button data-testid="toggle-btn" onClick={toggleConnection}>Toggle</button>
        </div>
    );
};

describe('useSystemVitality', () => {
    let mockFetchEventSource: jest.Mock;
    
    beforeEach(() => {
        jest.clearAllMocks();
        mockFetchEventSource = fetchEventSource as jest.Mock;
        
        // Mock to immediately call onopen
        mockFetchEventSource.mockImplementation(async (url, options) => {
            if (options.onopen) {
                await options.onopen({ ok: true, status: 200 } as Response);
            }
        });
    });

    it('should establish SSE connection and set status to connected', async () => {
        await act(async () => {
            render(
                <SystemVitalityProvider>
                    <TestComponent />
                </SystemVitalityProvider>
            );
        });

        expect(screen.getByTestId('status').textContent).toBe('connected');
        expect(mockFetchEventSource).toHaveBeenCalledTimes(1);
    });

    it('should receive and parse valid events', async () => {
        let onMessageCallback: any;
        
        mockFetchEventSource.mockImplementation(async (url, options) => {
            if (options.onopen) await options.onopen({ ok: true, status: 200 } as Response);
            onMessageCallback = options.onmessage;
        });

        await act(async () => {
            render(
                <SystemVitalityProvider>
                    <TestComponent />
                </SystemVitalityProvider>
            );
        });

        // Trigger an event
        act(() => {
            onMessageCallback({ event: 'level_up', data: JSON.stringify({ level: 2 }) });
        });

        expect(screen.getByTestId('event-count').textContent).toBe('1');
    });

    it('should ignore invalid quality_gate events', async () => {
        let onMessageCallback: any;
        
        mockFetchEventSource.mockImplementation(async (url, options) => {
            if (options.onopen) await options.onopen({ ok: true, status: 200 } as Response);
            onMessageCallback = options.onmessage;
        });

        await act(async () => {
            render(
                <SystemVitalityProvider>
                    <TestComponent />
                </SystemVitalityProvider>
            );
        });

        act(() => {
            // Missing score and passed boolean
            onMessageCallback({ event: 'quality_gate', data: JSON.stringify({ invalid: true }) });
        });

        expect(screen.getByTestId('event-count').textContent).toBe('0');
    });

    it('should calculate RTT on ping event', async () => {
        let onMessageCallback: any;
        
        mockFetchEventSource.mockImplementation(async (url, options) => {
            if (options.onopen) await options.onopen({ ok: true, status: 200 } as Response);
            onMessageCallback = options.onmessage;
        });

        await act(async () => {
            render(
                <SystemVitalityProvider>
                    <TestComponent />
                </SystemVitalityProvider>
            );
        });

        act(() => {
            onMessageCallback({ event: 'ping', data: new Date(Date.now() - 50).toISOString() });
        });

        expect(screen.getByTestId('ping').textContent).not.toBe('none');
    });

    it('should toggle connection pause state', async () => {
        await act(async () => {
            render(
                <SystemVitalityProvider>
                    <TestComponent />
                </SystemVitalityProvider>
            );
        });

        expect(screen.getByTestId('status').textContent).toBe('connected');

        act(() => {
            fireEvent.click(screen.getByTestId('toggle-btn'));
        });

        expect(screen.getByTestId('status').textContent).toBe('paused');
    });

    it('should listen to custom window events', async () => {
        await act(async () => {
            render(
                <SystemVitalityProvider>
                    <TestComponent />
                </SystemVitalityProvider>
            );
        });

        act(() => {
            const event = new CustomEvent('aiome_vitality_event', {
                detail: { type: 'karma_update', data: { amount: 10 } }
            });
            window.dispatchEvent(event);
        });

        expect(screen.getByTestId('event-count').textContent).toBe('1');
    });
});

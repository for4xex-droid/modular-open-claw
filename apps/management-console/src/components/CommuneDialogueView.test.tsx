import React from 'react';
import { render, screen, fireEvent, waitFor, act } from '@testing-library/react';
import CommuneDialogueView from './CommuneDialogueView';
import { authenticatedFetch } from '../lib/auth';

jest.mock('../lib/auth', () => ({
    authenticatedFetch: jest.fn()
}));

jest.mock('../config', () => ({
    API_BASE: 'http://localhost:3000'
}));

jest.mock('../i18n', () => ({
    useTranslation: () => ({ t: (key: string) => key })
}));

// Mock framer-motion to avoid animation issues
jest.mock('framer-motion', () => ({
    motion: {
        div: ({ children, ...props }: any) => <div {...props}>{children}</div>
    },
    AnimatePresence: ({ children }: any) => <>{children}</>
}));

// Mock lucide-react icons
jest.mock('lucide-react', () => ({
    Wifi: () => <div data-testid="icon-wifi"></div>,
    Play: () => <div data-testid="icon-play"></div>,
    Square: () => <div data-testid="icon-square"></div>,
    User: () => <div data-testid="icon-user"></div>,
    Bot: () => <div data-testid="icon-bot"></div>,
    History: () => <div data-testid="icon-history"></div>,
    Target: () => <div data-testid="icon-target"></div>,
    MessageSquare: () => <div data-testid="icon-message"></div>,
    Network: () => <div data-testid="icon-network"></div>
}));

describe('CommuneDialogueView Component', () => {
    let mockFetch: jest.Mock;

    beforeEach(() => {
        jest.clearAllMocks();
        mockFetch = authenticatedFetch as jest.Mock;

        // Mock window.HTMLElement.prototype.scrollIntoView or scrollTop if needed
        Object.defineProperty(HTMLElement.prototype, 'scrollTop', {
            configurable: true,
            get() { return 0; },
            set(v) {}
        });
        Object.defineProperty(HTMLElement.prototype, 'scrollHeight', {
            configurable: true,
            get() { return 1000; }
        });
        jest.useFakeTimers();
    });

    afterEach(() => {
        jest.useRealTimers();
    });

    const mockMessages = [
        {
            id: 1,
            sender_pubkey: 'self',
            recipient_pubkey: 'PEER_NODE',
            topic_id: 'general_deliberation',
            content: 'Hello from local intelligence',
            created_at: '2026-05-14T10:00:00Z'
        },
        {
            id: 2,
            sender_pubkey: 'PEER_NODE_DEFAULT_B',
            recipient_pubkey: 'self',
            topic_id: 'general_deliberation',
            content: 'Hello from peer',
            created_at: '2026-05-14T10:01:00Z'
        }
    ];

    const mockStatusStopped = {
        running: false,
        config: null
    };

    const mockStatusRunning = {
        running: true,
        config: {
            topic_id: 'custom_topic',
            peer_pubkey: 'CUSTOM_PEER',
            interval_secs: 15,
            max_rounds: 20
        }
    };

    it('renders empty state when no messages', async () => {
        mockFetch.mockImplementation((url) => {
            if (url.includes('/api/commune/list')) return Promise.resolve({ ok: true, json: async () => [] });
            if (url.includes('/api/commune/autonomous/status')) return Promise.resolve({ ok: true, json: async () => mockStatusStopped });
            return Promise.resolve({ ok: true, json: async () => [] });
        });

        render(<CommuneDialogueView />);

        expect(screen.getByText('commune.dialogueStream')).toBeTruthy();
        
        await waitFor(() => {
            expect(screen.getByText('commune.waitingMessages')).toBeTruthy();
        });
    });

    it('fetches and displays messages correctly', async () => {
        mockFetch.mockImplementation((url) => {
            if (url.includes('/api/commune/list')) return Promise.resolve({ ok: true, json: async () => mockMessages });
            if (url.includes('/api/commune/autonomous/status')) return Promise.resolve({ ok: true, json: async () => mockStatusStopped });
            return Promise.resolve({ ok: true, json: async () => [] });
        });

        render(<CommuneDialogueView />);

        await waitFor(() => {
            expect(screen.getByText('Hello from local intelligence')).toBeTruthy();
            expect(screen.getByText('Hello from peer')).toBeTruthy();
        });

        // Ensure both self and peer identifiers are rendered
        expect(screen.getByText('commune.localIntelligence')).toBeTruthy();
        expect(screen.getByText(/commune\.peer.*PEER_NOD/)).toBeTruthy();
    });

    it('handles starting the autonomous engine', async () => {
        let isRunning = false;
        mockFetch.mockImplementation((url, options) => {
            if (url.includes('/api/commune/list')) return Promise.resolve({ ok: true, json: async () => [] });
            if (url.includes('/api/commune/autonomous/status')) {
                return Promise.resolve({ ok: true, json: async () => isRunning ? mockStatusRunning : mockStatusStopped });
            }
            if (url.includes('/api/commune/autonomous/start') && options?.method === 'POST') {
                isRunning = true;
                return Promise.resolve({ ok: true });
            }
            return Promise.resolve({ ok: true, json: async () => [] });
        });

        render(<CommuneDialogueView />);

        // Wait for initial load
        await waitFor(() => {
            expect(screen.getByText('commune.startDialogue')).toBeTruthy();
        });

        // Change inputs
        const peerInput = screen.getAllByRole('textbox')[0];
        const topicInput = screen.getAllByRole('textbox')[1];
        
        fireEvent.change(peerInput, { target: { value: 'NEW_PEER' } });
        fireEvent.change(topicInput, { target: { value: 'new_topic' } });

        // Click Start
        const startBtn = screen.getByText('commune.startDialogue');
        
        await act(async () => {
            fireEvent.click(startBtn);
        });

        // Should call start API with new params
        expect(mockFetch).toHaveBeenCalledWith(
            'http://localhost:3000/api/commune/autonomous/start',
            expect.objectContaining({
                method: 'POST',
                body: expect.stringContaining('"topic_id":"new_topic"')
            })
        );
        expect(mockFetch).toHaveBeenCalledWith(
            'http://localhost:3000/api/commune/autonomous/start',
            expect.objectContaining({
                method: 'POST',
                body: expect.stringContaining('"peer_pubkey":"NEW_PEER"')
            })
        );

        // After start, should show Stop button
        await waitFor(() => {
            expect(screen.getByText(/commune.stopLoop|Stop Autonomous Loop/)).toBeTruthy();
            expect(screen.getByText('commune.autonomousActive')).toBeTruthy();
        });
    });

    it('handles stopping the autonomous engine', async () => {
        let isRunning = true;
        mockFetch.mockImplementation((url, options) => {
            if (url.includes('/api/commune/list')) return Promise.resolve({ ok: true, json: async () => [] });
            if (url.includes('/api/commune/autonomous/status')) {
                return Promise.resolve({ ok: true, json: async () => isRunning ? mockStatusRunning : mockStatusStopped });
            }
            if (url.includes('/api/commune/autonomous/stop') && options?.method === 'POST') {
                isRunning = false;
                return Promise.resolve({ ok: true });
            }
            return Promise.resolve({ ok: true, json: async () => [] });
        });

        render(<CommuneDialogueView />);

        await waitFor(() => {
            expect(screen.getByText(/commune.stopLoop|Stop Autonomous Loop/)).toBeTruthy();
        });

        const stopBtn = screen.getByText(/commune.stopLoop|Stop Autonomous Loop/);
        
        await act(async () => {
            fireEvent.click(stopBtn);
        });

        expect(mockFetch).toHaveBeenCalledWith(
            'http://localhost:3000/api/commune/autonomous/stop',
            expect.objectContaining({ method: 'POST' })
        );

        await waitFor(() => {
            expect(screen.getByText('commune.startDialogue')).toBeTruthy();
            expect(screen.getByText('commune.manualMode')).toBeTruthy();
        });
    });

    it('polls for new messages and status', async () => {
        mockFetch.mockResolvedValue({ ok: true, json: async () => [] });
        
        render(<CommuneDialogueView />);

        await waitFor(() => {
            expect(mockFetch).toHaveBeenCalledWith('http://localhost:3000/api/commune/list');
            expect(mockFetch).toHaveBeenCalledWith('http://localhost:3000/api/commune/autonomous/status');
        });

        const initialCallCount = mockFetch.mock.calls.length;

        // Advance timers by 5 seconds
        act(() => {
            jest.advanceTimersByTime(5000);
        });

        await waitFor(() => {
            expect(mockFetch.mock.calls.length).toBeGreaterThan(initialCallCount);
        });
    });

    it('handles API error (network failure) gracefully', async () => {
        const consoleSpy = jest.spyOn(console, 'error').mockImplementation(() => {});

        mockFetch.mockRejectedValue(new Error('Network error'));

        render(<CommuneDialogueView />);

        await waitFor(() => {
            expect(consoleSpy).toHaveBeenCalledWith(
                'Failed to fetch messages',
                expect.any(Error)
            );
        });

        // Component should still render without crashing
        expect(screen.getByText('commune.dialogueStream')).toBeTruthy();

        consoleSpy.mockRestore();
    });

    it('handles non-ok API response gracefully', async () => {
        mockFetch.mockResolvedValue({ ok: false, status: 500 });

        render(<CommuneDialogueView />);

        // Should not crash, should show empty/waiting state
        await waitFor(() => {
            expect(screen.getByText('commune.waitingMessages')).toBeTruthy();
        });
    });

    it('cleans up polling interval on unmount', () => {
        mockFetch.mockResolvedValue({ ok: true, json: async () => [] });

        const { unmount } = render(<CommuneDialogueView />);
        const clearIntervalSpy = jest.spyOn(global, 'clearInterval');

        unmount();

        expect(clearIntervalSpy).toHaveBeenCalled();
        clearIntervalSpy.mockRestore();
    });

    it('applies correct kebab-case CSS properties in styling', () => {
        const { container } = render(<CommuneDialogueView />);
        const styleTags = container.querySelectorAll('style');
        expect(styleTags.length).toBeGreaterThan(0);
        
        let foundStyleTag = false;
        styleTags.forEach(styleTag => {
            const content = styleTag.textContent || '';
            if (content.includes('.info-box-glass')) {
                foundStyleTag = true;
                // camelCase properties should NOT be present
                expect(content).not.toContain('borderRadius:');
                expect(content).not.toContain('fontSize:');
                expect(content).not.toContain('lineHeight:');
                
                // kebab-case properties should be present
                expect(content).toContain('border-radius:');
                expect(content).toContain('font-size:');
                expect(content).toContain('line-height:');
            }
        });
        expect(foundStyleTag).toBe(true);
    });
});

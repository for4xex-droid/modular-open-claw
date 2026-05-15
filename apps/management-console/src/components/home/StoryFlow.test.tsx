import React from 'react';
import { render, screen, fireEvent, waitFor, act } from '@testing-library/react';
import StoryFlow from './StoryFlow';
import { useAgentChat } from '../../hooks/useAgentChat';
import { useCortexSuggestions } from '../../hooks/useCortexSuggestions';

jest.mock('../../hooks/useAgentChat');
jest.mock('../../hooks/useCortexSuggestions');

jest.mock('../../config', () => ({
    API_BASE: 'http://localhost:3000'
}));

jest.mock('../../i18n', () => ({
    useTranslation: () => ({ t: (key: string) => key })
}));

jest.mock('../common/TokenSavingsIndicator', () => ({
    TokenSavingsIndicator: () => <div data-testid="token-indicator" />
}));

jest.mock('./FlowCard', () => {
    return function MockFlowCard(props: any) {
        return (
            <div data-testid="flow-card">
                <h3>{props.title}</h3>
                <p>{props.content}</p>
                <button onClick={() => props.onFeedback && props.onFeedback('positive')}>Feedback</button>
            </div>
        );
    };
});

// Mock framer-motion
jest.mock('framer-motion', () => ({
    motion: {
        div: ({ children, ...props }: any) => <div {...props}>{children}</div>
    },
    AnimatePresence: ({ children }: any) => <>{children}</>
}));

// Mock lucide-react icons
jest.mock('lucide-react', () => ({
    Send: () => <div data-testid="icon-send"></div>,
    Sparkles: () => <div data-testid="icon-sparkles"></div>,
    Volume2: () => <div data-testid="icon-volume2"></div>,
    VolumeX: () => <div data-testid="icon-volumex"></div>,
    Cpu: () => <div data-testid="icon-cpu"></div>,
    Wifi: () => <div data-testid="icon-wifi"></div>,
    WifiOff: () => <div data-testid="icon-wifioff"></div>,
    Brain: () => <div data-testid="icon-brain"></div>
}));

describe('StoryFlow Component', () => {
    let mockChat: any;
    let mockSuggestions: any;

    beforeEach(() => {
        jest.clearAllMocks();

        mockChat = {
            history: [],
            input: '',
            setInput: jest.fn(),
            sendMessage: jest.fn(),
            isTyping: false,
            status: 'IDLE',
            autoTts: false,
            setAutoTts: jest.fn(),
            relevantKarma: null,
            relevantKarmaData: null,
            activeKnowledge: null,
            streamingText: '',
            handleFeedback: jest.fn()
        };

        (useAgentChat as jest.Mock).mockReturnValue(mockChat);

        mockSuggestions = {
            suggestions: [],
            fetchSuggestions: jest.fn()
        };

        (useCortexSuggestions as jest.Mock).mockReturnValue(mockSuggestions);

        Object.defineProperty(HTMLElement.prototype, 'scrollTop', {
            configurable: true,
            get() { return 0; },
            set(v) {}
        });
        Object.defineProperty(HTMLElement.prototype, 'scrollHeight', {
            configurable: true,
            get() { return 1000; }
        });
    });

    it('renders empty state correctly', () => {
        render(<StoryFlow />);

        expect(screen.getByText('storyFlow.activeFeed')).toBeTruthy();
        expect(screen.getByText('agent.ready')).toBeTruthy(); // empty state message
        expect(screen.getByText('storyFlow.emptyHint')).toBeTruthy();
    });

    it('builds unified timeline from chat history', () => {
        mockChat.history = [
            { role: 'user', content: 'Hello AI' },
            { role: 'assistant', content: 'Hello Human' }
        ];

        render(<StoryFlow />);

        expect(screen.getByText('Hello AI')).toBeTruthy();
        expect(screen.getByText('Hello Human')).toBeTruthy();
        expect(screen.getAllByTestId('flow-card').length).toBe(2);
    });

    it('renders system events correctly', () => {
        const sysEvents = [
            { type: 'level_up', data: { level: 5 } },
            { type: 'karma_update', data: { lesson: 'Learned something new' } },
            { type: 'job_started', data: { job_type: 'Data Analysis' } },
            { type: 'sot_progress', data: { type: 'SessionStart', message: 'Started deliberation' } }
        ];

        render(<StoryFlow sysEvents={sysEvents as any} />);

        expect(screen.getAllByText('storyFlow.levelUp').length).toBeGreaterThan(0);
        expect(screen.getByText('Learned something new')).toBeTruthy();
        expect(screen.getByText('Data Analysis initiated')).toBeTruthy();
        expect(screen.getByText('Started deliberation')).toBeTruthy();
    });

    it('shows context and streaming elements', () => {
        mockChat.relevantKarma = 'Past context...';
        mockChat.activeKnowledge = 'Project structure...';
        mockChat.streamingText = 'I am thinking...';

        render(<StoryFlow />);

        expect(screen.getByText('Past context...')).toBeTruthy();
        expect(screen.getByText('Project structure...')).toBeTruthy();
        expect(screen.getByText('I am thinking...')).toBeTruthy();
    });

    it('handles text input and sending', () => {
        const { rerender } = render(<StoryFlow />);

        const textarea = screen.getByPlaceholderText('agent.ready');
        
        fireEvent.change(textarea, { target: { value: 'New message' } });
        expect(mockChat.setInput).toHaveBeenCalledWith('New message');

        // Simulate typing and entering
        mockChat.input = 'New message'; // Update mock value for the button state
        rerender(<StoryFlow />); // re-render to apply mockChat update

        // Find Send button via its child icon's data-testid (stable selector)
        const sendIcon = screen.getByTestId('icon-send');
        const sendBtn = sendIcon.closest('button')!;
        fireEvent.click(sendBtn);

        expect(mockChat.sendMessage).toHaveBeenCalled();
    });

    it('handles enter key to send', () => {
        mockChat.input = 'Message via enter';
        render(<StoryFlow />);

        const textarea = screen.getByPlaceholderText('agent.ready');
        fireEvent.keyDown(textarea, { key: 'Enter', shiftKey: false });

        expect(mockChat.sendMessage).toHaveBeenCalled();
    });

    it('shows slash commands when input starts with /', () => {
        mockChat.input = '/cle';
        render(<StoryFlow />);

        // Slash command suggestions should appear
        expect(screen.getByText('/clear')).toBeTruthy();
        expect(screen.getByText(/Clear history/i)).toBeTruthy();
    });

    it('handles toggling TTS', () => {
        render(<StoryFlow />);

        const ttsBtn = screen.getByText('TTS');
        fireEvent.click(ttsBtn);

        expect(mockChat.setAutoTts).toHaveBeenCalledWith(true);
    });

    it('handles cortex suggestions on focus', async () => {
        mockSuggestions.suggestions = ['How do I fix this?', 'Optimize this code'];
        
        render(<StoryFlow />);

        const textarea = screen.getByPlaceholderText('agent.ready');
        fireEvent.focus(textarea);

        expect(mockSuggestions.fetchSuggestions).toHaveBeenCalled();

        await waitFor(() => {
            expect(screen.getByText('How do I fix this?')).toBeTruthy();
        });

        fireEvent.click(screen.getByText('How do I fix this?'));
        expect(mockChat.setInput).toHaveBeenCalledWith('How do I fix this?');
    });

    it('handles feedback correctly', () => {
        mockChat.history = [
            { role: 'user', content: 'Hello' },
            { role: 'assistant', content: 'Response', isError: false }
        ];
        mockChat.relevantKarmaData = { entries: [{ id: '1' }] };

        render(<StoryFlow />);

        const feedbackBtn = screen.getAllByText('Feedback')[1]; // assistant's card
        fireEvent.click(feedbackBtn);

        expect(mockChat.handleFeedback).toHaveBeenCalledWith(0, 'positive');
    });

    it('handles keyboard navigation for slash commands', () => {
        jest.useFakeTimers();
        mockChat.input = '/c';
        render(<StoryFlow />);

        const textarea = screen.getByPlaceholderText('agent.ready');

        // Arrow down/up should cycle through filtered commands
        fireEvent.keyDown(textarea, { key: 'ArrowDown' });
        fireEvent.keyDown(textarea, { key: 'ArrowUp' });

        // Enter to select the current command
        fireEvent.keyDown(textarea, { key: 'Enter' });
        // setInput should have been called with the selected slash command
        expect(mockChat.setInput).toHaveBeenCalledWith('/clear');

        // sendMessage is dispatched via setTimeout(0) — flush it
        act(() => {
            jest.runAllTimers();
        });
        expect(mockChat.sendMessage).toHaveBeenCalledWith('/clear');
        jest.useRealTimers();
    });

    it('does not send on Enter while composing (IME)', () => {
        mockChat.input = 'テスト';
        render(<StoryFlow />);

        const textarea = screen.getByPlaceholderText('agent.ready');
        // isComposing = true means IME is active; sendMessage should NOT fire
        const event = new KeyboardEvent('keydown', { key: 'Enter', bubbles: true });
        Object.defineProperty(event, 'isComposing', { value: true });
        textarea.dispatchEvent(event);

        expect(mockChat.sendMessage).not.toHaveBeenCalled();
    });

    it('does not send on Shift+Enter (newline)', () => {
        mockChat.input = 'Some text';
        render(<StoryFlow />);

        const textarea = screen.getByPlaceholderText('agent.ready');
        fireEvent.keyDown(textarea, { key: 'Enter', shiftKey: true });

        expect(mockChat.sendMessage).not.toHaveBeenCalled();
    });

    it('handles textarea blur with timeout', async () => {
        jest.useFakeTimers();
        mockSuggestions.suggestions = ['Suggestion A'];

        render(<StoryFlow />);

        const textarea = screen.getByPlaceholderText('agent.ready');

        // Focus: suggestions become visible
        fireEvent.focus(textarea);
        expect(screen.getByText('Suggestion A')).toBeTruthy();

        // Blur: after the 200ms debounce, suggestions should disappear
        fireEvent.blur(textarea);
        act(() => {
            jest.advanceTimersByTime(250);
        });

        // After timeout, the suggestion should no longer be rendered
        expect(screen.queryByText('Suggestion A')).toBeNull();

        jest.useRealTimers();
    });
});

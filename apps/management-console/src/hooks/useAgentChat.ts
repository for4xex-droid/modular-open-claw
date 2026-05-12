/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

/**
 * useAgentChat — AgentConsole.tsx から抽出されたチャットロジックフック。
 * SSE ストリーミング、TTS 自動再生、Karma フィードバックを一元管理する。
 * AgentConsole（既存）と StoryFlow（新規）の両方で共用可能。
 */
import { useState, useCallback, useRef, useEffect } from 'react';
import { SLASH_COMMANDS } from '../constants/slashCommands';
import { API_BASE } from '../config';
import { ChatMessage } from '../types';
import { authenticatedFetch } from '../lib/auth';
import { useSystemVitality } from './useSystemVitality';

export interface KarmaData {
    is_ood: boolean;
    entries: { id: string; lesson: string }[];
}

export interface AgentChatState {
    history: ChatMessage[];
    input: string;
    isTyping: boolean;
    streamingText: string;
    status: string;
    autoTts: boolean;
    relevantKarma: string | null;
    relevantKarmaData: KarmaData | null;
    activeKnowledge: string | null;
    channelId: string;
}

export interface UseAgentChatReturn extends AgentChatState {
    setInput: (value: string) => void;
    sendMessage: (overridePrompt?: string) => Promise<void>;
    setAutoTts: (value: boolean) => void;
    handleFeedback: (index: number, type: 'positive' | 'negative') => Promise<void>;
    clearHistory: () => void;
}

export const useAgentChat = (): UseAgentChatReturn => {
    const [input, setInput] = useState("");
    const [history, setHistory] = useState<ChatMessage[]>([]);
    const [isTyping, setIsTyping] = useState(false);
    const [streamingText, setStreamingText] = useState("");
    const [autoTts, setAutoTts] = useState(true);
    const [status, setStatus] = useState<string>("IDLE");
    const [relevantKarma, setRelevantKarma] = useState<string | null>(null);
    const [relevantKarmaData, setRelevantKarmaData] = useState<KarmaData | null>(null);
    const [activeKnowledge, setActiveKnowledge] = useState<string | null>(null);
    const autoTtsRef = useRef(autoTts);
    autoTtsRef.current = autoTts;
    
    // P3: Intent-First Suggestion Integration
    const { lastEvent } = useSystemVitality();

    useEffect(() => {
        if (lastEvent?.type === 'proactive_talk') {
            const message = (lastEvent.data as any)?.message;
            if (message) {
                setHistory(prev => {
                    // Prevent duplicate consecutive messages
                    const lastMsg = prev[prev.length - 1];
                    if (lastMsg && lastMsg.role === 'assistant' && lastMsg.content === message) {
                        return prev;
                    }
                    return [...prev, { role: "assistant", content: message }];
                });
                if (autoTtsRef.current) {
                    playTts(message);
                }
            }
        }
    }, [lastEvent]); // playTts is not in deps to avoid issues, we use ref inside playTts if needed, but it's useCallback. Let's omit playTts from deps.

    const [channelId] = useState(() => {
        const stored = sessionStorage.getItem('aiome_console_channel_id');
        if (stored) return stored;
        const newId = crypto.randomUUID();
        sessionStorage.setItem('aiome_console_channel_id', newId);
        return newId;
    });

    useEffect(() => {
        const loadHistory = async () => {
            try {
                const resp = await authenticatedFetch(`${API_BASE}/api/stream/history?channel_id=${channelId}`);
                if (resp.ok) {
                    const data = await resp.json();
                    if (data.messages && Array.isArray(data.messages)) {
                        setHistory(data.messages.map((m: any) => ({
                            role: m.role,
                            content: m.content,
                            reasoning: m.metadata?.reasoning
                        })));
                    }
                }
            } catch (e) {
                console.error("Failed to load history", e);
            }
        };
        loadHistory();
    }, [channelId]);

    // We must declare sendMessage before the ref if we want to initialize it, but since sendMessage uses many states, we'll initialize the ref as null or use a separate ref update.
    const sendMessageRef = useRef<((overridePrompt?: string) => Promise<void>) | null>(null);

    // Listen for JS bridge feedback from HTML artifacts
    useEffect(() => {
        const handleInjectPrompt = (e: Event) => {
            const customEvent = e as CustomEvent;
            if (customEvent.detail && customEvent.detail.prompt) {
                const text = customEvent.detail.prompt;
                setInput(text);
                if (customEvent.detail.autoSend && sendMessageRef.current) {
                    sendMessageRef.current(text);
                }
            }
        };
        window.addEventListener('aiome_inject_prompt', handleInjectPrompt);
        return () => window.removeEventListener('aiome_inject_prompt', handleInjectPrompt);
    }, []);

    const playTts = useCallback(async (text: string) => {
        if (!text) return;
        try {
            const response = await authenticatedFetch(`${API_BASE}/api/v1/voice/synthesize`, {
                method: 'POST',
                body: JSON.stringify({ text })
            });
            if (!response.ok) throw new Error("TTS failed");
            const blob = await response.blob();
            const url = URL.createObjectURL(blob);
            const audio = new Audio(url);
            await audio.play();
        } catch (e) {
            console.error("TTS Playback failed:", e);
        }
    }, []);

    const sendMessage = useCallback(async (overridePrompt?: string) => {
        const currentPrompt = overridePrompt || input;
        if (!currentPrompt.trim() || isTyping) return;

        // NOTE: '/clear' は SLASH_COMMANDS 内で envelopeType: null として定義されている特殊コマンド。
        // UI サーフェスの生成ではなくステートリセットを行うため、ここで明示的にハンドリングする。
        // コマンド名を変更する場合は slashCommands.ts と同期すること。
        if (currentPrompt === '/clear') {
            setHistory([]);
            setStreamingText("");
            setRelevantKarma(null);
            setRelevantKarmaData(null);
            setActiveKnowledge(null);
            setStatus("IDLE");
            setInput("");
            return;
        }

        const matchedCmd = SLASH_COMMANDS.find(c => c.cmd === currentPrompt && c.envelopeType);
        const matchedEnvelopeType = matchedCmd?.envelopeType;
        if (matchedEnvelopeType) {
            setHistory(prev => [
                ...prev,
                { role: "user", content: currentPrompt },
                {
                    role: "assistant",
                    content: "",
                    a2uiEnvelope: {
                        type: 'createSurface',
                        surface: {
                            id: crypto.randomUUID(),
                            version: 'local',
                            source: 'slash-command',
                            components: [{ type: matchedEnvelopeType, props: {}, children: [] }]
                        }
                    }
                }
            ]);
            setInput("");
            return;
        }

        const userMsg: ChatMessage = { role: "user", content: currentPrompt };
        setHistory(prev => [...prev, userMsg]);
        setInput("");
        setIsTyping(true);
        setStreamingText("");
        setStatus("THINKING");
        setRelevantKarma(null);
        setRelevantKarmaData(null);
        setActiveKnowledge(null);

        try {
            const response = await authenticatedFetch(`${API_BASE}/api/stream/chat`, {
                method: 'POST',
                body: JSON.stringify({
                    prompt: currentPrompt,
                    history: history,
                    channel_id: channelId
                })
            });

            if (!response.body) throw new Error("No response body");

            const reader = response.body.getReader();
            const decoder = new TextDecoder();
            let accumulatedText = "";
            let currentEvent = "";
            let buffer = "";

            while (true) {
                const { done, value } = await reader.read();
                if (done) break;

                buffer += decoder.decode(value, { stream: true });
                const lines = buffer.split('\n');
                buffer = lines.pop() || "";

                for (const line of lines) {
                    const trimmedLine = line.trim();
                    if (!trimmedLine) continue;

                    if (trimmedLine.startsWith('event: ')) {
                        currentEvent = trimmedLine.replace('event: ', '');
                    } else if (trimmedLine.startsWith('data: ')) {
                        const data = trimmedLine.replace('data: ', '');

                        if (currentEvent === 'text') {
                            accumulatedText += data;
                            setStreamingText(accumulatedText);
                        } else if (currentEvent === 'tool_exec' || currentEvent === 'tool_detect') {
                            setStatus(`EXECUTING: ${data}`);
                        } else if (currentEvent === 'error') {
                            setHistory(prev => [...prev, { role: "assistant", content: `🚨 Error: ${data}`, isError: true }]);
                        } else if (currentEvent === 'task_awaiting_input') {
                            setStatus("AWAITING INPUT");
                        } else if (currentEvent === 'a2ui') {
                            try {
                                const envelope = JSON.parse(data);
                                // Runtime shape-check: TypeScript types don't exist at runtime.
                                const validTypes = ['createSurface', 'updateComponents', 'deleteSurface'];
                                const isValidShape = envelope
                                    && typeof envelope.type === 'string'
                                    && validTypes.includes(envelope.type)
                                    && (envelope.type !== 'createSurface' || (envelope.surface && typeof envelope.surface.id === 'string'));

                                if (isValidShape) {
                                    if (accumulatedText.trim().length > 0) {
                                        setHistory(prev => [...prev, { role: "assistant", content: accumulatedText }]);
                                        accumulatedText = "";
                                        setStreamingText("");
                                    }
                                    setHistory(prev => {
                                        const newHistory: ChatMessage[] = [...prev, { role: "assistant" as const, content: "", a2uiEnvelope: envelope }];
                                        const MAX_SURFACES = 20;
                                        let a2uiCount = 0;
                                        for (let i = newHistory.length - 1; i >= 0; i--) {
                                            if (newHistory[i].a2uiEnvelope) {
                                                a2uiCount++;
                                                if (a2uiCount > MAX_SURFACES) {
                                                    newHistory[i] = { ...newHistory[i], content: "[A2UI Surface Expired]", a2uiEnvelope: undefined };
                                                }
                                            }
                                        }
                                        return newHistory;
                                    });
                                } else {
                                    console.warn('[A2UI] Rejected malformed envelope:', envelope?.type);
                                }
                            } catch (e) {
                                console.error("Failed to parse A2UI JSON:", e);
                            }
                        } else if (currentEvent === 'done') {
                            setStatus("IDLE");
                        } else if (currentEvent === 'karma') {
                            setRelevantKarma(data);
                        } else if (currentEvent === 'karma_data') {
                            try {
                                setRelevantKarmaData(JSON.parse(data));
                            } catch (e) {
                                console.error("Failed to parse karma_data", e);
                            }
                        } else if (currentEvent === 'knowledge') {
                            setActiveKnowledge(data);
                        } else if (currentEvent === 'token_saved') {
                            try {
                                const parsed = JSON.parse(data);
                                if (typeof parsed.saved_chars === 'number' && parsed.saved_chars > 0) {
                                    window.dispatchEvent(new CustomEvent('aiome_vitality_event', {
                                        detail: {
                                            type: 'token_saved',
                                            data: { saved_chars: parsed.saved_chars, ts: Date.now() }
                                        }
                                    }));
                                }
                            } catch { /* ignore malformed */ }
                        }
                    }
                }
            }

            if (accumulatedText) {
                setHistory(prev => [...prev, { role: "assistant", content: accumulatedText }]);
                setStreamingText("");
                if (autoTtsRef.current) {
                    playTts(accumulatedText);
                }
            }
        } catch (_e) {
            setHistory(prev => [...prev, { role: "assistant", content: "⚠️ Connection error to Aiome layer.", isError: true }]);
        } finally {
            setIsTyping(false);
            setStatus("IDLE");
        }
    }, [input, isTyping, history, channelId, playTts]);

    useEffect(() => {
        sendMessageRef.current = sendMessage;
    }, [sendMessage]);

    const handleFeedback = useCallback(async (_index: number, type: 'positive' | 'negative') => {
        if (!relevantKarmaData || !relevantKarmaData.entries || relevantKarmaData.entries.length === 0) return;

        const primaryKarmaId = relevantKarmaData.entries[0].id;

        try {
            await authenticatedFetch(`${API_BASE}/api/agent/feedback`, {
                method: 'POST',
                body: JSON.stringify({
                    karma_id: primaryKarmaId,
                    is_positive: type === 'positive'
                })
            });
            setStatus(`FEEDBACK RECORDED: ${type.toUpperCase()}`);
            setTimeout(() => setStatus("IDLE"), 2000);
        } catch (e) {
            console.error("Failed to send feedback", e);
        }
    }, [relevantKarmaData]);

    const clearHistory = useCallback(() => {
        setHistory([]);
        setStreamingText("");
        setRelevantKarma(null);
        setRelevantKarmaData(null);
        setActiveKnowledge(null);
        setStatus("IDLE");
    }, []);

    return {
        history,
        input,
        isTyping,
        streamingText,
        status,
        autoTts,
        relevantKarma,
        relevantKarmaData,
        activeKnowledge,
        channelId,
        setInput,
        sendMessage,
        setAutoTts,
        handleFeedback,
        clearHistory,
    };
};

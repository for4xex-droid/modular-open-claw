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
import { useState, useCallback, useRef } from 'react';
import { API_BASE } from '../config';
import { ChatMessage } from '../types';
import { authenticatedFetch } from '../lib/auth';

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
    sendMessage: () => Promise<void>;
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
    
    const [channelId] = useState(() => {
        const stored = sessionStorage.getItem('aiome_console_channel_id');
        if (stored) return stored;
        const newId = crypto.randomUUID();
        sessionStorage.setItem('aiome_console_channel_id', newId);
        return newId;
    });

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

    const sendMessage = useCallback(async () => {
        if (!input.trim() || isTyping) return;

        const currentPrompt = input;
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

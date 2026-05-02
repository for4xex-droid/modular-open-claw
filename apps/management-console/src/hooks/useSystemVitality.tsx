/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import React, { createContext, useContext, useEffect, useState, useCallback, useRef, ReactNode } from 'react';
import { API_BASE } from '../config';
import { AgentStats, Karma, SoTEvent } from '../types';
import { getAuthHeaders } from '../lib/auth';
import { fetchEventSource } from '@microsoft/fetch-event-source';

export interface SystemVitality {
    status: 'idle' | 'thinking' | 'speaking' | 'learning' | 'meditating' | 'awakened';
    data: AgentStats | Karma | SoTEvent | unknown;
}

export type VitalityEvent = {
    type: 'level_up' | 'karma_update' | 'inspiration' | 'job_started' | 'job_completed' | 'tts_started' | 'tts_completed' | 'skill_loaded' | 'skill_ready' | 'immune_alert' | 'aegis_sentinel' | 'skill_execution' | 'agent_stats' | 'proactive_talk' | 'plugin_event' | 'gig_published' | 'sot_progress' | 'token_saved' | 'task_awaiting_input' | 'quality_gate';
    data: AgentStats | Karma | SoTEvent | unknown;
};

type ConnectionStatus = 'connected' | 'connecting' | 'disconnected' | 'paused';

interface SystemVitalityContextValue {
    events: VitalityEvent[];
    lastEvent: VitalityEvent | null;
    connectionStatus: ConnectionStatus;
    toggleConnection: () => void;
    lastPingMs: number | null;
}

const SystemVitalityContext = createContext<SystemVitalityContextValue | null>(null);

export const SystemVitalityProvider: React.FC<{ children: ReactNode }> = ({ children }) => {
    const [events, setEvents] = useState<VitalityEvent[]>([]);
    const [lastEvent, setLastEvent] = useState<VitalityEvent | null>(null);
    const [connectionStatus, setConnectionStatus] = useState<ConnectionStatus>('connecting');
    const [lastPingMs, setLastPingMs] = useState<number | null>(null);
    const [isPaused, setIsPaused] = useState(false);
    const [retryTrigger, setRetryTrigger] = useState(0);

    const abortControllerRef = useRef<AbortController | null>(null);

    const addEvent = useCallback((type: VitalityEvent['type'], data: VitalityEvent['data']) => {
        const newEvent = { type, data };
        setEvents(prev => [newEvent, ...prev].slice(0, 50));
        setLastEvent(newEvent);
    }, []);

    const toggleConnection = useCallback(() => {
        if (connectionStatus === 'disconnected') {
            setRetryTrigger(prev => prev + 1);
            setIsPaused(false);
        } else {
            setIsPaused(prev => !prev);
        }
    }, [connectionStatus]);

    useEffect(() => {
        if (isPaused) {
            setConnectionStatus('paused');
            if (abortControllerRef.current) {
                abortControllerRef.current.abort();
                abortControllerRef.current = null;
            }
            return;
        }

        const MAX_RETRIES = 5;
        let retryCount = 0;

        const connect = async () => {
            if (abortControllerRef.current) {
                abortControllerRef.current.abort();
            }

            abortControllerRef.current = new AbortController();
            setConnectionStatus('connecting');

            try {
                await fetchEventSource(`${API_BASE}/api/stream/vitality`, {
                    method: 'GET',
                    headers: {
                        ...getAuthHeaders(),
                        'Accept': 'text/event-stream'
                    },
                    signal: abortControllerRef.current.signal,
                    onopen: async (response) => {
                        if (response.ok) {
                            console.log("✨ [SSE] Connection established");
                            setConnectionStatus('connected');
                            retryCount = 0;
                            return; 
                        }
                        throw new Error(`SSE Status ${response.status}`);
                    },
                    onmessage: (msg) => {
                        if (!msg.event || !msg.data) return;

                        const validEvents = [
                            'level_up', 'karma_update', 'inspiration',
                            'job_started', 'job_completed',
                            'tts_started', 'tts_completed',
                            'skill_loaded', 'skill_ready',
                            'immune_alert', 'aegis_sentinel', 'skill_execution', 'agent_stats', 'proactive_talk', 'plugin_event', 'gig_published', 'sot_progress', 'token_saved', 'task_awaiting_input', 'quality_gate'
                        ];

                        if (validEvents.includes(msg.event)) {
                            try {
                                const data = msg.data ? JSON.parse(msg.data) : null;
                                if (msg.event === 'quality_gate' && data) {
                                    if (typeof data.score !== 'number' || typeof data.passed !== 'boolean') {
                                        console.warn('Invalid quality_gate payload:', data);
                                        return;
                                    }
                                }
                                addEvent(msg.event as VitalityEvent['type'], data);
                            } catch (err) {
                                console.error(`Error parsing SSE event ${msg.event}:`, err);
                            }
                        } else if (msg.event === 'ping') {
                            try {
                                const serverTime = new Date(msg.data).getTime();
                                const clientTime = Date.now();
                                const rtt = Math.abs(clientTime - serverTime);
                                setLastPingMs(rtt);
                            } catch { /* ignore parse errors */ }
                        }
                    },
                    onclose: () => {
                        console.warn("⚠️ [SSE] Connection closed from server, retrying...");
                        setConnectionStatus('disconnected');
                    },
                    onerror: (err) => {
                        setConnectionStatus('disconnected');
                        
                        if (abortControllerRef.current?.signal.aborted) {
                           throw err;
                        }

                        if (retryCount >= MAX_RETRIES) {
                            console.error("❌ [SSE] Max retries reached. Stopping.");
                            throw err;
                        }

                        const delay = Math.min(1000 * Math.pow(2, retryCount), 10000);
                        retryCount++;
                        console.log(`🔄 [SSE] Error: ${err.message}. Retrying in ${delay}ms (Attempt ${retryCount}/${MAX_RETRIES})`);
                        
                        return delay;
                    }
                });
            } catch (err) {
                setConnectionStatus('disconnected');
            }
        };

        connect();

        const handleCustomEvent = (e: Event) => {
            const customEvent = e as CustomEvent<VitalityEvent>;
            if (customEvent.detail && customEvent.detail.type) {
                const type = customEvent.detail.type;
                const data = customEvent.detail.data || { ...customEvent.detail };
                addEvent(type, data);
            }
        };
        window.addEventListener('aiome_vitality_event', handleCustomEvent);

        return () => {
            window.removeEventListener('aiome_vitality_event', handleCustomEvent);
            if (abortControllerRef.current) {
                abortControllerRef.current.abort();
                abortControllerRef.current = null;
            }
        };
    }, [addEvent, isPaused, retryTrigger]);

    return (
        <SystemVitalityContext.Provider value={{ events, lastEvent, connectionStatus, toggleConnection, lastPingMs }}>
            {children}
        </SystemVitalityContext.Provider>
    );
};

export const useSystemVitality = () => {
    const context = useContext(SystemVitalityContext);
    if (!context) {
        throw new Error("useSystemVitality must be used within a SystemVitalityProvider");
    }
    return context;
};

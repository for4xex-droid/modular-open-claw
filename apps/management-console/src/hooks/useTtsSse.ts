/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import { useCallback, useRef } from 'react';
import { fetchEventSource } from '@microsoft/fetch-event-source';
import { API_BASE } from '../config';
import { getAuthHeaders } from '../lib/auth';
import type { VisemeFrame } from '../types/avatar';

/** Sanitize server-provided error messages to prevent log injection. */
const sanitizeErrorMessage = (raw: string): string => {
    return raw.replace(/[\r\n]/g, ' ').slice(0, 256);
};

export interface UseTtsSseReturn {
    /** Start SSE streaming TTS. Throws on server error to enable fallback. */
    speak: (text: string) => Promise<void>;
    /** Abort any in-flight stream and stop audio playback. */
    cancel: () => void;
}

export const useTtsSse = (): UseTtsSseReturn => {
    const abortControllerRef = useRef<AbortController | null>(null);
    const audioRef = useRef<HTMLAudioElement | null>(null);

    /** Abort any in-flight SSE stream and stop audio playback. */
    const cleanup = useCallback(() => {
        if (abortControllerRef.current) {
            abortControllerRef.current.abort();
            abortControllerRef.current = null;
        }
        if (audioRef.current) {
            audioRef.current.pause();
            audioRef.current.src = "";
            audioRef.current = null;
        }
    }, []);

    const speak = useCallback(async (text: string) => {
        if (!text.trim()) return; // Guard: skip empty/whitespace-only text

        cleanup();

        const ctrl = new AbortController();
        abortControllerRef.current = ctrl;

        // Capture server-side errors to re-throw after fetchEventSource resolves.
        // onmessage callbacks cannot reliably throw within fetchEventSource's internal loop.
        let serverError: Error | null = null;
        const audioChunks: Uint8Array[] = [];
        const visemes: VisemeFrame[] = [];

        try {
            await fetchEventSource(`${API_BASE}/api/v1/voice/synthesize?stream=true`, {
                method: 'POST',
                headers: {
                    ...getAuthHeaders(),
                    'Content-Type': 'application/json',
                },
                body: JSON.stringify({ text }),
                signal: ctrl.signal,
                async onopen(response) {
                    if (response.status === 402) {
                        if (typeof window !== 'undefined') {
                            window.dispatchEvent(new CustomEvent('stripe-402-payment-required'));
                        }
                        throw new Error('Subscription required');
                    }
                    if (!response.ok) {
                        throw new Error(`TTS request failed: ${response.status}`);
                    }
                },
                onmessage(ev) {
                    if (ev.event === 'audio') {
                        const binaryStr = window.atob(ev.data);
                        const len = binaryStr.length;
                        const bytes = new Uint8Array(len);
                        for (let i = 0; i < len; i++) {
                            bytes[i] = binaryStr.charCodeAt(i);
                        }
                        audioChunks.push(bytes);
                    } else if (ev.event === 'viseme') {
                        try {
                            const vData = JSON.parse(ev.data);
                            visemes.push(vData);
                        } catch (e) {
                            console.error("Failed to parse viseme", e);
                        }
                    } else if (ev.event === 'error') {
                        let errStr = ev.data;
                        try {
                            errStr = JSON.parse(ev.data).error || errStr;
                        } catch { /* ignore parse error */ }
                        serverError = new Error(sanitizeErrorMessage(errStr));
                        ctrl.abort(); // Gracefully terminate the stream
                    }
                },
                onclose() {
                    // Skip playback if a server error was received (chunks may be incomplete/corrupt)
                    if (audioChunks.length === 0 || serverError) return;

                    const blob = new Blob(audioChunks, { type: 'audio/wav' });
                    const url = URL.createObjectURL(blob);
                    const audio = new Audio(url);
                    audioRef.current = audio;

                    audio.addEventListener('ended', () => {
                        URL.revokeObjectURL(url);
                        window.dispatchEvent(new CustomEvent('aiome_vitality_event', {
                            detail: { type: 'tts_completed', data: {} }
                        }));
                        audioRef.current = null;
                    });

                    audio.addEventListener('error', () => {
                        URL.revokeObjectURL(url);
                        audioRef.current = null;
                    });

                    audio.play().then(() => {
                        window.dispatchEvent(new CustomEvent('aiome_vitality_event', {
                            detail: { type: 'tts_started', data: { visemes, audioElement: audio } }
                        }));
                    }).catch(e => {
                        console.error('TTS Playback failed:', e);
                        URL.revokeObjectURL(url);
                        audioRef.current = null;
                    });
                },
                onerror(err) {
                    console.error("useTtsSse fetchEventSource error:", err);
                    throw err; // Stop retrying and fallback
                }
            });

            // If a server-side error event was received, throw it after the stream ends.
            if (serverError) {
                throw serverError;
            }
        } catch (err: unknown) {
            if (err instanceof Error && err.name !== 'AbortError') {
                throw err; // bubble up for blob fallback
            }
            // If aborted due to serverError, re-throw the captured error.
            if (serverError) {
                throw serverError;
            }
        }
    }, [cleanup]);

    return { speak, cancel: cleanup } satisfies UseTtsSseReturn;
};

/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import { useRef, useEffect, useCallback } from 'react';
import { VisemeFrame } from '../types/avatar';

export interface VisemeSyncState {
    viseme: string | null;
    weight: number;
    isActive: boolean;
}

/**
 * アバター非依存の Viseme 時間同期フック。
 * - tts_started / tts_completed イベントをリッスン
 * - audio.currentTime ベースの高精度同期（提供されている場合）
 * - フォールバックとして delta 累積同期を提供
 */
export const useVisemeSync = () => {
    const visemeQueueRef = useRef<VisemeFrame[]>([]);
    const speakingElapsedMsRef = useRef<number>(0);
    const currentVisemeIndexRef = useRef<number>(0);
    const audioElementRef = useRef<HTMLAudioElement | null>(null);

    useEffect(() => {
        const handleEvent = (e: Event) => {
            const customEvent = e as CustomEvent;
            if (customEvent.detail?.type === 'tts_started') {
                // Defensive null check (for fallback TTS path)
                const data = customEvent.detail.data ?? {};
                visemeQueueRef.current = Array.isArray(data.visemes) ? data.visemes : [];
                speakingElapsedMsRef.current = 0;
                currentVisemeIndexRef.current = 0;
                audioElementRef.current = data.audioElement ?? null;
            } else if (customEvent.detail?.type === 'tts_completed') {
                visemeQueueRef.current = [];
                audioElementRef.current = null;
            }
        };

        window.addEventListener('aiome_vitality_event', handleEvent);
        return () => window.removeEventListener('aiome_vitality_event', handleEvent);
    }, []);

    const tick = useCallback((deltaSec: number): VisemeSyncState => {
        if (visemeQueueRef.current.length === 0) {
            return { viseme: null, weight: 0, isActive: false };
        }

        let elapsedMs: number;
        if (audioElementRef.current) {
            // High-precision sync based on audio playback time
            elapsedMs = audioElementRef.current.currentTime * 1000;
        } else {
            // Fallback: procedural elapsed time based on frame delta
            speakingElapsedMsRef.current += deltaSec * 1000;
            elapsedMs = speakingElapsedMsRef.current;
        }

        const queue = visemeQueueRef.current;

        // Fast forward index if needed
        while (
            currentVisemeIndexRef.current < queue.length &&
            queue[currentVisemeIndexRef.current].timestamp_ms + queue[currentVisemeIndexRef.current].duration_ms < elapsedMs
        ) {
            currentVisemeIndexRef.current++;
        }

        if (currentVisemeIndexRef.current < queue.length) {
            const frame = queue[currentVisemeIndexRef.current];
            if (elapsedMs >= frame.timestamp_ms) {
                return { viseme: frame.viseme.toLowerCase(), weight: 0.85, isActive: true };
            }
            // Still waiting for the next viseme, so we're active but no viseme
            return { viseme: null, weight: 0, isActive: true };
        }

        // Queue is exhausted
        return { viseme: null, weight: 0, isActive: false };
    }, []);

    return { tick };
};

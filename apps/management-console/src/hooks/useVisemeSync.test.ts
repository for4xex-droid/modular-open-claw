/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import { renderHook, act } from '@testing-library/react';
import { useVisemeSync } from './useVisemeSync';

describe('useVisemeSync', () => {

    it('tts_started イベントで Viseme キューを初期化し、tick で現在の Viseme を返す', () => {
        const { result } = renderHook(() => useVisemeSync());

        // Arrange: モックイベントの送信
        const mockVisemes = [
            { viseme: 'aa', timestamp_ms: 0, duration_ms: 100 },
            { viseme: 'ih', timestamp_ms: 100, duration_ms: 100 },
        ];

        act(() => {
            window.dispatchEvent(new CustomEvent('aiome_vitality_event', {
                detail: {
                    type: 'tts_started',
                    data: { visemes: mockVisemes, audioElement: null }
                }
            }));
        });

        // Act & Assert 1: 初期状態 (delta = 0)
        let syncState = result.current.tick(0);
        expect(syncState.viseme).toBe('aa');
        expect(syncState.weight).toBeGreaterThan(0);
        
        // Act & Assert 2: 120ms 経過 (delta = 0.12)
        syncState = result.current.tick(0.12);
        expect(syncState.viseme).toBe('ih');

        // Act & Assert 3: 250ms 経過し、キューが空になった場合 (delta = 0.13 -> total 0.25)
        syncState = result.current.tick(0.13);
        expect(syncState.viseme).toBeNull();
    });

    it('audioElement が存在する場合は currentTime を基準に同期する', () => {
        const { result } = renderHook(() => useVisemeSync());

        const mockAudioElement = { currentTime: 0 } as HTMLAudioElement;
        const mockVisemes = [
            { viseme: 'ou', timestamp_ms: 0, duration_ms: 500 },
            { viseme: 'ee', timestamp_ms: 500, duration_ms: 500 },
        ];

        act(() => {
            window.dispatchEvent(new CustomEvent('aiome_vitality_event', {
                detail: {
                    type: 'tts_started',
                    data: { visemes: mockVisemes, audioElement: mockAudioElement }
                }
            }));
        });

        // initial state
        result.current.tick(0.1); 
        
        // 600ms 地点にシークしたと仮定
        mockAudioElement.currentTime = 0.6;
        
        const syncState = result.current.tick(0.1);
        expect(syncState.viseme).toBe('ee');
    });

    it('tts_completed でキューがクリアされる', () => {
        const { result } = renderHook(() => useVisemeSync());

        const mockVisemes = [
            { viseme: 'aa', timestamp_ms: 0, duration_ms: 1000 },
        ];

        act(() => {
            window.dispatchEvent(new CustomEvent('aiome_vitality_event', {
                detail: {
                    type: 'tts_started',
                    data: { visemes: mockVisemes, audioElement: null }
                }
            }));
        });

        expect(result.current.tick(0).viseme).toBe('aa');

        act(() => {
            window.dispatchEvent(new CustomEvent('aiome_vitality_event', {
                detail: { type: 'tts_completed', data: {} }
            }));
        });

        expect(result.current.tick(0.1).viseme).toBeNull();
    });

    it('visemeの再生がすべて終了すると isActive が false になる', () => {
        const { result } = renderHook(() => useVisemeSync());

        const mockVisemes = [
            { viseme: 'aa', timestamp_ms: 0, duration_ms: 100 },
        ];

        act(() => {
            window.dispatchEvent(new CustomEvent('aiome_vitality_event', {
                detail: {
                    type: 'tts_started',
                    data: { visemes: mockVisemes, audioElement: null }
                }
            }));
        });

        // initial state, queue holds item
        let syncState = result.current.tick(0.05);
        expect(syncState.isActive).toBe(true);
        expect(syncState.viseme).toBe('aa');

        // wait past the duration of the viseme
        syncState = result.current.tick(0.1);
        // now elapsed is 150ms, index moves to 1 (queue.length)
        expect(syncState.isActive).toBe(false);
        expect(syncState.viseme).toBeNull();
    });

    it('tts_started に空の visemes 配列を渡した場合、tick は即座に非アクティブを返す', () => {
        const { result } = renderHook(() => useVisemeSync());

        act(() => {
            window.dispatchEvent(new CustomEvent('aiome_vitality_event', {
                detail: {
                    type: 'tts_started',
                    data: { visemes: [], audioElement: null }
                }
            }));
        });

        const syncState = result.current.tick(0.1);
        expect(syncState.viseme).toBeNull();
        expect(syncState.isActive).toBe(false);
    });

    it('tts_started に data が欠落している場合でもクラッシュしない', () => {
        const { result } = renderHook(() => useVisemeSync());

        act(() => {
            window.dispatchEvent(new CustomEvent('aiome_vitality_event', {
                detail: {
                    type: 'tts_started',
                    data: null
                }
            }));
        });

        const syncState = result.current.tick(0.1);
        expect(syncState.viseme).toBeNull();
        expect(syncState.isActive).toBe(false);
    });

    it('連続した tts_started でキューが上書きされる', () => {
        const { result } = renderHook(() => useVisemeSync());

        act(() => {
            window.dispatchEvent(new CustomEvent('aiome_vitality_event', {
                detail: {
                    type: 'tts_started',
                    data: {
                        visemes: [{ viseme: 'aa', timestamp_ms: 0, duration_ms: 1000 }],
                        audioElement: null
                    }
                }
            }));
        });

        expect(result.current.tick(0).viseme).toBe('aa');

        // 2回目の tts_started でキューが置換される
        act(() => {
            window.dispatchEvent(new CustomEvent('aiome_vitality_event', {
                detail: {
                    type: 'tts_started',
                    data: {
                        visemes: [{ viseme: 'oh', timestamp_ms: 0, duration_ms: 1000 }],
                        audioElement: null
                    }
                }
            }));
        });

        // elapsed はリセットされるので、新しいキューの先頭が返る
        const syncState = result.current.tick(0);
        expect(syncState.viseme).toBe('oh');
    });

    it('viseme 間のギャップでは isActive=true, viseme=null を返す', () => {
        const { result } = renderHook(() => useVisemeSync());

        // 0-100ms: aa, 200-300ms: ih (100ms のギャップ)
        const mockVisemes = [
            { viseme: 'aa', timestamp_ms: 0, duration_ms: 100 },
            { viseme: 'ih', timestamp_ms: 200, duration_ms: 100 },
        ];

        act(() => {
            window.dispatchEvent(new CustomEvent('aiome_vitality_event', {
                detail: {
                    type: 'tts_started',
                    data: { visemes: mockVisemes, audioElement: null }
                }
            }));
        });

        // 150ms 地点 (aa の duration 後、ih の timestamp 前)
        const syncState = result.current.tick(0.15);
        expect(syncState.isActive).toBe(true);
        expect(syncState.viseme).toBeNull();
    });
});

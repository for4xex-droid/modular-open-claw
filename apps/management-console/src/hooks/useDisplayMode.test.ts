/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

import { renderHook, act } from '@testing-library/react';
import { useDisplayMode } from './useDisplayMode';

describe('useDisplayMode', () => {
    beforeEach(() => {
        localStorage.clear();
    });

    it('defaults to vrm when unset', () => {
        const { result } = renderHook(() => useDisplayMode());
        expect(result.current.mode).toBe('vrm');
    });

    it('migrates legacy inx to lite and persists lite (Phase E E5)', () => {
        localStorage.setItem('aiome_display_mode', 'inx');
        const { result } = renderHook(() => useDisplayMode());
        expect(result.current.mode).toBe('lite');
        expect(localStorage.getItem('aiome_display_mode')).toBe('lite');
    });

    it('rejects unknown saved values and falls back to vrm', () => {
        localStorage.setItem('aiome_display_mode', 'live2d');
        const { result } = renderHook(() => useDisplayMode());
        expect(result.current.mode).toBe('vrm');
    });

    it('setMode updates storage', () => {
        const { result } = renderHook(() => useDisplayMode());
        act(() => {
            result.current.setMode('off');
        });
        expect(result.current.mode).toBe('off');
        expect(localStorage.getItem('aiome_display_mode')).toBe('off');
    });
});

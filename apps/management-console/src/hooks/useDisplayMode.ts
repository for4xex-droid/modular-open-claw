/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import { useState, useEffect } from 'react';

export type DisplayMode = 'vrm' | 'lite' | 'off';

export const useDisplayMode = () => {
    const [mode, setMode] = useState<DisplayMode>(() => {
        const saved = localStorage.getItem('aiome_display_mode');
        // Phase E E5: legacy Inochi mode migrates to 2D lite
        if (saved === 'inx') {
            return 'lite';
        }
        if (saved === 'vrm' || saved === 'lite' || saved === 'off') {
            return saved;
        }
        return 'vrm';
    });

    useEffect(() => {
        localStorage.setItem('aiome_display_mode', mode);
    }, [mode]);

    return { mode, setMode };
};

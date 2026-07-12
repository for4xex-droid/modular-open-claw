/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

import React, {
    createContext,
    useCallback,
    useContext,
    useEffect,
    useMemo,
    useState,
} from 'react';
import { API_BASE } from '../config';
import { authenticatedFetch } from '../lib/auth';
import { SettingEntry, ViewMode } from '../types';

/** U2-1: migrate legacy view_mode values to simple | cockpit */
export const migrateViewMode = (raw: string): ViewMode => {
    if (raw === 'simple' || raw === 'cockpit') return raw;
    if (raw === 'beginner') return 'simple';
    if (raw === 'intermediate' || raw === 'advanced' || raw === 'expert') return 'cockpit';
    return 'cockpit';
};

/** @deprecated Use migrateViewMode */
export const normalizeViewMode = migrateViewMode;

interface ViewModeContextValue {
    viewMode: ViewMode;
    setViewMode: (mode: ViewMode) => Promise<void>;
}

const ViewModeContext = createContext<ViewModeContextValue | null>(null);

function readInitialViewMode(): ViewMode {
    const saved = localStorage.getItem('aiome_view_mode');
    return saved ? migrateViewMode(saved) : 'cockpit';
}

export const ViewModeProvider: React.FC<{ children: React.ReactNode }> = ({ children }) => {
    const [viewMode, setViewModeState] = useState<ViewMode>(readInitialViewMode);

    useEffect(() => {
        const fetchViewMode = async () => {
            try {
                const resp = await authenticatedFetch(`${API_BASE}/api/v1/settings?category=ui`);
                if (resp.ok) {
                    const data = await resp.json();
                    const modeSetting = data.find((s: SettingEntry) => s.key === 'view_mode');
                    if (modeSetting) {
                        const migrated = migrateViewMode(modeSetting.value);
                        setViewModeState(migrated);
                        localStorage.setItem('aiome_view_mode', migrated);
                    }
                }
            } catch (e) {
                console.error("Failed to fetch view mode", e);
            }
        };
        fetchViewMode();
    }, []);

    const updateViewMode = useCallback(async (newMode: ViewMode) => {
        setViewModeState(newMode);
        localStorage.setItem('aiome_view_mode', newMode);
        try {
            await authenticatedFetch(`${API_BASE}/api/v1/settings`, {
                method: 'PUT',
                body: JSON.stringify({ key: 'view_mode', value: newMode, category: 'ui' })
            });
        } catch (e) {
            console.error("Failed to update view mode", e);
        }
    }, []);

    const value = useMemo(
        () => ({ viewMode, setViewMode: updateViewMode }),
        [viewMode, updateViewMode],
    );

    return React.createElement(ViewModeContext.Provider, { value }, children);
};

export const useViewMode = (): ViewModeContextValue => {
    const ctx = useContext(ViewModeContext);
    if (!ctx) {
        throw new Error('useViewMode must be used within ViewModeProvider');
    }
    return ctx;
};

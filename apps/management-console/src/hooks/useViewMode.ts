/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

import { useState, useEffect } from 'react';
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

export const useViewMode = () => {
    const [viewMode, setViewMode] = useState<ViewMode>(() => {
        const saved = localStorage.getItem('aiome_view_mode');
        return saved ? migrateViewMode(saved) : 'cockpit';
    });

    useEffect(() => {
        const fetchViewMode = async () => {
            try {
                const resp = await authenticatedFetch(`${API_BASE}/api/v1/settings?category=ui`);
                if (resp.ok) {
                    const data = await resp.json();
                    const modeSetting = data.find((s: SettingEntry) => s.key === 'view_mode');
                    if (modeSetting) {
                        const migrated = migrateViewMode(modeSetting.value);
                        setViewMode(migrated);
                        localStorage.setItem('aiome_view_mode', migrated);
                    }
                }
            } catch (e) {
                console.error("Failed to fetch view mode", e);
            }
        };
        fetchViewMode();
    }, []);

    const updateViewMode = async (newMode: ViewMode) => {
        setViewMode(newMode);
        localStorage.setItem('aiome_view_mode', newMode);
        try {
            await authenticatedFetch(`${API_BASE}/api/v1/settings`, {
                method: 'PUT',
                body: JSON.stringify({ key: 'view_mode', value: newMode, category: 'ui' })
            });
        } catch (e) {
            console.error("Failed to update view mode", e);
        }
    };

    return { viewMode, setViewMode: updateViewMode };
};

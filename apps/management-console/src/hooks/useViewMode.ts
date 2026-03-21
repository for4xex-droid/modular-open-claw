/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

import { useState, useEffect } from 'react';
import { API_BASE } from '../config';
import { authenticatedFetch } from '../lib/auth';

export type ViewMode = 'beginner' | 'intermediate' | 'advanced';

export const useViewMode = () => {
    const [viewMode, setViewMode] = useState<ViewMode>(() => {
        const saved = localStorage.getItem('aiome_view_mode') as ViewMode;
        return saved || 'intermediate';
    });

    useEffect(() => {
        const fetchViewMode = async () => {
            try {
                const resp = await authenticatedFetch(`${API_BASE}/api/v1/settings?category=ui`);
                if (resp.ok) {
                    const data = await resp.json();
                    const modeSetting = data.find((s: any) => s.key === 'view_mode');
                    if (modeSetting) {
                        setViewMode(modeSetting.value as ViewMode);
                        localStorage.setItem('aiome_view_mode', modeSetting.value);
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

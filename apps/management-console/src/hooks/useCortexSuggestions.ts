/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import { useState, useCallback } from 'react';
import { authenticatedFetch } from '../lib/auth';
import { API_BASE } from '../config';

export function useCortexSuggestions() {
    const [suggestions, setSuggestions] = useState<string[]>([]);
    const [isLoading, setIsLoading] = useState(false);

    const fetchSuggestions = useCallback(async () => {
        if (suggestions.length > 0 || isLoading) return;
        
        setIsLoading(true);
        try {
            const res = await authenticatedFetch(`${API_BASE}/api/v1/cortex/suggestions`);
            
            if (!res.ok) throw new Error(`HTTP error! status: ${res.status}`);
            
            const data = await res.json();
            if (Array.isArray(data)) {
                setSuggestions(data);
            }
        } catch (e) {
            console.error('Failed to fetch Cortex suggestions:', e);
        } finally {
            setIsLoading(false);
        }
    }, [suggestions.length, isLoading]);

    return {
        suggestions,
        fetchSuggestions,
        isLoading
    };
}

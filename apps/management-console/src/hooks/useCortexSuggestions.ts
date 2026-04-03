import { useState, useCallback } from 'react';

export function useCortexSuggestions() {
    const [suggestions, setSuggestions] = useState<string[]>([]);
    const [isLoading, setIsLoading] = useState(false);

    const fetchSuggestions = useCallback(async () => {
        if (suggestions.length > 0 || isLoading) return;
        
        setIsLoading(true);
        try {
            const token = sessionStorage.getItem('aiome_secret') || '';
            const res = await fetch('/api/v1/cortex/suggestions', {
                headers: { 'Authorization': `Bearer ${token}` }
            });
            
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

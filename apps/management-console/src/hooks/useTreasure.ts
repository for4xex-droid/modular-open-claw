import { useState, useCallback, useEffect } from 'react';
import { TreasureItem, TreasureFeedback } from '../types';
import { authenticatedFetch } from '../lib/auth';
import { API_BASE } from '../config';

export const useTreasure = () => {
    const [items, setItems] = useState<TreasureItem[]>([]);
    const [loading, setLoading] = useState(false);
    const [error, setError] = useState<string | null>(null);

    const fetchTreasure = useCallback(async () => {
        setLoading(true);
        setError(null);
        try {
            const res = await authenticatedFetch(`${API_BASE}/api/v1/treasure`);
            if (!res.ok) {
                throw new Error(`Failed to fetch treasure: ${res.statusText}`);
            }
            const data: TreasureItem[] = await res.json();
            setItems(data);
        } catch (err: any) {
            setError(err.message);
            console.error(err);
        } finally {
            setLoading(false);
        }
    }, []);

    const recordFeedback = useCallback(async (item_id: string, action: string) => {
        try {
            const feedback: TreasureFeedback = { item_id, action };
            const res = await authenticatedFetch(`${API_BASE}/api/v1/treasure/feedback`, {
                method: 'POST',
                body: JSON.stringify(feedback),
            });
            if (!res.ok) {
                console.warn('Failed to record feedback', res.status);
            }
            return res.ok;
        } catch (err) {
            console.error('Error recording feedback', err);
            return false;
        }
    }, []);

    useEffect(() => {
        fetchTreasure();
    }, [fetchTreasure]);

    return { items, loading, error, refresh: fetchTreasure, recordFeedback };
};

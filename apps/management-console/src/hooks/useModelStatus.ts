/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import { useState, useCallback, useRef } from 'react';
import { API_BASE } from '../config';
import { authenticatedFetch, getAuthHeaders } from '../lib/auth';
import { fetchEventSource } from '@microsoft/fetch-event-source';
import type { components } from '../types/generated';

type ModelStatusResponse = components['schemas']['ModelStatusResponse'];

export const useModelStatus = () => {
    const [status, setStatus] = useState<ModelStatusResponse | null>(null);
    const [loading, setLoading] = useState<boolean>(true);
    const [error, setError] = useState<string | null>(null);
    const [pullProgress, setPullProgress] = useState<{ status: string; completed?: number; total?: number } | null>(null);
    const [isPulling, setIsPulling] = useState<boolean>(false);
    
    const abortControllerRef = useRef<AbortController | null>(null);

    const checkStatus = useCallback(async () => {
        setLoading(true);
        setError(null);
        try {
            const res = await authenticatedFetch(`${API_BASE}/api/v1/models/status`);
            if (!res.ok) {
                throw new Error(`Failed to fetch model status: ${res.status}`);
            }
            const data: ModelStatusResponse = await res.json();
            setStatus(data);
        } catch (err: any) {
            console.error("fetch status error:", err);
            setError(err.message || "Connection error");
        } finally {
            setLoading(false);
        }
    }, []);

    const pullModel = useCallback(async (modelName: string) => {
        setPullProgress({ status: "Preparing to download..." });
        setIsPulling(true);
        setError(null);
        
        if (abortControllerRef.current) {
            abortControllerRef.current.abort();
        }
        
        const ctrl = new AbortController();
        abortControllerRef.current = ctrl;
        
        try {
            await fetchEventSource(`${API_BASE}/api/v1/models/pull`, {
                method: 'POST',
                headers: {
                    ...getAuthHeaders(),
                    'Content-Type': 'application/json',
                },
                body: JSON.stringify({ name: modelName }),
                signal: ctrl.signal,
                onmessage(ev) {
                    if (ev.event === 'progress') {
                        try {
                            const data = JSON.parse(ev.data);
                            setPullProgress({
                                status: data.status || "Downloading...",
                                completed: data.completed,
                                total: data.total
                            });
                        } catch(e) {
                            // ignore json parse error
                        }
                    } else if (ev.event === 'done') {
                        setPullProgress({ status: "Success!" });
                        setIsPulling(false);
                        checkStatus(); // Refresh status when done
                    } else if (ev.event === 'error') {
                        let errStr = ev.data;
                        try {
                            errStr = JSON.parse(ev.data).error || errStr;
                        } catch {}
                        setError(errStr);
                        setIsPulling(false);
                        ctrl.abort(); // Cancel stream
                    }
                },
                onerror(err) {
                    console.error("fetchEventSource error:", err);
                    setError(err.message || "Failed to download model.");
                    setIsPulling(false);
                    throw err; // Stop retrying
                }
            });
        } catch (err: any) {
            if (err.name !== 'AbortError') {
                setError(err.message || "Failed to download model.");
            }
            setIsPulling(false);
        }
    }, [checkStatus]);

    const cancelPull = useCallback(() => {
        if (abortControllerRef.current) {
            abortControllerRef.current.abort();
            abortControllerRef.current = null;
        }
        setIsPulling(false);
        setPullProgress(null);
    }, []);

    return {
        status,
        loading,
        error,
        pullProgress,
        isPulling,
        checkStatus,
        pullModel,
        cancelPull
    };
};

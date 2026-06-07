/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

/**
 * セキュア認証トークン管理
 * - sessionStorage を使用（ブラウザ閉鎖で自動消去）
 * - 本番環境での 'dev_secret' フォールバックを廃止
 */
export const getAuthToken = (): string | null => {
    return sessionStorage.getItem('aiome_secret');
};

const normalizeHeaders = (headersInit?: HeadersInit): Record<string, string> => {
    if (!headersInit) return {};
    if (headersInit instanceof Headers) {
        const headers: Record<string, string> = {};
        headersInit.forEach((value, key) => {
            headers[key] = value;
        });
        return headers;
    }
    if (Array.isArray(headersInit)) {
        const headers: Record<string, string> = {};
        headersInit.forEach(([key, value]) => {
            headers[key] = value;
        });
        return headers;
    }
    return { ...headersInit };
};

/**
 * 認証済みの fetch リクエストを実行するためのヘルパー。
 */
export const authenticatedFetch = async (url: string, options: RequestInit = {}): Promise<Response> => {
    const token = getAuthToken();
    const headers = normalizeHeaders(options.headers);

    if (token) {
        headers['Authorization'] = `Bearer ${token}`;
    }

    // Handle JSON content type as default for body-bearing requests, but avoid overriding FormData boundaries
    if (options.body && !headers['Content-Type'] && !(options.body instanceof FormData)) {
        headers['Content-Type'] = 'application/json';
    }

    const response = await fetch(url, { ...options, headers });

    if (response.status === 402) {
        if (typeof window !== 'undefined') {
            window.dispatchEvent(new CustomEvent('stripe-402-payment-required'));
        }
    }

    return response;
};

/**
 * API の全エンドポイントで共通して使用する認証ヘッダーを生成します。
 */
export const getAuthHeaders = () => {
    const token = getAuthToken();
    return {
        ...(token ? { 'Authorization': `Bearer ${token}` } : {}),
    };
};

/**
 * トークンを SessionStorage に保存します。
 */
export const setAuthToken = (token: string): void => {
    sessionStorage.setItem('aiome_secret', token);
    sessionStorage.setItem('aiome_secret_updated_at', Date.now().toString());
};

/**
 * トークンを SessionStorage から削除します。
 */
export const clearAuthToken = (): void => {
    sessionStorage.removeItem('aiome_secret');
    sessionStorage.removeItem('aiome_secret_updated_at');
};

/**
 * 現在認証されているかどうかを判定します。
 */
export const isAuthenticated = (): boolean => {
    return getAuthToken() !== null;
};

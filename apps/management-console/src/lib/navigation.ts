/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

/**
 * 外部のURLや別のページへ安全にリダイレクトするためのユーティリティ関数。
 * テスト容易性のために独立した関数として定義しています。
 */
export const redirect = (url: string): void => {
    if (typeof window !== 'undefined' && window.location) {
        window.location.assign(url);
    }
};

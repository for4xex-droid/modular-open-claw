/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

/** SSE 経由で受信する Viseme フレームの共通定義 */
export interface VisemeFrame {
    viseme: string;
    timestamp_ms: number;
    duration_ms: number;
}

/**
 * アバターフォーマットごとの Viseme 適用アダプター。
 * VRM / Inochi2D / GLB 各レンダラーが実装する。
 */
export interface AvatarLipSyncAdapter {
    applyViseme(viseme: string, weight: number): void;
    resetVisemes(): void;
}

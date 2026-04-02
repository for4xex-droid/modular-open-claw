/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import { invoke } from "@tauri-apps/api/core";

/**
 * [Milestone 3] UI Dynamic Discovery
 * Tauri のバックエンドプロセスから API ポート/URL を動的に取得します。
 * これにより、サンドボックス化された環境や動的ポート設定に対応します。
 */
export class APIResolver {
  private static cachedUrl: string | null = null;

  /**
   * API のベース URL を取得します。
   * Tauri 環境下では Rust 側から取得し、ブラウザ環境下では VITE_API_BASE またはデフォルトを使用します。
   */
  static async resolve(): Promise<string> {
    if (this.cachedUrl) return this.cachedUrl;

    // 1. Tauri 環境のチェック
    if (window && (window as any).__TAURI_INTERNALS__) {
      try {
        console.log("🔍 [APIResolver] Detecting backend via Tauri invoke...");
        const url = await invoke<string>("get_api_url");
        const resolved = url.endsWith("/") ? url.slice(0, -1) : url;
        this.cachedUrl = resolved;
        console.log("✅ [APIResolver] Resolved Dynamic Backend:", this.cachedUrl);
        return resolved;
      } catch (e) {
        console.warn("⚠️ [APIResolver] Tauri invoke failed, falling back to default:", e);
      }
    }

    // 2. ブラウザ環境・フォールバック
    const envBase = import.meta.env.VITE_API_BASE;
    // In production (Docker), static UI is served by the backend, so we use window.location.origin.
    // In dev, the UI and API run on different ports, so we default to 3015.
    const finalFallback = envBase || (import.meta.env.DEV ? "http://localhost:3015" : window.location.origin);
    this.cachedUrl = finalFallback;
    return finalFallback;
  }
}

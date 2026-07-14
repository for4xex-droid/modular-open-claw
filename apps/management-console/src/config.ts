/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import { APIResolver } from "./lib/api_resolver";

export let API_BASE = import.meta.env.VITE_API_BASE || (import.meta.env.DEV ? "http://localhost:3015" : window.location.origin);
export const APP_VERSION = import.meta.env.VITE_APP_VERSION || "v1.0.2";
export const STRIPE_PRICE_ID = import.meta.env.VITE_STRIPE_PRICE_ID || "price_gold_monthly";

/** 利用規約の版（正本: docs/legal/TERMS_OF_SERVICE.md。docs/legal/CONSENT_SPEC.md §1） */
export const TOS_VERSION = "v2.1";
/** 公開法務ページのベース URL（利用規約・特商法・解約ポリシー） */
export const LEGAL_BASE_URL = "https://aiome.dev";

/**
 * [Milestone 3] UI Dynamic Discovery
 * アプリ起動時に API エンドポイントを動的に解決します。
 */
export const initApiBase = async () => {
  API_BASE = await APIResolver.resolve();
};

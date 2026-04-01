import { APIResolver } from "./lib/api_resolver";

export let API_BASE = import.meta.env.VITE_API_BASE || "http://localhost:3015";

/**
 * [Milestone 3] UI Dynamic Discovery
 * アプリ起動時に API エンドポイントを動的に解決します。
 */
export const initApiBase = async () => {
  API_BASE = await APIResolver.resolve();
};

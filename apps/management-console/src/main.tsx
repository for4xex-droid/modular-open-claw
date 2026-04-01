/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./App.css";
import { AvatarCharacterProvider } from "./hooks/AvatarContext";
import ErrorBoundary from "./components/ErrorBoundary";
import { initApiBase } from "./config";

/**
 * [Milestone 3] UI Dynamic Discovery
 * Tauri サンドボックスや動的ポート環境に対応するため、
 * アプリのレンダリング前にバックエンド URL を解決します。
 */
async function boot() {
  try {
    await initApiBase();
  } catch (e) {
    console.error("❌ [Main] Failed to initialize API Base:", e);
  }

  const root = document.getElementById("root");
  if (root) {
    ReactDOM.createRoot(root).render(
      <React.StrictMode>
        <ErrorBoundary>
          <AvatarCharacterProvider>
            <App />
          </AvatarCharacterProvider>
        </ErrorBoundary>
      </React.StrictMode>,
    );
  }
}

boot();

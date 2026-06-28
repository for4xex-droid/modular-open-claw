/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import React from "react";
import ReactDOM from "react-dom/client";
import { BiomeGame } from "./lib/biome/BiomeGame";
import "./App.css";
import ErrorBoundary from "./components/ErrorBoundary";
import { LanguageProvider } from "./i18n";
import { initApiBase } from "./config";
import { ToastProvider } from "./components/common/Toast";

async function boot() {
  try {
    await initApiBase();
  } catch (e) {
    console.error("❌ [Biome Popup] Failed to initialize API Base:", e);
  }

  const root = document.getElementById("root");
  if (root) {
    ReactDOM.createRoot(root).render(
      <React.StrictMode>
        <ErrorBoundary>
          <LanguageProvider>
            <ToastProvider>
              <div style={{
                width: '100vw',
                height: '100vh',
                display: 'flex',
                justifyContent: 'center',
                alignItems: 'center',
                background: '#030712',
                overflow: 'hidden'
              }}>
                <BiomeGame standalone={true} />
              </div>
            </ToastProvider>
          </LanguageProvider>
        </ErrorBoundary>
      </React.StrictMode>,
    );
  }
}

boot();

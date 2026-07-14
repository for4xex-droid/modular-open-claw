/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import { defineConfig, devices } from '@playwright/test';

/**
 * NT-5 証拠撮影専用。
 * - 通常の playwright.config.ts（webServer / :3015）とは分離
 * - パスワード直書き禁止 → NT5_ADMIN_PASSWORD 必須
 * - 既定 baseURL はローカル Vite（OrbStack）1420
 */
export default defineConfig({
  testDir: './e2e',
  testMatch: 'nt5_evidence.spec.ts',
  fullyParallel: false,
  workers: 1,
  timeout: 180_000,
  reporter: 'list',
  use: {
    baseURL: process.env.NT5_BASE_URL || 'http://127.0.0.1:1420',
    viewport: { width: 1920, height: 1080 },
    colorScheme: 'dark',
    trace: 'off',
  },
  projects: [
    {
      name: 'chromium',
      use: { ...devices['Desktop Chrome'], viewport: { width: 1920, height: 1080 } },
    },
  ],
});

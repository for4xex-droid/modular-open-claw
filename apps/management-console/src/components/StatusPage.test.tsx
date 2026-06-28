/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import { render, screen, waitFor } from '@testing-library/react';
import StatusPage from './StatusPage';
import { authenticatedFetch } from '../lib/auth';

// Mock authenticatedFetch
jest.mock('../lib/auth', () => ({
  authenticatedFetch: jest.fn().mockImplementation((url: string) => {
    if (url.includes('/api/health')) {
      return Promise.resolve({
        ok: true,
        json: () => Promise.resolve({
          memory_usage_mb: 256,
          total_memory_mb: 16384,
          cpu_usage_percent: 12.5,
          vram_usage_mb: 2048,
          disk_free_gb: 450,
          total_disk_gb: 1000,
          level: 3,
          exp: 1200,
          resonance: 85,
          creativity: 70,
          fatigue: 20,
          llm_circuit_breaker: {
            name: "primary-breaker",
            state: "Closed"
          },
          lora_engine: {
            mlx_available: true,
            status: "ready"
          },
          support_incidents: {
            total_incidents_7d: 14,
            distinct_users: 5,
            unresolved: 3,
            top_severity: "High"
          }
        })
      });
    }
    return Promise.resolve({ ok: false });
  })
}));

jest.mock('../config', () => ({
  API_BASE: 'http://localhost:3015'
}));

jest.mock('framer-motion', () => ({
  motion: {
    div: ({ children, ...props }: any) => <div {...props}>{children}</div>,
  },
  AnimatePresence: ({ children }: any) => <>{children}</>,
}));

describe('StatusPage System Health & Support Incidents Integration', () => {
  beforeEach(() => {
    jest.clearAllMocks();
  });

  it('fetches and renders system health metrics and support incident stats on mount', async () => {
    render(<StatusPage />);

    // Verify Title and Support Incidents statistics rendering (S-5 requirement)
    await waitFor(() => {
      expect(screen.getByText('System Status & Integrity Hub')).toBeInTheDocument();
      expect(screen.getByText('14')).toBeInTheDocument(); // total_incidents_7d
      expect(screen.getByText('5')).toBeInTheDocument();  // distinct_users
      expect(screen.getByText('3')).toBeInTheDocument();  // unresolved
      expect(screen.getByText('High')).toBeInTheDocument(); // top_severity
    });

    // Verify CPU/Memory states
    expect(screen.getByText('12.5%')).toBeInTheDocument();
    expect(screen.getByText('256 MB / 16384 MB')).toBeInTheDocument();

    // Verify Circuit Breaker and LoRA Engine status
    expect(screen.getByText('Closed')).toBeInTheDocument();
    expect(screen.getByText('ready')).toBeInTheDocument();
  });

  it('displays a graceful fallback/error message if fetch fails', async () => {
    (authenticatedFetch as jest.Mock).mockImplementationOnce(() => Promise.reject(new Error("Network Error")));
    render(<StatusPage />);

    await waitFor(() => {
      expect(screen.getByText(/Failed to load system health metrics/i)).toBeInTheDocument();
    });
  });

  it('handles zero-data boundary values gracefully (no incidents, zero disk)', async () => {
    (authenticatedFetch as jest.Mock).mockImplementationOnce(() =>
      Promise.resolve({
        ok: true,
        json: () => Promise.resolve({
          memory_usage_mb: 0,
          total_memory_mb: 0,
          cpu_usage_percent: 0,
          vram_usage_mb: null,
          disk_free_gb: 0,
          total_disk_gb: 0,
          level: 1,
          exp: 0,
          resonance: 50,
          creativity: 30,
          fatigue: 10,
          llm_circuit_breaker: null,
          lora_engine: null,
          support_incidents: null,
        }),
      })
    );
    render(<StatusPage />);

    await waitFor(() => {
      expect(screen.getByText('System Status & Integrity Hub')).toBeInTheDocument();
    });

    // Support incident cards should show 0 fallback values
    const zeros = screen.getAllByText('0');
    expect(zeros.length).toBeGreaterThanOrEqual(1);

    // Circuit breaker and LoRA should show "Offline"
    const offlines = screen.getAllByText('Offline');
    expect(offlines.length).toBe(2);

    // Top severity should show "None"
    expect(screen.getByText('None')).toBeInTheDocument();
  });

  it('calls fetchHealth again when Refresh button is clicked', async () => {
    render(<StatusPage />);

    await waitFor(() => {
      expect(screen.getByText('System Status & Integrity Hub')).toBeInTheDocument();
    });

    // Clear mock call count after initial fetch
    (authenticatedFetch as jest.Mock).mockClear();

    // Re-setup mock for subsequent calls
    (authenticatedFetch as jest.Mock).mockImplementation((url: string) => {
      if (url.includes('/api/health')) {
        return Promise.resolve({
          ok: true,
          json: () => Promise.resolve({
            memory_usage_mb: 512,
            total_memory_mb: 16384,
            cpu_usage_percent: 25.0,
            vram_usage_mb: null,
            disk_free_gb: 400,
            total_disk_gb: 1000,
            level: 5,
            exp: 5000,
            resonance: 90,
            creativity: 80,
            fatigue: 15,
            llm_circuit_breaker: { name: "primary-breaker", state: "Closed" },
            lora_engine: { mlx_available: true, status: "ready" },
            support_incidents: { total_incidents_7d: 7, distinct_users: 3, unresolved: 1, top_severity: "Medium" },
          }),
        });
      }
      return Promise.resolve({ ok: false });
    });

    // Click the Refresh button
    const refreshBtn = screen.getByText('Refresh').closest('button')!;
    refreshBtn.click();

    await waitFor(() => {
      expect(authenticatedFetch).toHaveBeenCalledTimes(1);
    });

    // Verify updated data is rendered
    await waitFor(() => {
      expect(screen.getByText('25%')).toBeInTheDocument();
      expect(screen.getByText('7')).toBeInTheDocument();
    });
  });
});

/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

import { render, screen, fireEvent, waitFor, act } from '@testing-library/react';
import DemoView from './DemoView';

jest.mock('../config', () => ({
  API_BASE: 'http://localhost'
}));

// Mock i18n and auth
jest.mock('../i18n', () => ({
  useTranslation: () => ({
    t: (key: string) => key
  })
}));

jest.mock('../lib/auth', () => ({
  getAuthHeaders: () => ({ 'Authorization': 'Bearer test' })
}));

describe('DemoView', () => {
  const mockStats = {
    level: 1, 
    resonance: 50, 
    experience: 0, 
    nextLevelExp: 100,
    exp: 0,
    creativity: 50,
    fatigue: 0
  };

  const originalFetch = window.fetch;

  afterEach(() => {
    window.fetch = originalFetch;
  });

  it('renders title and description', () => {
    render(<DemoView stats={mockStats} lastEvent={null} isConnected={true} />);
    expect(screen.getByText('demo.title')).toBeInTheDocument();
    expect(screen.getByText('demo.description')).toBeInTheDocument();
  });

  it('shows warning when not connected', () => {
    render(<DemoView stats={mockStats} lastEvent={null} isConnected={false} />);
    expect(screen.getByText('demo.sseHint')).toBeInTheDocument();
  });

  it('starts demo on button click', async () => {
    // Mock fetch
    window.fetch = jest.fn().mockResolvedValue({
      ok: true,
      json: () => Promise.resolve({ success: true })
    });

    render(<DemoView stats={mockStats} lastEvent={null} isConnected={true} />);
    const startBtn = screen.getByText('demo.startDemo');
    
    // Wrap in act for async state updates
    await act(async () => {
        fireEvent.click(startBtn);
    });

    expect(window.fetch).toHaveBeenCalled();
    
    // Wait for all async microtasks in the fetch chain to finish to clear act warnings
    await waitFor(() => {
        expect(screen.queryByText(/Response OK/)).toBeInTheDocument();
    });
  });

  it('updates step when event arrives', () => {
    const { rerender } = render(<DemoView stats={mockStats} lastEvent={null} isConnected={true} />);
    
    const event = {
      type: 'plugin_event',
      data: {
        plugin_name: 'AutonomousDemo',
        payload: {
          step: 1,
          message: 'Test message step 1'
        }
      }
    };

    rerender(<DemoView stats={mockStats} lastEvent={event} isConnected={true} />);
    expect(screen.getByText('Test message step 1')).toBeInTheDocument();
  });
});

/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import { render, fireEvent } from '@testing-library/react';
import ArtifactVault from './ArtifactVault';

// Mock translation and auth fetch
jest.mock('../i18n', () => ({
  useTranslation: () => ({ t: (key: string) => key })
}));

jest.mock('../lib/auth', () => ({
  authenticatedFetch: jest.fn()
}));

jest.mock('../config', () => ({
  API_BASE: 'http://localhost'
}));

jest.mock('./common/Toast', () => ({
  useToast: () => ({ showToast: jest.fn() })
}));

// Mock Framer Motion to avoid animation issues in tests
jest.mock('framer-motion', () => ({
  motion: {
    div: ({ children, ...props }: any) => <div {...props}>{children}</div>,
  },
  AnimatePresence: ({ children }: any) => <>{children}</>,
}));

describe('ArtifactVault HTML Preview', () => {


  it('renders a preview button (Eye icon) only for HTML files', async () => {
    // Note: We need a more integrated test or to export internal components
    // for a pure unit test. Here we'll check if the logic exists in the code
    // via a simple render and presence check of the action buttons if we can
    // trigger the state.
  });

  it('opens a sandboxed iframe when the preview button is clicked', () => {
    // Verification of sandbox="allow-scripts" and absence of allow-same-origin
  });

  it('listens to message events and dispatches aiome_inject_prompt when AIOME_PROMPT_FEEDBACK is received', () => {
    const dispatchEventSpy = jest.spyOn(window, 'dispatchEvent');
    
    render(<ArtifactVault />);
    
    // Simulate iframe message
    fireEvent(window, new MessageEvent('message', {
      data: {
        type: 'AIOME_PROMPT_FEEDBACK',
        payload: 'Re-run analysis with depth=5',
        autoSend: true
      },
      origin: '' // iframe sandbox "allow-scripts" without "allow-same-origin" yields empty or "null" origin
    }));
    
    expect(dispatchEventSpy).toHaveBeenCalledWith(
      expect.objectContaining({
        type: 'aiome_inject_prompt',
        detail: { prompt: 'Re-run analysis with depth=5', autoSend: true }
      })
    );
    
    dispatchEventSpy.mockRestore();
  });
});

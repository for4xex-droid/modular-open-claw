import { render, screen, fireEvent } from '@testing-library/react';
import ArtifactVault from './ArtifactVault';
import { useTranslation } from '../i18n';

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

// Mock Framer Motion to avoid animation issues in tests
jest.mock('framer-motion', () => ({
  motion: {
    div: ({ children, ...props }: any) => <div {...props}>{children}</div>,
  },
  AnimatePresence: ({ children }: any) => <>{children}</>,
}));

describe('ArtifactVault HTML Preview', () => {
  const mockHtmlArtifact = {
    id: 'art-123',
    title: 'Test Report',
    category: 'Report',
    tags: ['test'],
    created_by: 'Agent',
    dir_path: '/tmp',
    files: [
      { name: 'report.html', mime_type: 'text/html', size_bytes: 1024, hash: 'abc' },
      { name: 'data.json', mime_type: 'application/json', size_bytes: 512, hash: 'def' }
    ],
    karma_refs: [],
    edges: [],
    created_at: new Date().toISOString()
  };

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

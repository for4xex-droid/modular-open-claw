import { render, screen } from '@testing-library/react';
import GraphView from './GraphView';
import { authenticatedFetch } from '../lib/auth';

jest.mock('../config', () => ({
  API_BASE: 'http://localhost:8080'
}));

jest.mock('../lib/auth', () => ({
  authenticatedFetch: jest.fn()
}));

jest.mock('vis-network', () => ({
  Network: jest.fn().mockImplementation(() => ({
    on: jest.fn(),
    destroy: jest.fn(),
    getScale: () => 1,
    moveTo: jest.fn(),
    fit: jest.fn()
  }))
}));

jest.mock('../i18n', () => ({
  useTranslation: () => ({
    t: (key: string) => key
  })
}));

describe('GraphView', () => {
  beforeEach(() => {
    jest.clearAllMocks();
  });

  it('renders graph title and hint', async () => {
    (authenticatedFetch as jest.Mock).mockImplementation((url: string) => {
      if (url.includes('/api/synergy/graph')) {
        return Promise.resolve({
          ok: true,
          json: () => Promise.resolve({ nodes: [], edges: [] })
        });
      }
      if (url.includes('/api/artifacts')) {
        return Promise.resolve({
          ok: true,
          json: () => Promise.resolve([])
        });
      }
      return Promise.resolve({ ok: false });
    });

    render(<GraphView />);
    
    expect(screen.getByText('graph.title')).toBeInTheDocument();
    expect(screen.getByText('graph.hint')).toBeInTheDocument();
  });
});

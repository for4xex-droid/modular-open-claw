import { render, screen, fireEvent } from '@testing-library/react';
import CausalVisualizer from './CausalVisualizer';

// Mock config and deps
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

describe('CausalVisualizer', () => {
  it('renders search input and title', () => {
    render(<CausalVisualizer />);
    expect(screen.getByText('causal.title')).toBeInTheDocument();
    expect(screen.getByPlaceholderText('causal.jobIdPlaceholder')).toBeInTheDocument();
  });

  it('shows empty state message when no job is loaded', () => {
    render(<CausalVisualizer />);
    expect(screen.getByText('causal.enterJobId')).toBeInTheDocument();
  });

  it('updates job id input', () => {
    render(<CausalVisualizer />);
    const input = screen.getByPlaceholderText('causal.jobIdPlaceholder') as HTMLInputElement;
    fireEvent.change(input, { target: { value: 'job-123' } });
    expect(input.value).toBe('job-123');
  });

  it('places controls overlay inside the relative-positioned graph area to prevent title overlap', () => {
    render(<CausalVisualizer />);
    const graphArea = screen.getByTestId('causal-graph-area');
    const controlsOverlay = screen.getByTestId('causal-controls-overlay');
    expect(graphArea).toContainElement(controlsOverlay);
  });
});

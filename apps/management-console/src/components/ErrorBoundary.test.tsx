
import { render, screen } from '@testing-library/react';
import ErrorBoundary from './ErrorBoundary';

// Helper component that throws an error when rendered
const Bomb = () => {
  throw new Error('Test error');
};

describe('ErrorBoundary', () => {
  it('renders children when no error occurs', () => {
    render(
      <ErrorBoundary>
        <div data-testid="child">Child content</div>
      </ErrorBoundary>
    );
    expect(screen.getByTestId('child')).toBeInTheDocument();
  });

  it('catches errors and displays fallback UI', () => {
    // Suppress console.error output for the test
    jest.spyOn(console, 'error').mockImplementation(() => {});
    render(
      <ErrorBoundary>
        <Bomb />
      </ErrorBoundary>
    );
    // The fallback UI contains the default title
    expect(screen.getByText('System Error Detected')).toBeInTheDocument();
    // The error message should be displayed in the debug area
    expect(screen.getByText(/Test error/)).toBeInTheDocument();
    // Restore console.error
    (console.error as jest.Mock).mockRestore();
  });

  it('uses custom errorTitle prop when provided', () => {
    jest.spyOn(console, 'error').mockImplementation(() => {});
    render(
      <ErrorBoundary errorTitle="Custom Title">
        <Bomb />
      </ErrorBoundary>
    );
    expect(screen.getByText('Custom Title')).toBeInTheDocument();
    (console.error as jest.Mock).mockRestore();
  });
});

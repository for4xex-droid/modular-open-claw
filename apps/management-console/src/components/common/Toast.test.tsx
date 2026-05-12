import { render, screen, act, waitFor } from '@testing-library/react';
import { ToastProvider, useToast } from './Toast';

// Mock framer-motion to bypass animation delays in tests
declare const require: any;
jest.mock('framer-motion', () => {
  const React = require('react');
  return {
    motion: {
      div: ({ children, ...props }: any) => {
        const { initial, animate, exit, ...rest } = props;
        return React.createElement('div', rest, children);
      },
    },
    AnimatePresence: ({ children }: any) => children,
  };
});

describe('ToastProvider and useToast', () => {
  beforeEach(() => {
    jest.useFakeTimers();
  });

  afterEach(() => {
    jest.useRealTimers();
  });

  it('renders a toast message when show is called', () => {
    const TestComponent = () => {
      const { showToast } = useToast();
      return (
        <button onClick={() => showToast('success', 'Test Success Message')}>
          Show Success Toast
        </button>
      );
    };

    render(
      <ToastProvider>
        <TestComponent />
      </ToastProvider>
    );

    expect(screen.queryByText('Test Success Message')).not.toBeInTheDocument();

    act(() => {
      screen.getByText('Show Success Toast').click();
    });

    expect(screen.getByText('Test Success Message')).toBeInTheDocument();
  });

  it('automatically dismisses the toast after 5 seconds', async () => {
    const TestComponent = () => {
      const { showToast } = useToast();
      return (
        <button onClick={() => showToast('error', 'Test Error Message')}>
          Show Error Toast
        </button>
      );
    };

    render(
      <ToastProvider>
        <TestComponent />
      </ToastProvider>
    );

    act(() => {
      screen.getByText('Show Error Toast').click();
    });

    expect(screen.getByText('Test Error Message')).toBeInTheDocument();

    act(() => {
      jest.advanceTimersByTime(5000);
    });

    await waitFor(() => {
      expect(screen.queryByText('Test Error Message')).not.toBeInTheDocument();
    });
  });
});

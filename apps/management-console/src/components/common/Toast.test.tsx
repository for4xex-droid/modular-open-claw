/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
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

  it('queues up to 3 toasts simultaneously', () => {
    const TestComponent = () => {
      const { showToast } = useToast();
      return (
        <>
          <button onClick={() => showToast('success', 'Toast One')}>Show One</button>
          <button onClick={() => showToast('error', 'Toast Two')}>Show Two</button>
          <button onClick={() => showToast('success', 'Toast Three')}>Show Three</button>
          <button onClick={() => showToast('error', 'Toast Four')}>Show Four</button>
        </>
      );
    };

    render(
      <ToastProvider>
        <TestComponent />
      </ToastProvider>
    );

    act(() => {
      screen.getByText('Show One').click();
      screen.getByText('Show Two').click();
      screen.getByText('Show Three').click();
      screen.getByText('Show Four').click();
    });

    expect(screen.queryByText('Toast One')).not.toBeInTheDocument();
    expect(screen.getByText('Toast Two')).toBeInTheDocument();
    expect(screen.getByText('Toast Three')).toBeInTheDocument();
    expect(screen.getByText('Toast Four')).toBeInTheDocument();
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

/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import { render, screen, fireEvent } from '@testing-library/react';
import ConfirmModal from './ConfirmModal';

// Mock framer-motion to bypass animation delays
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

describe('ConfirmModal', () => {
  it('renders nothing when isOpen is false', () => {
    const { container } = render(
      <ConfirmModal
        isOpen={false}
        title="Test Title"
        message="Test Message"
        onConfirm={jest.fn()}
        onCancel={jest.fn()}
      />
    );
    expect(container).toBeEmptyDOMElement();
  });

  it('renders title, message and details when isOpen is true', () => {
    render(
      <ConfirmModal
        isOpen={true}
        title="Delete Item"
        message="Are you sure?"
        details="This action cannot be undone."
        onConfirm={jest.fn()}
        onCancel={jest.fn()}
      />
    );
    
    expect(screen.getByText('Delete Item')).toBeInTheDocument();
    expect(screen.getByText('Are you sure?')).toBeInTheDocument();
    expect(screen.getByText('This action cannot be undone.')).toBeInTheDocument();
  });

  it('calls onConfirm when confirm button is clicked', () => {
    const onConfirmMock = jest.fn();
    render(
      <ConfirmModal
        isOpen={true}
        title="Title"
        message="Message"
        confirmText="Yes, delete it"
        onConfirm={onConfirmMock}
        onCancel={jest.fn()}
      />
    );
    
    fireEvent.click(screen.getByText('Yes, delete it'));
    expect(onConfirmMock).toHaveBeenCalledTimes(1);
  });

  it('calls onCancel when cancel button is clicked', () => {
    const onCancelMock = jest.fn();
    render(
      <ConfirmModal
        isOpen={true}
        title="Title"
        message="Message"
        cancelText="No, wait"
        onConfirm={jest.fn()}
        onCancel={onCancelMock}
      />
    );
    
    fireEvent.click(screen.getByText('No, wait'));
    expect(onCancelMock).toHaveBeenCalledTimes(1);
  });
});

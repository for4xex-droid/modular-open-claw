/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import React from 'react';
import { render, screen, fireEvent } from '@testing-library/react';
import { ToxicityConfig } from './ToxicityConfig';

// Mock translation hook
jest.mock('../i18n', () => ({
  useTranslation: () => ({
    t: (key: string, options?: any) => options?.defaultValue || key,
  }),
}));

describe('ToxicityConfig', () => {
  it('renders correctly with no blocked words', () => {
    const mockOnUpdate = jest.fn();
    render(<ToxicityConfig value="" onUpdate={mockOnUpdate} />);
    
    expect(screen.getByText('Content Safety Filter')).toBeInTheDocument();
    expect(screen.getByText('No blocked words.')).toBeInTheDocument();
  });

  it('renders existing words correctly', () => {
    const mockOnUpdate = jest.fn();
    render(<ToxicityConfig value="badword1, badword2" onUpdate={mockOnUpdate} />);
    
    expect(screen.getByText('badword1')).toBeInTheDocument();
    expect(screen.getByText('badword2')).toBeInTheDocument();
  });

  it('adds a new word correctly', () => {
    const mockOnUpdate = jest.fn();
    render(<ToxicityConfig value="badword1" onUpdate={mockOnUpdate} />);
    
    const input = screen.getByPlaceholderText('Enter a banned word...');
    fireEvent.change(input, { target: { value: 'badword2' } });
    fireEvent.keyDown(input, { key: 'Enter', code: 'Enter' });

    expect(mockOnUpdate).toHaveBeenCalledWith('badword1,badword2');
  });

  it('does not add empty words or duplicates', () => {
    const mockOnUpdate = jest.fn();
    render(<ToxicityConfig value="badword1" onUpdate={mockOnUpdate} />);
    
    const input = screen.getByPlaceholderText('Enter a banned word...');
    
    // Empty word
    fireEvent.change(input, { target: { value: '   ' } });
    fireEvent.keyDown(input, { key: 'Enter', code: 'Enter' });
    expect(mockOnUpdate).not.toHaveBeenCalled();

    // Duplicate word
    fireEvent.change(input, { target: { value: 'badword1' } });
    fireEvent.keyDown(input, { key: 'Enter', code: 'Enter' });
    expect(mockOnUpdate).toHaveBeenCalledWith('badword1');
  });

  it('removes a word correctly', () => {
    const mockOnUpdate = jest.fn();
    render(<ToxicityConfig value="badword1, badword2" onUpdate={mockOnUpdate} />);
    
    // The X icon has an onClick handler. We can find the parent div of 'badword1' and click the svg inside.
    const badword1 = screen.getByText('badword1');
    const removeIcon = badword1.parentElement?.querySelector('svg');
    
    if (removeIcon) {
        fireEvent.click(removeIcon);
    }
    
    expect(mockOnUpdate).toHaveBeenCalledWith('badword2');
  });
});

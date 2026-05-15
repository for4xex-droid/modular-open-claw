import React from 'react';
import { render, screen, fireEvent } from '@testing-library/react';
import { OriginManager } from './OriginManager';

// Mock translation hook
jest.mock('../i18n', () => ({
  useTranslation: () => ({
    t: (key: string, options?: any) => options?.defaultValue || key,
  }),
}));

describe('OriginManager', () => {
  it('renders correctly with no origins', () => {
    const mockOnUpdate = jest.fn();
    render(<OriginManager value="" onUpdate={mockOnUpdate} />);
    
    expect(screen.getByText('settings.allowedOrigins')).toBeInTheDocument();
  });

  it('renders existing origins correctly', () => {
    const mockOnUpdate = jest.fn();
    render(<OriginManager value="http://localhost:3000, https://example.com" onUpdate={mockOnUpdate} />);
    
    expect(screen.getByText('http://localhost:3000')).toBeInTheDocument();
    expect(screen.getByText('https://example.com')).toBeInTheDocument();
  });

  it('adds a new origin correctly', () => {
    const mockOnUpdate = jest.fn();
    render(<OriginManager value="http://localhost:3000" onUpdate={mockOnUpdate} />);
    
    const input = screen.getByPlaceholderText('https://example.com');
    fireEvent.change(input, { target: { value: 'https://test.com' } });
    
    const addButton = screen.getByText('settings.add');
    fireEvent.click(addButton);

    expect(mockOnUpdate).toHaveBeenCalledWith('http://localhost:3000,https://test.com');
  });

  it('shows error on duplicate origin', () => {
    const mockOnUpdate = jest.fn();
    render(<OriginManager value="http://localhost:3000" onUpdate={mockOnUpdate} />);
    
    const input = screen.getByPlaceholderText('https://example.com');
    fireEvent.change(input, { target: { value: 'http://localhost:3000' } });
    
    const addButton = screen.getByText('settings.add');
    fireEvent.click(addButton);

    expect(mockOnUpdate).not.toHaveBeenCalled();
    expect(screen.getByText('settings.originExists')).toBeInTheDocument();
  });

  it('removes an origin correctly', () => {
    const mockOnUpdate = jest.fn();
    render(<OriginManager value="http://localhost:3000, https://example.com" onUpdate={mockOnUpdate} />);
    
    const origin1 = screen.getByText('http://localhost:3000');
    const removeIcon = origin1.parentElement?.querySelector('svg');
    
    if (removeIcon) {
        fireEvent.click(removeIcon);
    }
    
    expect(mockOnUpdate).toHaveBeenCalledWith('https://example.com');
  });
});

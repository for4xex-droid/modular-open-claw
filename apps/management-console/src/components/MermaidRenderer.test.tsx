import React from 'react';
import { render, screen } from '@testing-library/react';
import { MermaidRenderer } from './MermaidRenderer';
import { renderMermaidSVG } from 'beautiful-mermaid';

// Mock beautiful-mermaid to avoid actual heavy parsing in tests
jest.mock('beautiful-mermaid', () => ({
    renderMermaidSVG: jest.fn()
}), { virtual: true });

describe('MermaidRenderer', () => {
    beforeEach(() => {
        jest.clearAllMocks();
    });

    it('renders SVG successfully when valid mermaid code is provided', () => {
        // Arrange
        const mockSvg = '<svg data-testid="mermaid-svg"><g><text>Test Graph</text></g></svg>';
        (renderMermaidSVG as jest.Mock).mockReturnValue(mockSvg);
        const code = 'graph TD\nA-->B';

        // Act
        render(<MermaidRenderer code={code} />);

        // Assert
        expect(renderMermaidSVG).toHaveBeenCalledWith(code, expect.objectContaining({
            transparent: true,
            bg: expect.any(String),
            fg: expect.any(String)
        }));
        expect(screen.getByTestId('mermaid-svg')).toBeInTheDocument();
    });

    it('renders an error fallback when rendering fails', () => {
        // Arrange
        (renderMermaidSVG as jest.Mock).mockImplementation(() => {
            throw new Error('Parse error');
        });
        const code = 'invalid code';

        // Act
        render(<MermaidRenderer code={code} />);

        // Assert
        expect(screen.getByText(/Failed to render diagram/i)).toBeInTheDocument();
    });
});

import { render, screen } from '@testing-library/react';
import { Hero } from './Hero';
import '../i18n/config'; // Setup i18n for tests

describe('Hero Component', () => {
  it('renders the hero title and subtitle', () => {
    render(<Hero />);
    
    // Test the specific copy we defined in our plan
    expect(screen.getByText('Build AI that heals itself.')).toBeInTheDocument();
    
    // Subtitle should contain key phrases
    const subtitleText = screen.getByText(/The autonomous operating system for AI agents/i);
    expect(subtitleText).toBeInTheDocument();
  });

  it('renders call-to-action links with correct destinations', () => {
    render(<Hero />);
    
    const primaryLink = screen.getByRole('link', { name: /Get Started Free/i });
    expect(primaryLink).toBeInTheDocument();
    expect(primaryLink).toHaveAttribute('href', '#quickstart');

    const secondaryLink = screen.getByRole('link', { name: /View on GitHub/i });
    expect(secondaryLink).toBeInTheDocument();
    expect(secondaryLink).toHaveAttribute('href', 'https://github.com/motivationstudio-llc/aiome');
    expect(secondaryLink).toHaveAttribute('target', '_blank');
    expect(secondaryLink).toHaveAttribute('rel', 'noopener noreferrer');
  });
});

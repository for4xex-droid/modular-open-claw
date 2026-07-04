/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import { render, screen } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import { Hero } from './Hero';
import '../i18n/config'; // Setup i18n for tests

// Mock FluidHeroBackground to avoid WebGL errors in test environment
vi.mock('./FluidHeroBackground', () => ({
  FluidHeroBackground: () => <div data-testid="mock-fluid-hero-background" />
}));


describe('Hero Component', () => {
  it('renders the hero title and subtitle', () => {
    render(<Hero />);
    
    // Test the specific copy we defined in our plan
    expect(screen.getByText('The sovereign OS for autonomous AI.')).toBeInTheDocument();
    
    // Subtitle should contain key phrases (A2C hook)
    const subtitleText = screen.getByText(/Care for your AI every day/i);
    expect(subtitleText).toBeInTheDocument();
  });

  it('renders the fluid background component', () => {
    render(<Hero />);
    expect(screen.getByTestId('mock-fluid-hero-background')).toBeInTheDocument();
  });

  it('renders the aiome OGP hero logo (white-ogp)', () => {
    render(<Hero />);
    const logoImg = screen.getByTestId('hero-logo');
    expect(logoImg).toBeInTheDocument();
    expect(logoImg).toHaveAttribute('src', '/aiome-hero-white.png');
    expect(logoImg).toHaveAttribute('alt', 'Aiome logo');
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


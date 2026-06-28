/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import { render, screen, fireEvent } from '@testing-library/react';
import { Navbar } from './Navbar';
import i18n from '../i18n/config';

describe('Navbar Component', () => {
  beforeEach(() => {
    i18n.changeLanguage('en');
  });

  it('renders navigation links in english by default', () => {
    render(<Navbar />);
    expect(screen.getByText('Features')).toBeInTheDocument();
    expect(screen.getByText('Quickstart')).toBeInTheDocument();
  });

  it('renders the aiome icon logo', () => {
    render(<Navbar />);
    const logoImg = screen.getByTestId('navbar-logo');
    expect(logoImg).toHaveAttribute('src', '/aiome-horizontal-white.png');
  });

  it('renders external links with target=_blank and noopener', () => {
    render(<Navbar />);
    const githubLink = screen.getByText('GitHub').closest('a');
    expect(githubLink).toHaveAttribute('target', '_blank');
    expect(githubLink).toHaveAttribute('rel', 'noopener noreferrer');

    const docsLink = screen.getByText('Documentation').closest('a');
    expect(docsLink).toHaveAttribute('target', '_blank');
    expect(docsLink).toHaveAttribute('rel', 'noopener noreferrer');
  });

  it('toggles language when the language button is clicked', async () => {
    render(<Navbar />);
    const langBtn = screen.getByRole('button', { name: /Switch to Japanese/i });
    expect(langBtn).toBeInTheDocument();
    
    fireEvent.click(langBtn);
    
    expect(await screen.findByText('機能')).toBeInTheDocument();
    expect(await screen.findByText('クイックスタート')).toBeInTheDocument();
  });

  it('renders a skip link for accessibility (i18n)', () => {
    render(<Navbar />);
    const skipLink = screen.getByText('Skip to main content');
    expect(skipLink).toBeInTheDocument();
    expect(skipLink).toHaveAttribute('href', '#main-content');
  });

  it('renders mobile menu toggle and opens/closes it', () => {
    render(<Navbar />);
    const menuBtn = screen.getByRole('button', { name: /Open menu/i });
    expect(menuBtn).toBeInTheDocument();
    expect(menuBtn).toHaveAttribute('aria-expanded', 'false');

    // Open menu
    fireEvent.click(menuBtn);
    
    // After click, the button label changes to "Close menu"
    const closeBtn = screen.getByRole('button', { name: /Close menu/i });
    expect(closeBtn).toHaveAttribute('aria-expanded', 'true');

    // Mobile links should now be visible (duplicated from desktop)
    const featureLinks = screen.getAllByText('Features');
    expect(featureLinks.length).toBeGreaterThanOrEqual(2);

    // Close menu
    fireEvent.click(closeBtn);
    const reopenBtn = screen.getByRole('button', { name: /Open menu/i });
    expect(reopenBtn).toHaveAttribute('aria-expanded', 'false');
  });

  it('closes mobile menu when Escape key is pressed', () => {
    render(<Navbar />);
    const menuBtn = screen.getByRole('button', { name: /Open menu/i });
    
    // Open menu
    fireEvent.click(menuBtn);
    expect(screen.getByRole('button', { name: /Close menu/i })).toHaveAttribute('aria-expanded', 'true');

    // Press Escape
    fireEvent.keyDown(document, { key: 'Escape' });

    // Menu should now be closed
    const reopenBtn = screen.getByRole('button', { name: /Open menu/i });
    expect(reopenBtn).toHaveAttribute('aria-expanded', 'false');
  });
});

/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import { render, screen } from '@testing-library/react';
import { Footer } from './Footer';
import '../i18n/config';

describe('Footer Component', () => {
  it('renders copyright info', () => {
    render(<Footer />);
    expect(screen.getByText(/2026 MotivationStudio LLC/i)).toBeInTheDocument();
  });

  it('renders privacy, terms, cancellation, and tokushoho links', () => {
    render(<Footer />);
    expect(screen.getByText('Privacy Policy')).toBeInTheDocument();
    expect(screen.getByText('Terms of Service')).toBeInTheDocument();
    expect(screen.getByText('Cancellation & Refund')).toBeInTheDocument();

    const cancellationLink = screen.getByText('Cancellation & Refund');
    expect(cancellationLink.closest('a')).toHaveAttribute('href', '/cancellation');

    const tokushohoLink = screen.getByText('Specified Commercial Transactions Act');
    expect(tokushohoLink).toBeInTheDocument();
    expect(tokushohoLink.closest('a')).toHaveAttribute('href', '/tokushoho');
  });

  it('renders GitHub and X (Twitter) social links with correct href and target', () => {
    render(<Footer />);
    const githubLink = screen.getByRole('link', { name: /GitHub/i });
    const xLink = screen.getByRole('link', { name: /X \(formerly Twitter\)/i });
    
    expect(githubLink).toBeInTheDocument();
    expect(githubLink).toHaveAttribute('href', 'https://github.com/motivationstudio-llc/aiome');
    expect(githubLink).toHaveAttribute('target', '_blank');
    expect(githubLink).toHaveAttribute('rel', 'noopener noreferrer');
    
    expect(xLink).toBeInTheDocument();
    expect(xLink).toHaveAttribute('href', 'https://x.com/aiome_dev');
    expect(xLink).toHaveAttribute('target', '_blank');
    expect(xLink).toHaveAttribute('rel', 'noopener noreferrer');
  });
});

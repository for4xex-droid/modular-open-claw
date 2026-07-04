/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import { render, screen } from '@testing-library/react';
import { CTA } from './CTA';
import '../i18n/config';

describe('CTA Component', () => {
  it('renders the CTA title and description', () => {
    render(<CTA />);
    expect(screen.getByText('Five minutes from now, your AI team is working.')).toBeInTheDocument();
    expect(screen.getByText(/One Docker command and setup is done/i)).toBeInTheDocument();
  });

  it('renders a single CTA link to #quickstart', () => {
    render(<CTA />);
    const ctaLink = screen.getByRole('link', { name: /Get Started Free/i });
    expect(ctaLink).toBeInTheDocument();
    expect(ctaLink).toHaveAttribute('href', '#quickstart');
  });

  it('does not render email subscription form elements', () => {
    render(<CTA />);
    expect(screen.queryByPlaceholderText(/email/i)).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: /Get Early Access/i })).not.toBeInTheDocument();
    expect(screen.queryByLabelText(/I agree to receive updates/i)).not.toBeInTheDocument();
  });
});

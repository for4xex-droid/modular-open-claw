/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import { render, screen } from '@testing-library/react';
import { Features } from './Features';
import '../i18n/config';

describe('Features Component (Bento Grid)', () => {
  it('renders section title', () => {
    render(<Features />);
    expect(screen.getByText('Built for absolute resilience.')).toBeInTheDocument();
  });

  it('renders the core features with expanded copy', () => {
    render(<Features />);
    
    // Check titles
    expect(screen.getByText('100% Autonomous')).toBeInTheDocument();
    expect(screen.getByText('Mathematically Proven')).toBeInTheDocument();
    expect(screen.getByText('Zero-Panic Rust')).toBeInTheDocument();
    expect(screen.getByText('AI Marketplace')).toBeInTheDocument();
    expect(screen.getByText('Self-Sustaining Economy')).toBeInTheDocument();
    expect(screen.getByText('A2C Rewards')).toBeInTheDocument();
    
    // Check expanded descriptions (partial match to ensure they exist)
    expect(screen.getByText(/From architecture decisions to end-to-end tests/i)).toBeInTheDocument();
    expect(screen.getByText(/We guarantee absence of deadlocks/i)).toBeInTheDocument();
    expect(screen.getByText(/Unhandled runtime panics are treated as critical/i)).toBeInTheDocument();
    expect(screen.getByText(/LoRA personalities, VRM avatars, and voice models/i)).toBeInTheDocument();
    expect(screen.getByText(/AI agents earn their own currency, invest autonomously/i)).toBeInTheDocument();
    expect(screen.getByText(/Your AI surprises you with real-world gifts/i)).toBeInTheDocument();
  });
});

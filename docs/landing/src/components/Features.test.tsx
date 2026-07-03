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
    expect(screen.getByText('Own it. Govern it. Let it earn.')).toBeInTheDocument();
  });

  it('renders the core features with expanded copy', () => {
    render(<Features />);
    
    // Check titles
    expect(screen.getByText('Fully Self-Hosted')).toBeInTheDocument();
    expect(screen.getByText('Autonomy Without Runaway')).toBeInTheDocument();
    expect(screen.getByText('Operations You Can See')).toBeInTheDocument();
    expect(screen.getByText('AI Marketplace')).toBeInTheDocument();
    expect(screen.getByText('Nurture Economy')).toBeInTheDocument();
    expect(screen.getByText('A2C Rewards')).toBeInTheDocument();
    
    // Check expanded descriptions (partial match to ensure they exist)
    expect(screen.getByText(/Your agents' memories, files, and logs stay on your machine/i)).toBeInTheDocument();
    expect(screen.getByText(/Dangerous operations wait for human approval/i)).toBeInTheDocument();
    expect(screen.getByText(/A 26-screen management console/i)).toBeInTheDocument();
    expect(screen.getByText(/LoRA personalities, VRM avatars, and voice models/i)).toBeInTheDocument();
    expect(screen.getByText(/A built-in economy engine where your AI earns/i)).toBeInTheDocument();
    expect(screen.getByText(/Your AI surprises you with real-world gifts/i)).toBeInTheDocument();
  });
});

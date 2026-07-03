/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import { render, screen } from '@testing-library/react';
import { SocialProof } from './SocialProof';
import '../i18n/config'; // Setup i18n

describe('SocialProof Component', () => {
  it('renders the three metric values', () => {
    render(<SocialProof />);
    
    // Check metric values
    expect(screen.getByText('5 min')).toBeInTheDocument();
    expect(screen.getByText('5')).toBeInTheDocument();
    expect(screen.getByText('$0')).toBeInTheDocument();
  });

  it('renders the metric labels', () => {
    render(<SocialProof />);
    
    // Check labels
    expect(screen.getByText('to full setup')).toBeInTheDocument();
    expect(screen.getByText('TLA+ specs, model-checked')).toBeInTheDocument();
    expect(screen.getByText('per month, self-hosted')).toBeInTheDocument();
  });
});

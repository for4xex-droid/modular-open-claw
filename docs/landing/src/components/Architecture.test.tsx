/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import { render, screen } from '@testing-library/react';
import { Architecture } from './Architecture';
import '../i18n/config';

describe('Architecture Component', () => {
  it('renders section title', () => {
    render(<Architecture />);
    expect(screen.getByText('Engineered for absolute safety.')).toBeInTheDocument();
  });

  it('renders the three security layers', () => {
    render(<Architecture />);
    expect(screen.getByText('Trust Layer')).toBeInTheDocument();
    expect(screen.getByText('Cell Isolation')).toBeInTheDocument();
    expect(screen.getByText('Adaptive Immunity')).toBeInTheDocument();
  });
});

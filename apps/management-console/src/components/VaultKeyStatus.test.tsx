/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

import React from 'react';
import { render, screen } from '@testing-library/react';
import '@testing-library/jest-dom';
import { VaultKeyStatus } from './VaultKeyStatus';

// mock useTranslation
jest.mock('../i18n', () => ({
  useTranslation: () => ({
    t: (key: string) => key,
  }),
}));

describe('VaultKeyStatus Component', () => {
  it('renders correctly as configured', () => {
    render(<VaultKeyStatus isSet={true} />);
    expect(screen.getByText('vault.indicator.managed')).toBeInTheDocument();
    expect(screen.getByText('vault.status.set')).toBeInTheDocument();
  });

  it('renders correctly as not set', () => {
    render(<VaultKeyStatus isSet={false} />);
    expect(screen.getByText('vault.indicator.managed')).toBeInTheDocument();
    expect(screen.getByText('vault.status.notSet')).toBeInTheDocument();
  });
});

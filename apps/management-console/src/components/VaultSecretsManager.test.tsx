/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

import React from 'react';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import '@testing-library/jest-dom';
import { VaultSecretsManager } from './VaultSecretsManager';
import { authenticatedFetch } from '../lib/auth';

jest.mock('../lib/auth', () => ({
  authenticatedFetch: jest.fn(),
}));

jest.mock('../config', () => ({
  API_BASE: 'http://localhost:3015',
}));

// mock useTranslation
jest.mock('../i18n', () => ({
  useTranslation: () => ({
    t: (key: string, params?: any) => {
      if (key === 'vault.modal.title' && params) {
        return `Configure Secret: ${params.key}`;
      }
      return key;
    },
  }),
}));

jest.mock('./common/Toast', () => ({
  useToast: () => ({ showToast: jest.fn() }),
}));

jest.mock('./common/ConfirmModal', () => ({
  __esModule: true,
  default: ({ isOpen, onConfirm, onCancel, confirmText }: any) =>
    isOpen ? (
      <div data-testid="confirm-modal">
        <button onClick={onConfirm}>{confirmText || 'Confirm'}</button>
        <button onClick={onCancel}>Cancel</button>
      </div>
    ) : null,
}));

jest.mock('./ui/LoadingState', () => ({
  LoadingState: () => <div data-testid="loading-state">loading</div>,
}));

describe('VaultSecretsManager Component', () => {
  beforeEach(() => {
    jest.clearAllMocks();
  });

  it('renders loading state first, then lists secrets', async () => {
    (authenticatedFetch as jest.Mock).mockResolvedValueOnce({
      ok: true,
      json: async () => ({
        secrets: [
          { key: 'GEMINI_API_KEY', category: 'ai', is_set: true },
          { key: 'STRIPE_API_KEY', category: 'commerce', is_set: false },
        ],
        total: 2,
        configured: 1,
      }),
    });

    render(<VaultSecretsManager />);

    // Wait for data load
    await waitFor(() => {
      expect(screen.getByText('GEMINI_API_KEY')).toBeInTheDocument();
    });

    expect(screen.getByText('STRIPE_API_KEY')).toBeInTheDocument();
    expect(screen.getByText('vault.status.set')).toBeInTheDocument(); // set badge
    expect(screen.getByText('vault.status.notSet')).toBeInTheDocument(); // notSet badge
  });

  it('displays admin privilege required message on 403 Forbidden', async () => {
    (authenticatedFetch as jest.Mock).mockResolvedValueOnce({
      ok: false,
      status: 403,
    });

    render(<VaultSecretsManager />);

    await waitFor(() => {
      expect(screen.getByText('vault.permissionRequired')).toBeInTheDocument();
    });
  });

  it('opens configure modal on edit click and calls PUT on save', async () => {
    (authenticatedFetch as jest.Mock)
      .mockResolvedValueOnce({
        ok: true,
        json: async () => ({
          secrets: [{ key: 'GEMINI_API_KEY', category: 'ai', is_set: false }],
          total: 1,
          configured: 0,
        }),
      })
      .mockResolvedValueOnce({
        ok: true, // PUT response
      });

    render(<VaultSecretsManager />);

    await waitFor(() => {
      expect(screen.getByText('GEMINI_API_KEY')).toBeInTheDocument();
    });

    // Click Configure button
    const configBtn = screen.getByText('vault.status.configure');
    fireEvent.click(configBtn);

    // Modal should be open
    expect(screen.getByText('Configure Secret: GEMINI_API_KEY')).toBeInTheDocument();

    const input = screen.getByPlaceholderText('vault.modal.placeholder');
    fireEvent.change(input, { target: { value: 'my-new-secret-key-123' } });

    const saveBtn = screen.getByText('vault.modal.save');
    fireEvent.click(saveBtn);

    await waitFor(() => {
      expect(authenticatedFetch).toHaveBeenCalledWith(
        expect.stringContaining('/api/v1/vault/secrets'),
        expect.objectContaining({
          method: 'PUT',
          body: JSON.stringify({
            key: 'GEMINI_API_KEY',
            value: 'my-new-secret-key-123',
          }),
        })
      );
    });
  });
});

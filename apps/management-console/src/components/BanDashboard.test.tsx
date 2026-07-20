/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import { render, screen, fireEvent, waitFor, within } from '@testing-library/react';
import BanDashboard from './BanDashboard';
import { authenticatedFetch } from '../lib/auth';
import { useToast } from './common/Toast';

const mockShowToast = jest.fn();

jest.mock('../lib/auth', () => ({
  authenticatedFetch: jest.fn()
}));

jest.mock('./common/Toast', () => ({
  useToast: jest.fn()
}));

jest.mock('./common/ConfirmModal', () => ({
  __esModule: true,
  default: ({ isOpen, onConfirm, onCancel, confirmText, cancelText }: any) =>
    isOpen ? (
      <div data-testid="confirm-modal">
        <button onClick={onConfirm}>{confirmText || 'Confirm'}</button>
        <button onClick={onCancel}>{cancelText || 'common.cancel'}</button>
      </div>
    ) : null,
}));

jest.mock('../config', () => ({
  API_BASE: 'http://localhost:3015'
}));

jest.mock('../i18n', () => ({
  useTranslation: () => ({
    t: (key: string) => key
  })
}));

jest.mock('framer-motion', () => ({
  motion: {
    div: ({ children, ...props }: any) => <div {...props}>{children}</div>,
  },
  AnimatePresence: ({ children }: any) => <>{children}</>,
}));

const mockFetch = authenticatedFetch as jest.MockedFunction<typeof authenticatedFetch>;

function mockBansListOk() {
  mockFetch.mockImplementation((url: string) => {
    if (url.includes('/api/v1/admin/bans')) {
      return Promise.resolve({
        ok: true,
        json: () => Promise.resolve([
          {
            actor_id: 'agent-1111-uuid',
            reason: 'Malicious CSAM attempt',
            severity: 'CRITICAL',
            banned_by: 'system',
            banned_at: '2026-05-29T00:00:00Z',
            unbanned_at: null
          },
          {
            actor_id: 'agent-2222-uuid',
            reason: 'Spam activity',
            severity: 'MEDIUM',
            banned_by: 'admin',
            banned_at: '2026-05-28T00:00:00Z',
            unbanned_at: '2026-05-28T12:00:00Z'
          }
        ])
      } as Response);
    }
    if (url.includes('/api/v1/admin/ban')) {
      return Promise.resolve({ ok: true, json: () => Promise.resolve({ success: true }) } as Response);
    }
    if (url.includes('/api/v1/admin/unban')) {
      return Promise.resolve({ ok: true, json: () => Promise.resolve({ success: true }) } as Response);
    }
    return Promise.resolve({ ok: false } as Response);
  });
}

describe('BanDashboard Governance Integration', () => {
  beforeEach(() => {
    jest.clearAllMocks();
    (useToast as jest.Mock).mockReturnValue({ showToast: mockShowToast });
    mockBansListOk();
  });

  it('fetches and renders active and historical bans on mount', async () => {
    render(<BanDashboard />);

    expect(screen.getByText('ban.title')).toBeInTheDocument();

    const activeBan = await screen.findByText('agent-1111-uuid');
    expect(activeBan).toBeInTheDocument();
    expect(screen.getByText('Malicious CSAM attempt')).toBeInTheDocument();

    const inactiveBan = await screen.findByText('agent-2222-uuid');
    expect(inactiveBan).toBeInTheDocument();
    expect(screen.getByText('Spam activity')).toBeInTheDocument();
  });

  it('filters ban records based on search query input', async () => {
    render(<BanDashboard />);

    await screen.findByText('agent-1111-uuid');

    const searchInput = screen.getByPlaceholderText('ban.searchPlaceholder');
    fireEvent.change(searchInput, { target: { value: 'Spam' } });

    expect(screen.getByText('agent-2222-uuid')).toBeInTheDocument();
    expect(screen.queryByText('agent-1111-uuid')).not.toBeInTheDocument();
  });

  it('submits a new ban suspension request successfully', async () => {
    render(<BanDashboard />);

    const uuidInput = screen.getByPlaceholderText('00000000-0000-0000-0000-000000000000');
    fireEvent.change(uuidInput, { target: { value: 'new-agent-uuid' } });

    const reasonInput = screen.getByPlaceholderText('ban.reasonPlaceholder');
    fireEvent.change(reasonInput, { target: { value: 'Excessive spamming' } });

    const severitySelect = screen.getByRole('combobox');
    fireEvent.change(severitySelect, { target: { value: 'HIGH' } });

    const submitBtn = screen.getByRole('button', { name: 'ban.submit' });
    fireEvent.click(submitBtn);

    await waitFor(() => {
      expect(mockFetch).toHaveBeenCalledWith(
        'http://localhost:3015/api/v1/admin/ban',
        expect.objectContaining({
          method: 'POST',
          body: JSON.stringify({
            agent_id: 'new-agent-uuid',
            reason: 'Excessive spamming',
            severity: 'HIGH'
          })
        })
      );
    });
  });

  it('sends English DEFAULT_BAN_REASON when reason is empty (locale-independent API body)', async () => {
    render(<BanDashboard />);

    const uuidInput = await screen.findByPlaceholderText('00000000-0000-0000-0000-000000000000');
    fireEvent.change(uuidInput, { target: { value: 'empty-reason-agent' } });

    fireEvent.click(screen.getByRole('button', { name: 'ban.submit' }));

    await waitFor(() => {
      expect(mockFetch).toHaveBeenCalledWith(
        'http://localhost:3015/api/v1/admin/ban',
        expect.objectContaining({
          method: 'POST',
          body: JSON.stringify({
            agent_id: 'empty-reason-agent',
            reason: 'Policy violation',
            severity: 'HIGH'
          })
        })
      );
    });
  });

  it('toasts ban.errorFetch when list endpoint returns non-ok', async () => {
    mockFetch.mockImplementation((url: string) => {
      if (url.includes('/api/v1/admin/bans')) {
        return Promise.resolve({ ok: false, status: 403 } as Response);
      }
      return Promise.resolve({ ok: false } as Response);
    });

    render(<BanDashboard />);

    await waitFor(() => {
      expect(mockShowToast).toHaveBeenCalledWith('error', 'ban.errorFetch');
    });
  });

  it('toasts ban.errorBan when ban error body is non-JSON (not networkError)', async () => {
    mockFetch.mockImplementation((url: string) => {
      if (url.includes('/api/v1/admin/bans')) {
        return Promise.resolve({
          ok: true,
          json: () => Promise.resolve([])
        } as Response);
      }
      if (url.includes('/api/v1/admin/ban')) {
        return Promise.resolve({
          ok: false,
          json: () => Promise.reject(new Error('not json'))
        } as Response);
      }
      return Promise.resolve({ ok: false } as Response);
    });

    render(<BanDashboard />);

    const uuidInput = await screen.findByPlaceholderText('00000000-0000-0000-0000-000000000000');
    fireEvent.change(uuidInput, { target: { value: 'agent-x' } });
    fireEvent.click(screen.getByRole('button', { name: 'ban.submit' }));

    await waitFor(() => {
      expect(mockShowToast).toHaveBeenCalledWith('error', 'ban.errorBan');
    });
    expect(mockShowToast).not.toHaveBeenCalledWith('error', 'common.networkError');
  });

  it('calls unban endpoint when unban button is clicked and confirm is accepted', async () => {
    render(<BanDashboard />);

    const unbanBtn = await screen.findByRole('button', { name: 'ban.unban' });
    fireEvent.click(unbanBtn);

    await screen.findByTestId('confirm-modal');
    fireEvent.click(within(screen.getByTestId('confirm-modal')).getByRole('button', { name: 'ban.unban' }));

    await waitFor(() => {
      expect(mockFetch).toHaveBeenCalledWith(
        'http://localhost:3015/api/v1/admin/unban',
        expect.objectContaining({
          method: 'POST',
          body: JSON.stringify({
            agent_id: 'agent-1111-uuid'
          })
        })
      );
    });
  });
});

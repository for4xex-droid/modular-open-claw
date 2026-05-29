import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import BanDashboard from './BanDashboard';
import { authenticatedFetch } from '../lib/auth';

// Mock dependencies
jest.mock('../lib/auth', () => ({
  authenticatedFetch: jest.fn().mockImplementation((url: string, options?: any) => {
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
      });
    }
    if (url.includes('/api/v1/admin/ban')) {
      return Promise.resolve({ ok: true, json: () => Promise.resolve({ success: true }) });
    }
    if (url.includes('/api/v1/admin/unban')) {
      return Promise.resolve({ ok: true, json: () => Promise.resolve({ success: true }) });
    }
    return Promise.resolve({ ok: false });
  })
}));

jest.mock('./common/Toast', () => ({
  useToast: () => ({ showToast: jest.fn() })
}));

jest.mock('../config', () => ({
  API_BASE: 'http://localhost:3015'
}));

jest.mock('framer-motion', () => ({
  motion: {
    div: ({ children, ...props }: any) => <div {...props}>{children}</div>,
  },
  AnimatePresence: ({ children }: any) => <>{children}</>,
}));

// Mock confirm dialog
const originalConfirm = window.confirm;

describe('BanDashboard Governance Integration', () => {
  beforeEach(() => {
    jest.clearAllMocks();
    window.confirm = () => true;
  });

  afterAll(() => {
    window.confirm = originalConfirm;
  });

  it('fetches and renders active and historical bans on mount', async () => {
    render(<BanDashboard />);

    // Check header
    expect(screen.getByText('Governance & BAN Compliance Registry')).toBeInTheDocument();

    // Check rendering of ban records
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

    const searchInput = screen.getByPlaceholderText('Search by UUID or reason...');
    fireEvent.change(searchInput, { target: { value: 'Spam' } });

    // "Spam activity" should remain, "CSAM" should be filtered out
    expect(screen.getByText('agent-2222-uuid')).toBeInTheDocument();
    expect(screen.queryByText('agent-1111-uuid')).not.toBeInTheDocument();
  });

  it('submits a new ban suspension request successfully', async () => {
    render(<BanDashboard />);

    // Fill form
    const uuidInput = screen.getByPlaceholderText('00000000-0000-0000-0000-000000000000');
    fireEvent.change(uuidInput, { target: { value: 'new-agent-uuid' } });

    const reasonInput = screen.getByPlaceholderText(/Describe policy violation/i);
    fireEvent.change(reasonInput, { target: { value: 'Excessive spamming' } });

    const severitySelect = screen.getByRole('combobox');
    fireEvent.change(severitySelect, { target: { value: 'HIGH' } });

    const submitBtn = screen.getByRole('button', { name: /Enforce Suspension/i });
    fireEvent.click(submitBtn);

    await waitFor(() => {
      expect(authenticatedFetch).toHaveBeenCalledWith(
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

  it('calls unban endpoint when unban button is clicked and confirm is accepted', async () => {
    render(<BanDashboard />);

    const unbanBtn = await screen.findByRole('button', { name: 'Unban' });
    fireEvent.click(unbanBtn);

    await waitFor(() => {
      expect(authenticatedFetch).toHaveBeenCalledWith(
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

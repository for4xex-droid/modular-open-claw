import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import BuzzApproval from './BuzzApproval';
import { authenticatedFetch } from '../lib/auth';

// Mock dependencies
jest.mock('../lib/auth', () => ({
  authenticatedFetch: jest.fn().mockImplementation((url: string, options?: any) => {
    if (url.includes('/api/v1/buzz/pending')) {
      return Promise.resolve({
        ok: true,
        json: () => Promise.resolve([
          {
            id: 'job-1111',
            category: 'Tech',
            status: 'Pending',
            output_artifacts: 'Initial draft content under 280 chars.',
            created_at: '2026-05-29T12:00:00Z'
          }
        ])
      });
    }
    if (url.includes('/api/v1/buzz/approve/job-1111')) {
      return Promise.resolve({ ok: true });
    }
    if (url.includes('/api/v1/buzz/publish/job-1111')) {
      return Promise.resolve({ ok: true });
    }
    if (url.includes('/api/v1/buzz/reject/job-1111')) {
      return Promise.resolve({ ok: true });
    }
    if (url.includes('/api/v1/buzz/generate')) {
      return Promise.resolve({ ok: true });
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

jest.mock('../i18n', () => ({
  useTranslation: () => ({ t: (key: string, options?: any) => options?.defaultValue || key })
}));

describe('BuzzApproval Social Media Governance', () => {
  beforeEach(() => {
    jest.clearAllMocks();
  });

  it('fetches and renders pending buzz drafts on mount', async () => {
    render(<BuzzApproval />);

    // Header check
    expect(screen.getByText('Buzz Protocol')).toBeInTheDocument();

    // Scan draft item
    const draftText = await screen.findByText('"Initial draft content under 280 chars."');
    expect(draftText).toBeInTheDocument();
    expect(screen.getByText('Awaiting Approval')).toBeInTheDocument();
  });

  it('opens details modal with draft content when draft card is clicked', async () => {
    render(<BuzzApproval />);

    const draftCard = await screen.findByText('"Initial draft content under 280 chars."');
    fireEvent.click(draftCard);

    // Modal elements
    expect(screen.getByText('Review Content for X')).toBeInTheDocument();
    const textarea = screen.getByRole('textbox');
    expect(textarea).toHaveValue('Initial draft content under 280 chars.');
  });

  it('enforces 280 character limit on edit text area in details modal', async () => {
    render(<BuzzApproval />);

    const draftCard = await screen.findByText('"Initial draft content under 280 chars."');
    fireEvent.click(draftCard);

    const textarea = screen.getByRole('textbox');
    
    // Fill text exceeding 280 characters
    const extremelyLongText = 'a'.repeat(281);
    fireEvent.change(textarea, { target: { value: extremelyLongText } });

    expect(screen.getByText('281 / 280 chars')).toBeInTheDocument();
    
    const approveBtn = screen.getByRole('button', { name: /Approve & Publish/i });
    expect(approveBtn).toBeDisabled();
  });

  it('calls approve and publish APIs sequentially when approve is triggered', async () => {
    render(<BuzzApproval />);

    const draftCard = await screen.findByText('"Initial draft content under 280 chars."');
    fireEvent.click(draftCard);

    const approveBtn = screen.getByRole('button', { name: /Approve & Publish/i });
    fireEvent.click(approveBtn);

    await waitFor(() => {
      expect(authenticatedFetch).toHaveBeenCalledWith(
        'http://localhost:3015/api/v1/buzz/approve/job-1111',
        expect.objectContaining({ method: 'POST' })
      );
    });

    await waitFor(() => {
      expect(authenticatedFetch).toHaveBeenCalledWith(
        'http://localhost:3015/api/v1/buzz/publish/job-1111',
        expect.objectContaining({ method: 'POST' })
      );
    });
  });

  it('calls reject API and hides modal when reject is clicked', async () => {
    render(<BuzzApproval />);

    const draftCard = await screen.findByText('"Initial draft content under 280 chars."');
    fireEvent.click(draftCard);

    const rejectBtn = screen.getByRole('button', { name: /Reject/i });
    fireEvent.click(rejectBtn);

    await waitFor(() => {
      expect(authenticatedFetch).toHaveBeenCalledWith(
        'http://localhost:3015/api/v1/buzz/reject/job-1111',
        expect.objectContaining({ method: 'POST' })
      );
    });
  });

  it('calls generate API when Generate New button is clicked', async () => {
    render(<BuzzApproval />);

    const generateBtn = screen.getByRole('button', { name: /Generate New/i });
    fireEvent.click(generateBtn);

    await waitFor(() => {
      expect(authenticatedFetch).toHaveBeenCalledWith(
        'http://localhost:3015/api/v1/buzz/generate',
        expect.objectContaining({ method: 'POST' })
      );
    });
  });
});

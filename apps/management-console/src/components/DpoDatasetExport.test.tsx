
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import '@testing-library/jest-dom';
import DpoDatasetExport from './DpoDatasetExport';
import { authenticatedFetch } from '../lib/auth';

// Mock the authenticatedFetch
jest.mock('../lib/auth', () => ({
  authenticatedFetch: jest.fn(),
}));

jest.mock('../config', () => ({
  API_BASE: 'http://localhost:3000'
}));

// Mock i18n: t() returns the key, which enables the fallback `||` operator in the component
jest.mock('../i18n', () => ({
  useTranslation: () => ({
    t: (_key: string) => '' // Return empty string so the fallback english text is used
  })
}));

// Mock window.URL
const mockCreateObjectURL = jest.fn();
const mockRevokeObjectURL = jest.fn();
window.URL.createObjectURL = mockCreateObjectURL;
window.URL.revokeObjectURL = mockRevokeObjectURL;

describe('DpoDatasetExport', () => {
  beforeEach(() => {
    jest.clearAllMocks();
    mockCreateObjectURL.mockReturnValue('blob:mock-url');
  });

  it('renders export button with fallback text', () => {
    render(<DpoDatasetExport />);
    expect(screen.getByRole('button', { name: /Download Dataset \(JSONL\)/i })).toBeInTheDocument();
  });

  it('renders title and description', () => {
    render(<DpoDatasetExport />);
    expect(screen.getByText('DPO Dataset Export')).toBeInTheDocument();
    expect(screen.getByText(/Arena matches/i)).toBeInTheDocument();
  });

  it('handles successful dataset download', async () => {
    const mockBlob = new Blob(['{"test": "data"}'], { type: 'application/x-ndjson' });
    (authenticatedFetch as jest.Mock).mockResolvedValueOnce({
      ok: true,
      blob: async () => mockBlob,
    });

    const mockClick = jest.fn();
    const mockLink = {
      href: '',
      download: '',
      click: mockClick,
      remove: jest.fn(),
    } as unknown as HTMLAnchorElement;

    const originalCreateElement = document.createElement.bind(document);
    jest.spyOn(document, 'createElement').mockImplementation((tagName: string) => {
      if (tagName === 'a') return mockLink;
      return originalCreateElement(tagName);
    });

    const originalAppendChild = document.body.appendChild.bind(document.body);
    jest.spyOn(document.body, 'appendChild').mockImplementation((node: Node) => {
      if (node === mockLink as unknown as Node) return mockLink;
      return originalAppendChild(node);
    });

    render(<DpoDatasetExport />);
    const button = screen.getByRole('button', { name: /Download Dataset \(JSONL\)/i });

    fireEvent.click(button);

    // Verify loading state
    expect(button).toBeDisabled();

    await waitFor(() => {
      expect(authenticatedFetch).toHaveBeenCalledWith('http://localhost:3000/api/v1/cortex/dpo/dataset');
      expect(mockCreateObjectURL).toHaveBeenCalledWith(mockBlob);
      expect(mockLink.download).toBe('dpo_dataset.jsonl');
      expect(mockClick).toHaveBeenCalled();
    });

    // Verify recovery of button state
    expect(button).not.toBeDisabled();
  });

  it('handles API error during download', async () => {
    (authenticatedFetch as jest.Mock).mockResolvedValueOnce({
      ok: false,
      status: 403,
      text: async () => 'Forbidden',
    });

    render(<DpoDatasetExport />);
    const button = screen.getByRole('button', { name: /Download Dataset \(JSONL\)/i });

    fireEvent.click(button);

    await waitFor(() => {
      expect(screen.getByText(/Failed to export dataset/i)).toBeInTheDocument();
    });
  });

  it('handles network failure during download', async () => {
    (authenticatedFetch as jest.Mock).mockRejectedValueOnce(new Error('Network error'));

    render(<DpoDatasetExport />);
    const button = screen.getByRole('button', { name: /Download Dataset \(JSONL\)/i });

    fireEvent.click(button);

    await waitFor(() => {
      expect(screen.getByText(/Network error/i)).toBeInTheDocument();
    });

    // Verify button recovers from error state
    expect(button).not.toBeDisabled();
  });

  it('sets aria-busy during loading', async () => {
    (authenticatedFetch as jest.Mock).mockImplementation(
      () => new Promise(resolve => setTimeout(resolve, 100))
    );

    render(<DpoDatasetExport />);
    const button = screen.getByRole('button', { name: /Download Dataset \(JSONL\)/i });

    expect(button).toHaveAttribute('aria-busy', 'false');

    fireEvent.click(button);

    expect(button).toHaveAttribute('aria-busy', 'true');
  });
});

/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import SetupWizard from './SetupWizard';

jest.mock('../i18n', () => ({
  useTranslation: () => ({
    t: (key: string) => null, // return null to exercise fallback strings
    i18n: { changeLanguage: jest.fn() }
  })
}));

jest.mock('../config', () => ({
  API_BASE: 'http://localhost:3015'
}));

jest.mock('../lib/auth', () => ({
  setAuthToken: jest.fn(),
  authenticatedFetch: jest.fn()
}));

import { authenticatedFetch } from '../lib/auth';

global.fetch = jest.fn();

// window.location.reload cannot be mocked in jsdom; the component uses reloadApp instead
jest.mock('../lib/navigation', () => ({
  reloadApp: jest.fn()
}));

import { reloadApp } from '../lib/navigation';
const reloadMock = reloadApp as jest.Mock;

const SAMPLE_PLAYBOOKS = [
  { id: 'seo-operations', name: 'SEO Operations', description: 'SEO workflows', tags: ['seo'], workflow_count: 1, required_skills: [], required_mcp_servers: [] },
  { id: 'sns-operations', name: 'SNS Operations', description: 'SNS workflows', tags: ['sns'], workflow_count: 1, required_skills: [], required_mcp_servers: [] }
];

/** Drive the wizard from intro to a submitted setup/init request */
async function completeWizardUntilInit() {
  fireEvent.click(screen.getByRole('button', { name: /Start Setup/i }));
  await waitFor(() => screen.getByRole('heading', { name: /Terms of Service/i }));
  fireEvent.click(screen.getByLabelText(/I agree to/i));
  fireEvent.click(screen.getByRole('button', { name: /Next/i }));
  await waitFor(() => screen.getByText(/Name your AI/i));
  fireEvent.click(screen.getByRole('button', { name: /Next/i }));
  await waitFor(() => screen.getByText(/Choose your experience/i));
  fireEvent.click(screen.getByText(/Expert Mode/i));
  await waitFor(() => screen.getByText(/Create Admin/i));
  fireEvent.change(screen.getByLabelText(/Email/i), { target: { value: 'admin@example.com' } });
  fireEvent.change(screen.getByLabelText(/^Password/i), { target: { value: 'SecurePass1234!' } });
  fireEvent.change(screen.getByLabelText(/Confirm Password/i), { target: { value: 'SecurePass1234!' } });
  fireEvent.click(screen.getByRole('button', { name: /Initialize System/i }));
}

describe('SetupWizard Component', () => {
  beforeEach(() => {
    jest.clearAllMocks();
    (authenticatedFetch as jest.Mock).mockResolvedValue({
      ok: true,
      status: 200,
      json: async () => []
    });
  });

  it('should complete the full wizard flow end-to-end', async () => {
    (global.fetch as jest.Mock).mockResolvedValue({
      ok: true,
      json: async () => ({ access_token: "test_token" })
    });

    const onCompleteMock = jest.fn();
    render(<SetupWizard onComplete={onCompleteMock} />);

    // Step 0: Intro
    expect(screen.getByText(/Welcome to Aiome/i)).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: /Start Setup/i }));

    // Step 1: TOS
    await waitFor(() => expect(screen.getByRole('heading', { name: /Terms of Service/i })).toBeInTheDocument());
    const nextBtn = screen.getByRole('button', { name: /Next/i });
    expect(nextBtn).toBeDisabled(); // Cannot proceed without accepting TOS
    fireEvent.click(screen.getByLabelText(/I agree to/i));
    expect(nextBtn).not.toBeDisabled();
    fireEvent.click(nextBtn);

    // Step 2: AI Name
    await waitFor(() => expect(screen.getByText(/Name your AI/i)).toBeInTheDocument());
    fireEvent.change(screen.getByPlaceholderText(/e.g. Watchtower/i), { target: { value: 'MyTestAI' } });
    fireEvent.click(screen.getByRole('button', { name: /Next/i }));

    // Step 3: View Mode
    await waitFor(() => expect(screen.getByText(/Choose your experience/i)).toBeInTheDocument());
    fireEvent.click(screen.getByText(/Expert Mode/i));

    // Step 4: Admin Credentials
    await waitFor(() => expect(screen.getByText(/Create Admin/i)).toBeInTheDocument());
    fireEvent.change(screen.getByLabelText(/Email/i), { target: { value: 'admin@example.com' } });
    fireEvent.change(screen.getByLabelText(/^Password/i), { target: { value: 'SecurePass1234!' } });
    fireEvent.change(screen.getByLabelText(/Confirm Password/i), { target: { value: 'SecurePass1234!' } });
    fireEvent.click(screen.getByRole('button', { name: /Initialize System/i }));

    await waitFor(() => {
      expect(global.fetch).toHaveBeenCalledWith(
        'http://localhost:3015/api/v1/setup/init',
        expect.objectContaining({ method: 'POST' })
      );
      expect(onCompleteMock).toHaveBeenCalled();
    });

    // Step 6: Playbook selection appears after successful init
    await waitFor(() => expect(screen.getByText(/Choose a Playbook/i)).toBeInTheDocument());

    // Skip triggers the reload that used to happen right after init
    fireEvent.click(screen.getByRole('button', { name: /Skip/i }));
    expect(reloadMock).toHaveBeenCalled();
  });

  it('should list playbooks after successful setup and install selected one', async () => {
    (global.fetch as jest.Mock).mockResolvedValue({
      ok: true,
      json: async () => ({ access_token: "test_token" })
    });
    (authenticatedFetch as jest.Mock).mockImplementation(async (url: string, init?: RequestInit) => {
      if (url.endsWith('/api/v1/playbooks')) {
        return { ok: true, status: 200, json: async () => SAMPLE_PLAYBOOKS };
      }
      if (url.includes('/install') && init?.method === 'POST') {
        return { ok: true, status: 200, json: async () => ({ playbook_id: 'seo-operations', created_workflow_ids: ['x'] }) };
      }
      throw new Error(`Unexpected request: ${url}`);
    });

    render(<SetupWizard onComplete={jest.fn()} />);
    await completeWizardUntilInit();

    await waitFor(() => expect(screen.getByText(/SEO Operations/i)).toBeInTheDocument());
    expect(screen.getByText(/SNS Operations/i)).toBeInTheDocument();

    const installButtons = screen.getAllByRole('button', { name: /^Install$/i });
    fireEvent.click(installButtons[0]);

    await waitFor(() => expect(screen.getByText(/Installed/i)).toBeInTheDocument());
    expect(authenticatedFetch).toHaveBeenCalledWith(
      'http://localhost:3015/api/v1/playbooks/seo-operations/install',
      expect.objectContaining({ method: 'POST' })
    );
  });

  it('should allow skipping playbook selection', async () => {
    (global.fetch as jest.Mock).mockResolvedValue({
      ok: true,
      json: async () => ({ access_token: "test_token" })
    });

    render(<SetupWizard onComplete={jest.fn()} />);
    await completeWizardUntilInit();

    await waitFor(() => expect(screen.getByText(/Choose a Playbook/i)).toBeInTheDocument());
    fireEvent.click(screen.getByRole('button', { name: /Skip/i }));
    expect(reloadMock).toHaveBeenCalled();
  });

  it('should surface missing dependencies from 422 response', async () => {
    (global.fetch as jest.Mock).mockResolvedValue({
      ok: true,
      json: async () => ({ access_token: "test_token" })
    });
    (authenticatedFetch as jest.Mock).mockImplementation(async (url: string, init?: RequestInit) => {
      if (url.endsWith('/api/v1/playbooks')) {
        return { ok: true, status: 200, json: async () => SAMPLE_PLAYBOOKS };
      }
      if (url.includes('/install') && init?.method === 'POST') {
        return { ok: false, status: 422, json: async () => ({ missing_skills: ['keyword-analyzer'], missing_mcp_servers: [] }) };
      }
      throw new Error(`Unexpected request: ${url}`);
    });

    render(<SetupWizard onComplete={jest.fn()} />);
    await completeWizardUntilInit();

    await waitFor(() => expect(screen.getByText(/SEO Operations/i)).toBeInTheDocument());
    fireEvent.click(screen.getAllByRole('button', { name: /^Install$/i })[0]);

    await waitFor(() => expect(screen.getByText(/keyword-analyzer/i)).toBeInTheDocument());

    // Reload path must remain available even after an error
    fireEvent.click(screen.getByRole('button', { name: /Start Aiome/i }));
    expect(reloadMock).toHaveBeenCalled();
  });

  it('should show inline validation for invalid email', async () => {
    render(<SetupWizard onComplete={jest.fn()} />);

    // Navigate to admin step
    fireEvent.click(screen.getByRole('button', { name: /Start Setup/i }));
    await waitFor(() => screen.getByRole('heading', { name: /Terms of Service/i }));
    fireEvent.click(screen.getByLabelText(/I agree to/i));
    fireEvent.click(screen.getByRole('button', { name: /Next/i }));
    await waitFor(() => screen.getByText(/Name your AI/i));
    fireEvent.click(screen.getByRole('button', { name: /Next/i }));
    await waitFor(() => screen.getByText(/Choose your experience/i));
    fireEvent.click(screen.getByText(/Simple Mode/i));

    // Type invalid email
    await waitFor(() => screen.getByText(/Create Admin/i));
    fireEvent.change(screen.getByLabelText(/Email/i), { target: { value: 'notanemail' } });

    expect(screen.getByText(/valid email/i)).toBeInTheDocument();
  });

  it('should show inline validation for password mismatch', async () => {
    render(<SetupWizard onComplete={jest.fn()} />);

    // Navigate to admin step
    fireEvent.click(screen.getByRole('button', { name: /Start Setup/i }));
    await waitFor(() => screen.getByRole('heading', { name: /Terms of Service/i }));
    fireEvent.click(screen.getByLabelText(/I agree to/i));
    fireEvent.click(screen.getByRole('button', { name: /Next/i }));
    await waitFor(() => screen.getByText(/Name your AI/i));
    fireEvent.click(screen.getByRole('button', { name: /Next/i }));
    await waitFor(() => screen.getByText(/Choose your experience/i));
    fireEvent.click(screen.getByText(/Simple Mode/i));

    await waitFor(() => screen.getByText(/Create Admin/i));
    fireEvent.change(screen.getByLabelText(/^Password/i), { target: { value: 'SecurePass1234!' } });
    fireEvent.change(screen.getByLabelText(/Confirm Password/i), { target: { value: 'DifferentPass!' } });

    expect(screen.getByText(/do not match/i)).toBeInTheDocument();
  });

  it('should disable submit button when email is invalid', async () => {
    render(<SetupWizard onComplete={jest.fn()} />);

    // Navigate to admin step
    fireEvent.click(screen.getByRole('button', { name: /Start Setup/i }));
    await waitFor(() => screen.getByRole('heading', { name: /Terms of Service/i }));
    fireEvent.click(screen.getByLabelText(/I agree to/i));
    fireEvent.click(screen.getByRole('button', { name: /Next/i }));
    await waitFor(() => screen.getByText(/Name your AI/i));
    fireEvent.click(screen.getByRole('button', { name: /Next/i }));
    await waitFor(() => screen.getByText(/Choose your experience/i));
    fireEvent.click(screen.getByText(/Simple Mode/i));

    await waitFor(() => screen.getByText(/Create Admin/i));
    const initBtn = screen.getByRole('button', { name: /Initialize System/i });

    // Initially disabled
    expect(initBtn).toBeDisabled();

    // Fill valid email but short password
    fireEvent.change(screen.getByLabelText(/Email/i), { target: { value: 'admin@example.com' } });
    fireEvent.change(screen.getByLabelText(/^Password/i), { target: { value: 'short' } });
    expect(initBtn).toBeDisabled();
  });

  it('should show server error message when setup API returns 400', async () => {
    (global.fetch as jest.Mock).mockResolvedValue({
      ok: false,
      json: async () => ({ message: "Setup has already been completed" })
    });

    render(<SetupWizard onComplete={jest.fn()} />);

    // Navigate to admin step
    fireEvent.click(screen.getByRole('button', { name: /Start Setup/i }));
    await waitFor(() => screen.getByRole('heading', { name: /Terms of Service/i }));
    fireEvent.click(screen.getByLabelText(/I agree to/i));
    fireEvent.click(screen.getByRole('button', { name: /Next/i }));
    await waitFor(() => screen.getByText(/Name your AI/i));
    fireEvent.click(screen.getByRole('button', { name: /Next/i }));
    await waitFor(() => screen.getByText(/Choose your experience/i));
    fireEvent.click(screen.getByText(/Standard Mode/i));

    await waitFor(() => screen.getByText(/Create Admin/i));
    fireEvent.change(screen.getByLabelText(/Email/i), { target: { value: 'admin@example.com' } });
    fireEvent.change(screen.getByLabelText(/^Password/i), { target: { value: 'SecurePass1234!' } });
    fireEvent.change(screen.getByLabelText(/Confirm Password/i), { target: { value: 'SecurePass1234!' } });
    fireEvent.click(screen.getByRole('button', { name: /Initialize System/i }));

    // Should show error and return to credentials step
    await waitFor(() => {
      expect(screen.getByRole('alert')).toBeInTheDocument();
      expect(screen.getByText(/Setup has already been completed/i)).toBeInTheDocument();
    });
  });

  it('should show network error message when fetch rejects', async () => {
    (global.fetch as jest.Mock).mockRejectedValue(new TypeError('Failed to fetch'));

    render(<SetupWizard onComplete={jest.fn()} />);

    // Navigate to admin step
    fireEvent.click(screen.getByRole('button', { name: /Start Setup/i }));
    await waitFor(() => screen.getByRole('heading', { name: /Terms of Service/i }));
    fireEvent.click(screen.getByLabelText(/I agree to/i));
    fireEvent.click(screen.getByRole('button', { name: /Next/i }));
    await waitFor(() => screen.getByText(/Name your AI/i));
    fireEvent.click(screen.getByRole('button', { name: /Next/i }));
    await waitFor(() => screen.getByText(/Choose your experience/i));
    fireEvent.click(screen.getByText(/Expert Mode/i));

    await waitFor(() => screen.getByText(/Create Admin/i));
    fireEvent.change(screen.getByLabelText(/Email/i), { target: { value: 'admin@example.com' } });
    fireEvent.change(screen.getByLabelText(/^Password/i), { target: { value: 'SecurePass1234!' } });
    fireEvent.change(screen.getByLabelText(/Confirm Password/i), { target: { value: 'SecurePass1234!' } });
    fireEvent.click(screen.getByRole('button', { name: /Initialize System/i }));

    // Should show user-friendly network error message
    await waitFor(() => {
      expect(screen.getByRole('alert')).toBeInTheDocument();
      expect(screen.getByText(/Unable to reach the server/i)).toBeInTheDocument();
    });
  });
});

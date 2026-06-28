/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import { render, screen, fireEvent } from '@testing-library/react';
import { ModelSetupStep } from './ModelSetupStep';
import { useModelStatus } from '../hooks/useModelStatus';

jest.mock('../config', () => ({
  API_BASE: 'http://localhost'
}));

// Mock the hook and i18n
jest.mock('../hooks/useModelStatus');
jest.mock('../i18n', () => ({
  useTranslation: () => ({
    t: (key: string) => key
  })
}));

describe('ModelSetupStep', () => {
  const mockPullModel = jest.fn();
  const mockCheckStatus = jest.fn();
  const mockOnNext = jest.fn();
  const mockOnSkip = jest.fn();

  beforeEach(() => {
    (useModelStatus as jest.Mock).mockReturnValue({
      status: { ollama_connected: true, setup_required: true },
      loading: false,
      error: null,
      pullProgress: null,
      isPulling: false,
      checkStatus: mockCheckStatus,
      pullModel: mockPullModel
    });
  });

  it('renders loading state', () => {
    (useModelStatus as jest.Mock).mockReturnValue({
      loading: true,
      checkStatus: mockCheckStatus
    });
    render(<ModelSetupStep onNext={mockOnNext} onSkip={mockOnSkip} />);
    expect(screen.getByText('onboarding.llmSetup.checking')).toBeInTheDocument();
  });

  it('renders connection error when Ollama is not connected', () => {
    (useModelStatus as jest.Mock).mockReturnValue({
      status: { ollama_connected: false },
      loading: false,
      checkStatus: mockCheckStatus
    });
    render(<ModelSetupStep onNext={mockOnNext} onSkip={mockOnSkip} />);
    expect(screen.getByText('onboarding.llmSetup.notConnected')).toBeInTheDocument();
  });

  it('RED: renders 3 options (Local, Cloud, Demo) when Ollama is not connected', () => {
    (useModelStatus as jest.Mock).mockReturnValue({
      status: { ollama_connected: false },
      loading: false,
      checkStatus: mockCheckStatus
    });
    render(<ModelSetupStep onNext={mockOnNext} onSkip={mockOnSkip} />);
    
    // Test for the 3 distinct options
    expect(screen.getByText('onboarding.llmSetup.optionLocal.title')).toBeInTheDocument();
    expect(screen.getByText('onboarding.llmSetup.optionCloud.title')).toBeInTheDocument();
    expect(screen.getByText('onboarding.llmSetup.optionDemo.title')).toBeInTheDocument();
  });

  it('renders model options when setup is required', () => {
    render(<ModelSetupStep onNext={mockOnNext} onSkip={mockOnSkip} />);
    expect(screen.getByText('onboarding.llmSetup.options.gemma4_26b.title')).toBeInTheDocument();
  });

  it('calls pullModel when an option is clicked', () => {
    render(<ModelSetupStep onNext={mockOnNext} onSkip={mockOnSkip} />);
    fireEvent.click(screen.getByText('onboarding.llmSetup.options.gemma4_26b.title'));
    expect(mockPullModel).toHaveBeenCalledWith('gemma4:26b');
  });

  it('renders progress when pulling', () => {
    (useModelStatus as jest.Mock).mockReturnValue({
      status: { ollama_connected: true },
      isPulling: true,
      pullProgress: { status: 'Downloading...', completed: 50, total: 100 },
      checkStatus: mockCheckStatus
    });
    render(<ModelSetupStep onNext={mockOnNext} onSkip={mockOnSkip} />);
    expect(screen.getByText('onboarding.llmSetup.downloading')).toBeInTheDocument();
    expect(screen.getByText('50.0%')).toBeInTheDocument();
  });
});

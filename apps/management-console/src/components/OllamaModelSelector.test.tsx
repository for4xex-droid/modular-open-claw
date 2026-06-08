import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { OllamaModelSelector } from './OllamaModelSelector';
import { authenticatedFetch } from '../lib/auth';

jest.mock('../config', () => ({
  API_BASE: 'http://localhost:8080'
}));

jest.mock('../lib/auth', () => ({
  authenticatedFetch: jest.fn()
}));

jest.mock('../i18n', () => ({
  useTranslation: () => ({
    t: (key: string) => key
  })
}));

describe('OllamaModelSelector', () => {
  beforeEach(() => {
    jest.clearAllMocks();
  });

  it('renders selector and refreshes models', async () => {
    const mockModels = {
      models: [{ name: 'llama3:latest' }, { name: 'mistral:latest' }]
    };

    (authenticatedFetch as jest.Mock).mockResolvedValue({
      ok: true,
      json: () => Promise.resolve(mockModels)
    });

    const onSelect = jest.fn();
    render(<OllamaModelSelector value="llama3:latest" onSelect={onSelect} />);

    expect(screen.getByText('settings.ollamaModel')).toBeInTheDocument();

    await waitFor(() => {
      expect(screen.getByText('llama3:latest')).toBeInTheDocument();
      expect(screen.getByText('mistral:latest')).toBeInTheDocument();
    });

    // Fire onchange event
    const select = screen.getByRole('combobox');
    fireEvent.change(select, { target: { value: 'mistral:latest' } });
    expect(onSelect).toHaveBeenCalledWith('mistral:latest');
  });

  it('displays connection error on failure', async () => {
    (authenticatedFetch as jest.Mock).mockRejectedValue(new Error('Network failure'));

    render(<OllamaModelSelector value="" onSelect={jest.fn()} />);

    await waitFor(() => {
      expect(screen.getByText(/Network failure/)).toBeInTheDocument();
    });
  });
});


import { render, screen, fireEvent } from '@testing-library/react';
import DemoView from './DemoView';

// Mock i18n and auth
jest.mock('../i18n', () => ({
  useTranslation: () => ({
    t: (key: string) => key
  })
}));

jest.mock('../lib/auth', () => ({
  getAuthHeaders: () => ({ 'Authorization': 'Bearer test' })
}));

describe('DemoView', () => {
  const mockStats = {
    level: 5,
    resonance: 100,
    experience: 50,
    nextLevelExp: 100
  };

  it('renders title and description', () => {
    render(<DemoView stats={mockStats} lastEvent={null} isConnected={true} />);
    expect(screen.getByText('demo.title')).toBeInTheDocument();
    expect(screen.getByText('demo.description')).toBeInTheDocument();
  });

  it('shows warning when not connected', () => {
    render(<DemoView stats={mockStats} lastEvent={null} isConnected={false} />);
    expect(screen.getByText('demo.sseHint')).toBeInTheDocument();
  });

  it('starts demo on button click', async () => {
    // Mock fetch
    global.fetch = jest.fn().mockResolvedValue({
      ok: true,
      json: () => Promise.resolve({ success: true })
    });

    render(<DemoView stats={mockStats} lastEvent={null} isConnected={true} />);
    const startBtn = screen.getByText('demo.startDemo');
    fireEvent.click(startBtn);

    expect(global.fetch).toHaveBeenCalled();
  });

  it('updates step when event arrives', () => {
    const { rerender } = render(<DemoView stats={mockStats} lastEvent={null} isConnected={true} />);
    
    const event = {
      type: 'plugin_event',
      data: {
        plugin_name: 'AutonomousDemo',
        payload: {
          step: 1,
          message: 'Test message step 1'
        }
      }
    };

    rerender(<DemoView stats={mockStats} lastEvent={event} isConnected={true} />);
    expect(screen.getByText('Test message step 1')).toBeInTheDocument();
  });
});

import { render, screen } from '@testing-library/react';
import { Economy } from './Economy';
import '../i18n/config';

describe('Economy Component', () => {
  it('renders section title and description', () => {
    render(<Economy />);
    expect(screen.getByText('The AI Economy, Explained.')).toBeInTheDocument();
  });

  it('renders the three economic model cards', () => {
    render(<Economy />);
    expect(screen.getByText('AI Goes Shopping')).toBeInTheDocument();
    expect(screen.getByText('Agents Trade Skills')).toBeInTheDocument();
    expect(screen.getByText('AI Gives Back')).toBeInTheDocument();
  });

  it('renders the mock mode note', () => {
    render(<Economy />);
    expect(screen.getByText(/Mock mode runs the full economy simulation/i)).toBeInTheDocument();
  });
});

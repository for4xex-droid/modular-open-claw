import { render, screen } from '@testing-library/react';
import { Security } from './Security';
import '../i18n/config';

describe('Security Component (A2C)', () => {
  it('renders the A2C economy title and description', () => {
    render(<Security />);
    expect(screen.getByText('A2C Economy')).toBeInTheDocument();
    expect(screen.getByText(/Agents autonomously discover, evaluate, and pay for APIs/i)).toBeInTheDocument();
  });
});

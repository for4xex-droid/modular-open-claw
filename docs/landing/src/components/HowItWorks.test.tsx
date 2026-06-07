import { render, screen } from '@testing-library/react';
import { HowItWorks } from './HowItWorks';
import '../i18n/config';

describe('HowItWorks Component', () => {
  it('renders section title', () => {
    render(<HowItWorks />);
    expect(screen.getByText('Up and running in 3 steps.')).toBeInTheDocument();
  });

  it('renders the three execution steps', () => {
    render(<HowItWorks />);
    expect(screen.getByText('Deploy')).toBeInTheDocument();
    expect(screen.getByText('Awaken')).toBeInTheDocument();
    expect(screen.getByText('Thrive')).toBeInTheDocument();
  });
});

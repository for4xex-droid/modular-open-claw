import { render, screen } from '@testing-library/react';
import { CTA } from './CTA';
import '../i18n/config';

describe('CTA Component', () => {
  it('renders the CTA title and description', () => {
    render(<CTA />);
    expect(screen.getByText('Start building with Aiome today.')).toBeInTheDocument();
    expect(screen.getByText(/Ready to step into the future/i)).toBeInTheDocument();
  });

  it('renders the deploy link with correct destination', () => {
    render(<CTA />);
    const deployLink = screen.getByRole('link', { name: /Deploy Now/i });
    expect(deployLink).toBeInTheDocument();
    expect(deployLink).toHaveAttribute('href', '#quickstart');
  });
});

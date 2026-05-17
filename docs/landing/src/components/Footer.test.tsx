import { render, screen } from '@testing-library/react';
import { Footer } from './Footer';
import '../i18n/config';

describe('Footer Component', () => {
  it('renders copyright info', () => {
    render(<Footer />);
    expect(screen.getByText(/2026 MotivationStudio LLC/i)).toBeInTheDocument();
  });

  it('renders privacy and terms links', () => {
    render(<Footer />);
    expect(screen.getByText('Privacy Policy')).toBeInTheDocument();
    expect(screen.getByText('Terms of Service')).toBeInTheDocument();
  });
});

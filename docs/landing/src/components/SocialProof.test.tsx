import { render, screen } from '@testing-library/react';
import { SocialProof } from './SocialProof';
import '../i18n/config'; // Setup i18n

describe('SocialProof Component', () => {
  it('renders the three metric values', () => {
    render(<SocialProof />);
    
    // Check metric values
    expect(screen.getByText('262,000+')).toBeInTheDocument();
    expect(screen.getByText('700+')).toBeInTheDocument();
    expect(screen.getByText('0')).toBeInTheDocument();
  });

  it('renders the metric labels', () => {
    render(<SocialProof />);
    
    // Check labels
    expect(screen.getByText('lines of production Rust')).toBeInTheDocument();
    expect(screen.getByText('E2E tests passing')).toBeInTheDocument();
    expect(screen.getByText('runtime panics in production')).toBeInTheDocument();
  });
});

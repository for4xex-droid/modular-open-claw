import { render, screen } from '@testing-library/react';
import { Features } from './Features';
import '../i18n/config';

describe('Features Component (Bento Grid)', () => {
  it('renders section title', () => {
    render(<Features />);
    expect(screen.getByText('Built for absolute resilience.')).toBeInTheDocument();
  });

  it('renders the three core features with expanded copy', () => {
    render(<Features />);
    
    // Check titles
    expect(screen.getByText('100% Autonomous')).toBeInTheDocument();
    expect(screen.getByText('Mathematically Proven')).toBeInTheDocument();
    expect(screen.getByText('Zero-Panic Rust')).toBeInTheDocument();
    
    // Check expanded descriptions (partial match to ensure they exist)
    expect(screen.getByText(/From architecture decisions to end-to-end tests/i)).toBeInTheDocument();
    expect(screen.getByText(/We guarantee absence of deadlocks/i)).toBeInTheDocument();
    expect(screen.getByText(/Unhandled runtime panics are treated as critical/i)).toBeInTheDocument();
  });
});

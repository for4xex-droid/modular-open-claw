import { render, screen } from '@testing-library/react';
import { CodePreview } from './CodePreview';
import '../i18n/config';

describe('CodePreview Component', () => {
  it('renders the title and description', () => {
    render(<CodePreview />);
    expect(screen.getByText('Deploy in seconds.')).toBeInTheDocument();
    expect(screen.getByText(/Launch a fully autonomous agent into your environment/i)).toBeInTheDocument();
  });

  it('renders the code snippet', () => {
    render(<CodePreview />);
    // The command might be split by line breaks and spans, so we look for part of it
    expect(screen.getByText(/docker-compose\.quickstart\.yml/i)).toBeInTheDocument();
  });
});

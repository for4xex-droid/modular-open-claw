import { render, screen } from '@testing-library/react';
import CortexView from './CortexView';

// Mock the translation hook to just return the key for easy assertions
jest.mock('../../i18n', () => ({
  useTranslation: () => ({
    t: (key: string) => key
  })
}));

// Mock ESM dependent libraries
jest.mock('react-markdown', () => (props: any) => <div>{props.children}</div>);
jest.mock('rehype-sanitize', () => jest.fn());

// Mock the fetch call
jest.mock('../../lib/auth', () => ({
  authenticatedFetch: jest.fn().mockImplementation(() => new Promise(() => {}))
}));

// Mock config
jest.mock('../../config', () => ({
  API_BASE: 'http://localhost'
}));

describe('CortexView i18n', () => {
  it('renders translated headings instead of hardcoded english', () => {
    render(<CortexView />);
    
    // It should render the i18n key instead of 'Knowledge Index'
    const heading = screen.getByText('cortexView.knowledgeIndex');
    expect(heading).toBeInTheDocument();
  });

  it('renders translated loading states', () => {
    render(<CortexView />);
    
    // Initial state is loading index
    const loadingIdx = screen.getByText('cortexView.scanningIndex');
    expect(loadingIdx).toBeInTheDocument();
  });
});

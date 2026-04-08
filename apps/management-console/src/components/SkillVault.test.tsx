
import { render, screen, fireEvent } from '@testing-library/react';
import SkillVault from './SkillVault';

// Mock auth and i18n
jest.mock('../lib/auth', () => ({
  authenticatedFetch: jest.fn().mockResolvedValue({
    ok: true,
    json: () => Promise.resolve([
      { name: 'WasmSkill', description: 'Test description', source: 'wasm', status: 'Active', layer: 1, tools: ['tool1'] }
    ])
  })
}));

jest.mock('../i18n', () => ({
  useTranslation: () => ({
    t: (key: string) => key
  })
}));

describe('SkillVault', () => {
  it('renders title and search input', () => {
    render(<SkillVault />);
    expect(screen.getByText('LIBRARY CATEGORIES')).toBeInTheDocument();
    expect(screen.getByPlaceholderText('skill.search')).toBeInTheDocument();
  });

  it('filters skills based on search term', async () => {
    const { findByText } = render(<SkillVault />);
    const input = screen.getByPlaceholderText('skill.search');
    fireEvent.change(input, { target: { value: 'Wasm' } });
    
    expect(await findByText('WasmSkill')).toBeInTheDocument();
  });

  it('changes filter category', () => {
    render(<SkillVault />);
    const marketplaceBtn = screen.getByText('skill.marketplace');
    fireEvent.click(marketplaceBtn);
    // Active filter logic inside component would change state
  });
});

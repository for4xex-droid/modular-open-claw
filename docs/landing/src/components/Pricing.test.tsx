import { render, screen, act } from '@testing-library/react';
import { Pricing } from './Pricing';
import i18n from '../i18n/config';

describe('Pricing Component', () => {
  beforeEach(() => {
    i18n.changeLanguage('en');
  });

  it('renders the pricing plans in English by default', () => {
    render(<Pricing />);
    
    // Header check
    expect(screen.getByText('Flexible Pricing Plans')).toBeInTheDocument();
    expect(screen.getByText('Choose the plan that fits your evolutionary speed.')).toBeInTheDocument();

    // Plan Names
    expect(screen.getByText('Sovereign Free')).toBeInTheDocument();
    expect(screen.getByText('Autonomous Pro')).toBeInTheDocument();

    // Prices & Billing
    expect(screen.getByText('Free')).toBeInTheDocument();
    expect(screen.getByText('$9.99')).toBeInTheDocument();
    expect(screen.getAllByText('/month').length).toBeGreaterThanOrEqual(1);

    // Call to Action Buttons
    expect(screen.getByRole('link', { name: /Start for Free/i })).toBeInTheDocument();
    expect(screen.getByRole('link', { name: /Upgrade to Pro/i })).toBeInTheDocument();
  });

  it('renders features for each plan', () => {
    render(<Pricing />);

    // Free features
    expect(screen.getByText('1 Active Biome (P2P)')).toBeInTheDocument();
    expect(screen.getByText('Standard Sovereign Verification')).toBeInTheDocument();
    expect(screen.getByText('Basic Karma Analytics')).toBeInTheDocument();

    // Pro features
    expect(screen.getByText('Unlimited Biomes')).toBeInTheDocument();
    expect(screen.getByText('Priority Sovereign Verification')).toBeInTheDocument();
    expect(screen.getByText('Deep Karma Trend Sonar')).toBeInTheDocument();
    expect(screen.getByText('SkillVault Developer Access')).toBeInTheDocument();
  });

  it('switches content to Japanese when language changes', async () => {
    render(<Pricing />);
    
    await act(async () => {
      await i18n.changeLanguage('ja');
    });

    // Header check
    expect(screen.getByText('柔軟な料金プラン')).toBeInTheDocument();
    expect(screen.getByText('あなたの進化の速度に合わせた最適なプランをお選びください。')).toBeInTheDocument();

    // Plan Names
    expect(screen.getByText('ソブリン無料')).toBeInTheDocument();
    expect(screen.getByText('オートノマス・プロ')).toBeInTheDocument();

    // Prices
    expect(screen.getByText('無料')).toBeInTheDocument();
    expect(screen.getByText('¥1,200')).toBeInTheDocument();
    expect(screen.getAllByText('/月').length).toBeGreaterThanOrEqual(1);

    // Buttons
    expect(screen.getByRole('link', { name: /無料で始める/i })).toBeInTheDocument();
    expect(screen.getByRole('link', { name: /プロへアップグレード/i })).toBeInTheDocument();
  });
});

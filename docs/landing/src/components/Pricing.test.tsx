/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
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
    expect(screen.getByText('Owning is free. Go Pro to unlock the economy.')).toBeInTheDocument();
    expect(screen.getByText('Free is for owning your AI OS. Pro is for letting your AI actually earn. Cancel anytime.')).toBeInTheDocument();

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
    expect(screen.getByText('Self-Healing AI Agent with Soul System')).toBeInTheDocument();
    expect(screen.getByText('AI Chat + VRM Avatar (Inochi2D Live Expression)')).toBeInTheDocument();
    expect(screen.getByText('WASM Skill Ecosystem + mock economy mode')).toBeInTheDocument();

    // Pro features
    expect(screen.getByText('Everything in Free, plus:')).toBeInTheDocument();
    expect(screen.getByText('Real economy unlocked — AI earns and invests 24/7')).toBeInTheDocument();
    expect(screen.getByText('Creator Marketplace — sell LoRA, VRM, Voice assets (85% goes to you)')).toBeInTheDocument();
  });

  it('switches content to Japanese when language changes', async () => {
    render(<Pricing />);
    
    await act(async () => {
      await i18n.changeLanguage('ja');
    });

    // Header check
    expect(screen.getByText('所有は無料。経済圏の解禁はプロで。')).toBeInTheDocument();
    expect(screen.getByText('Free は「AI OS を所有する」プラン、Pro は「AI に実際に稼がせる」プラン。いつでも解約できます。')).toBeInTheDocument();

    // Plan Names
    expect(screen.getByText('ソブリン無料')).toBeInTheDocument();
    expect(screen.getByText('オートノマス・プロ')).toBeInTheDocument();

    // Prices
    expect(screen.getByText('無料')).toBeInTheDocument();
    expect(screen.getAllByText('$9.99').length).toBeGreaterThanOrEqual(1);
    expect(screen.getAllByText('/月').length).toBeGreaterThanOrEqual(1);

    // Buttons
    expect(screen.getByRole('link', { name: /無料で始める/i })).toBeInTheDocument();
    expect(screen.getByRole('link', { name: /プロへアップグレード/i })).toBeInTheDocument();
  });
});

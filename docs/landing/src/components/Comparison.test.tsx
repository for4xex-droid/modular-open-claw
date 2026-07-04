/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import { render, screen, act, within } from '@testing-library/react';
import { Comparison } from './Comparison';
import i18n from '../i18n/config';

describe('Comparison Component', () => {
  beforeEach(() => {
    i18n.changeLanguage('en');
  });

  it('renders the section title and description', () => {
    render(<Comparison />);
    expect(screen.getByRole('heading', { name: 'Why switching is worth it' })).toBeInTheDocument();
    expect(screen.getByText(/data sovereignty, governance, and visible results/i)).toBeInTheDocument();
  });

  it('renders table column headers including Aiome', () => {
    render(<Comparison />);
    expect(screen.getByText('Cloud agent platforms')).toBeInTheDocument();
    expect(screen.getByText('Agent frameworks')).toBeInTheDocument();
    expect(screen.getByText('Aiome')).toBeInTheDocument();
  });

  it('renders all six comparison rows', () => {
    render(<Comparison />);
    expect(screen.getByText('Where your data lives')).toBeInTheDocument();
    expect(screen.getByText('Runaway protection')).toBeInTheDocument();
    expect(screen.getByText('Management console')).toBeInTheDocument();
    expect(screen.getByText('AI economic activity')).toBeInTheDocument();
    expect(screen.getByText('Monthly cost')).toBeInTheDocument();
    expect(screen.getByText('Lock-in')).toBeInTheDocument();
  });

  it('renders Aiome column values in the table', () => {
    render(<Comparison />);
    const table = screen.getByRole('table');
    expect(within(table).getByText('Your machine only (fully self-hosted)')).toBeInTheDocument();
    expect(within(table).getByText('Approval queue + audit log + 3-layer defense, built in')).toBeInTheDocument();
    expect(within(table).getByText('26-screen management console')).toBeInTheDocument();
    expect(within(table).getByText('Built-in B2A / A2A / A2C economy')).toBeInTheDocument();
  });

  it('renders the comparison disclaimer note', () => {
    render(<Comparison />);
    expect(screen.getByText(/Comparison reflects general characteristics/i)).toBeInTheDocument();
  });

  it('switches content to Japanese when language changes', async () => {
    render(<Comparison />);

    await act(async () => {
      await i18n.changeLanguage('ja');
    });

    expect(screen.getByRole('heading', { name: 'なぜ乗り換える価値があるのか' })).toBeInTheDocument();
    expect(screen.getByText('クラウド型エージェント基盤')).toBeInTheDocument();
    expect(screen.getByText('エージェントフレームワーク')).toBeInTheDocument();
  });
});

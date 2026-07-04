/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import { render, screen, act } from '@testing-library/react';
import { Problem } from './Problem';
import i18n from '../i18n/config';

describe('Problem Component', () => {
  beforeEach(() => {
    i18n.changeLanguage('en');
  });

  it('renders the section title and description', () => {
    render(<Problem />);
    expect(screen.getByText("Be honest — don't AI agents make you nervous?")).toBeInTheDocument();
    expect(screen.getByText(/You want autonomous AI working for you/i)).toBeInTheDocument();
  });

  it('renders three pain point cards', () => {
    render(<Problem />);
    expect(screen.getByText('Afraid of handing over your data')).toBeInTheDocument();
    expect(screen.getByText('Afraid it will run wild')).toBeInTheDocument();
    expect(screen.getByText("Can't see the results")).toBeInTheDocument();
  });

  it('renders pain point descriptions', () => {
    render(<Problem />);
    expect(screen.getByText(/Cloud agents ship your memories/i)).toBeInTheDocument();
    expect(screen.getByText(/delegating without a kill switch/i)).toBeInTheDocument();
    expect(screen.getByText(/nobody can answer how many hours/i)).toBeInTheDocument();
  });

  it('renders the bridge statement', () => {
    render(<Problem />);
    expect(screen.getByText('Aiome was designed as the direct answer to all three.')).toBeInTheDocument();
  });

  it('switches content to Japanese when language changes', async () => {
    render(<Problem />);

    await act(async () => {
      await i18n.changeLanguage('ja');
    });

    expect(screen.getByText('AI エージェント、本当は不安じゃないですか？')).toBeInTheDocument();
    expect(screen.getByText('データを渡すのが怖い')).toBeInTheDocument();
    expect(screen.getByText('暴走が怖い')).toBeInTheDocument();
    expect(screen.getByText(/成果が見えない/i)).toBeInTheDocument();
  });
});

/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { TreasureBox } from './TreasureBox';
import { useTreasure } from '../hooks/useTreasure';

jest.mock('../hooks/useTreasure', () => ({
  useTreasure: jest.fn()
}));

jest.mock('../i18n', () => ({
  useTranslation: () => ({
    t: (key: string) => key
  })
}));

describe('TreasureBox', () => {
  const mockItems = [
    {
      id: 'treasure-1',
      title: 'Magic Wand',
      description: 'Cast beautiful spells',
      url: 'https://magic.example.com',
      price_coins: 100,
      category: 'Weapons',
      score: 9.5,
      disclosure_label: 'Affiliate Link'
    }
  ];

  beforeEach(() => {
    jest.clearAllMocks();
    window.open = jest.fn();
  });

  it('renders treasure title and items grid', () => {
    (useTreasure as jest.Mock).mockReturnValue({
      items: mockItems,
      loading: false,
      error: null,
      refresh: jest.fn(),
      recordFeedback: jest.fn()
    });

    render(<TreasureBox />);

    expect(screen.getByText('treasure.title')).toBeInTheDocument();
    expect(screen.getByText('Magic Wand')).toBeInTheDocument();
    expect(screen.getByText('Cast beautiful spells')).toBeInTheDocument();
    expect(screen.getByText('Affiliate Link')).toBeInTheDocument();
    expect(screen.getByText('100')).toBeInTheDocument();
  });

  it('handles item click, records feedback and opens url', async () => {
    const recordFeedback = jest.fn().mockResolvedValue(true);
    (useTreasure as jest.Mock).mockReturnValue({
      items: mockItems,
      loading: false,
      error: null,
      refresh: jest.fn(),
      recordFeedback
    });

    render(<TreasureBox />);

    const itemCard = screen.getByText('Magic Wand');
    fireEvent.click(itemCard);

    expect(recordFeedback).toHaveBeenCalledWith('treasure-1', 'click');
    await waitFor(() => {
      expect(window.open).toHaveBeenCalledWith('https://magic.example.com', '_blank', 'noopener,noreferrer');
    });
  });
});

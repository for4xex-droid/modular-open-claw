/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import React from 'react';
import { render, fireEvent, screen } from '@testing-library/react';
import { BiomeHUD } from './BiomeHUD';
import { BiomeControls } from './BiomeControls';
import { BiomeResult } from './BiomeResult';
import { BiomeDendou } from './BiomeDendou';
import { BiomeTutorial } from './BiomeTutorial';

describe('Biome HUD & Controls Components', () => {
  it('BiomeHUD が正しく情報を表示すること', () => {
    render(
      <BiomeHUD
        generation={150}
        rarity="Rare"
      />
    );

    expect(screen.getByText(/150/)).toBeInTheDocument();
    expect(screen.getByText(/Rare/)).toBeInTheDocument();
    expect(screen.queryByTestId('biome-lenia-scorecard')).not.toBeInTheDocument();
  });

  it('BiomeHUD が rarityProgress 提供時に Lenia スコアカードを描画すること', () => {
    const progress = {
      active_cells: 120,
      symmetry_score: 0.72,
      complexity_score: 0.55,
      mass: 350.5,
      locomotion: 0.42,
      longevity: 85,
      species_hash: 123456789,
      has_homeostasis: true,
      condition_structure: true,
    };

    render(
      <BiomeHUD
        generation={150}
        rarity="Rare"
        rarityProgress={progress}
        activeCellCount={120}
      />
    );

    expect(screen.getByTestId('biome-lenia-scorecard')).toBeInTheDocument();
    expect(screen.getByTestId('biome-mass')).toHaveTextContent('350.5');
    expect(screen.getByText(/85 tick/)).toBeInTheDocument();
  });

  it('正常系: BiomeHUD が Legendary, Epic, Uncommon, Common の各レアリティに応じた装飾を正しく表示すること', () => {
    const { rerender } = render(
      <BiomeHUD generation={200} rarity="Legendary" />
    );
    expect(screen.getByText('🔥')).toBeInTheDocument();
    const rarityBadge = screen.getByTestId('biome-rarity');
    expect(rarityBadge.style.color).toBe('var(--biome-rarity-legendary)');

    rerender(<BiomeHUD generation={200} rarity="Epic" />);
    expect(screen.getByText('🔮')).toBeInTheDocument();

    rerender(<BiomeHUD generation={200} rarity="Uncommon" />);
    expect(screen.getByText('🌟')).toBeInTheDocument();

    rerender(<BiomeHUD generation={200} rarity="Common" />);
    expect(screen.getByText('🍃')).toBeInTheDocument();
  });

  it('BiomeControls のボタン押下がハンドラーを呼び出すこと', () => {
    const onToggleSeedMode = jest.fn();
    const onShowCatalog = jest.fn();
    const onRewind = jest.fn();

    render(
      <BiomeControls
        seedMode={true}
        onToggleSeedMode={onToggleSeedMode}
        leniaMu={0.15}
        leniaSigma={0.017}
        onLeniaMuChange={jest.fn()}
        onLeniaSigmaChange={jest.fn()}
        onShowCatalog={onShowCatalog}
        onRewind={onRewind}
        paused={false}
        onTogglePause={jest.fn()}
      />
    );

    fireEvent.click(screen.getByTestId('control-seed-mode'));
    expect(onToggleSeedMode).toHaveBeenCalled();

    fireEvent.click(screen.getByTestId('control-catalog'));
    expect(onShowCatalog).toHaveBeenCalled();

    const rewindBtn = screen.getByRole('button', { name: /Rewind/i });
    fireEvent.click(rewindBtn);
    expect(onRewind).toHaveBeenCalled();
  });

  it('BiomeControls が onNewSeed ハンドラーを持つ場合に New Seed ボタンを表示し、クリック時にハンドラーを呼び出すこと', () => {
    const onNewSeed = jest.fn();
    render(
      <BiomeControls
        seedMode={true}
        onToggleSeedMode={jest.fn()}
        leniaMu={0.15}
        leniaSigma={0.017}
        onLeniaMuChange={jest.fn()}
        onLeniaSigmaChange={jest.fn()}
        onShowCatalog={jest.fn()}
        onRewind={jest.fn()}
        paused={false}
        onTogglePause={jest.fn()}
        onNewSeed={onNewSeed}
      />
    );

    const newSeedBtn = screen.getByRole('button', { name: /New Seed/i });
    expect(newSeedBtn).toBeInTheDocument();
    fireEvent.click(newSeedBtn);
    expect(onNewSeed).toHaveBeenCalled();
  });

  it('BiomeControls が onNewSeed ハンドラーを持たない場合に New Seed ボタンを表示しないこと', () => {
    render(
      <BiomeControls
        seedMode={true}
        onToggleSeedMode={jest.fn()}
        leniaMu={0.15}
        leniaSigma={0.017}
        onLeniaMuChange={jest.fn()}
        onLeniaSigmaChange={jest.fn()}
        onShowCatalog={jest.fn()}
        onRewind={jest.fn()}
        paused={false}
        onTogglePause={jest.fn()}
      />
    );

    expect(screen.queryByRole('button', { name: /New Seed/i })).not.toBeInTheDocument();
  });

  it('BiomeResult が最終結果を表示すること', () => {
    const onSave = jest.fn();
    render(
      <BiomeResult
        generation={300}
        rarity="Legendary"
        onSave={onSave}
        onClose={jest.fn()}
      />
    );

    expect(screen.getByText(/Legendary/)).toBeInTheDocument();
    expect(screen.getByText(/300/)).toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: /Save/i }));
    expect(onSave).toHaveBeenCalled();
  });

  it('BiomeDendou が殿堂入りのリストを正しく描画すること', () => {
    const onLoad = jest.fn();
    const mockList = [
      { id: '1', name: 'Legendary Specimen A', generation: 250, rarity: 'Legendary', date: '2026-06-10' },
    ];

    render(<BiomeDendou list={mockList} onLoad={onLoad} />);

    expect(screen.getByText('Legendary Specimen A')).toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: /Load/i }));
    expect(onLoad).toHaveBeenCalledWith('1');
  });

  it('BiomeDendou が Lenia 種パラメータを詳細に表示すること', () => {
    const mockList = [
      {
        id: '1',
        name: 'Orbium A',
        generation: 200,
        rarity: 'Epic',
        date: '2026-06-10',
        genome_data: JSON.stringify({ mu: 0.15, sigma: 0.017, species_hash: 999, mass: 400, longevity: 120 }),
        active_cell_count: 80,
      },
    ];

    render(<BiomeDendou list={mockList} onLoad={jest.fn()} />);

    fireEvent.click(screen.getByRole('button', { name: /biomeConsole\.detail/i }));

    expect(screen.getByText('Lenia 種パラメータ')).toBeInTheDocument();
    expect(screen.getByText('0.150')).toBeInTheDocument();
    expect(screen.getByText('120 tick')).toBeInTheDocument();
  });

  it('BiomeDendou が詳細情報（元素バランス、形態分布、発見した反応、活性セル数）を正しく描画すること', () => {
    const mockList = [
      {
        id: '1',
        name: 'Legendary Specimen A',
        generation: 250,
        rarity: 'Legendary',
        date: '2026-06-10',
        element_balance: '{"C":40,"N":30,"P":10,"H":20}',
        morphology_distribution: '{"Predator":2,"Producer":1}',
        discovered_reactions: '["N+P->C+H","Fe+O->Si"]',
        active_cell_count: 50,
      },
    ];

    render(<BiomeDendou list={mockList} onLoad={jest.fn()} />);

    fireEvent.click(screen.getByRole('button', { name: /biomeConsole\.detail/i }));

    expect(screen.getByText(/biomeConsole\.activeCells 50/i)).toBeInTheDocument();
    expect(screen.getByText('C')).toBeInTheDocument();
    expect(screen.getByText('40.0%')).toBeInTheDocument();
  });

  it('BiomeResult が詳細情報を描画すること', () => {
    render(
      <BiomeResult
        generation={300}
        rarity="Legendary"
        onSave={jest.fn()}
        onClose={jest.fn()}
        elementBalance={{ C: 40, N: 30, P: 10, H: 20 }}
        morphologyDistribution={{ Predator: 2, Producer: 1 }}
        discoveredReactions={['N+P->C+H', 'Fe+O->Si']}
        activeCellCount={50}
      />
    );

    expect(screen.getByText(/Legendary/)).toBeInTheDocument();
    expect(screen.getByText(/biomeConsole\.activeCells 50/)).toBeInTheDocument();
  });

  it('BiomeTutorial が Lenia 向けステップを正しく切り替えて表示すること', () => {
    render(<BiomeTutorial onClose={jest.fn()} />);

    expect(screen.getByText('🧬 Lenia 生命場を観察する')).toBeInTheDocument();

    const nextBtn = screen.getByRole('button', { name: /次へ/i });
    fireEvent.click(nextBtn);
    expect(screen.getByText('🌱 種まきで新しい種を誕生させる')).toBeInTheDocument();
  });

  it('BiomeDendou が空の detail データや不正な JSON でもクラッシュせずにレンダリングされること', () => {
    render(
      <BiomeDendou
        list={[{
          id: '2',
          name: 'Faulty Specimen',
          generation: 100,
          rarity: 'Common',
          date: '2026-06-11',
          element_balance: 'invalid-json',
        }]}
        onLoad={jest.fn()}
      />
    );

    expect(screen.getByText('Faulty Specimen')).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: /biomeConsole\.detail/i }));
    expect(screen.queryByText(/biomeConsole\.elementRatio/i)).not.toBeInTheDocument();
  });
});

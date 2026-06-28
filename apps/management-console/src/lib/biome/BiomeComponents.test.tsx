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
        elementBalance={{ C: 40, N: 30, P: 10, H: 20 }}
      />
    );

    // 世代数やレアリティが表示されていることをアサート
    expect(screen.getByText(/150/)).toBeInTheDocument();
    expect(screen.getByText(/Rare/)).toBeInTheDocument();
    // 進行度チェックリストが表示されていないことをアサート
    expect(screen.queryByText(/ランクアップ条件/)).not.toBeInTheDocument();
  });

  it('BiomeHUD が rarityProgress 提供時にチェックリストを正しく描画すること', () => {
    const progress = {
      active_cells: 550,
      morphology_count: 3,
      has_homeostasis: true,
      diversity_index: 0.85,
      condition_active_500: true,
      condition_morph_3: true,
      condition_morph_4: false,
      condition_active_1000: false,
    };

    render(
      <BiomeHUD
        generation={150}
        rarity="Rare"
        elementBalance={{ C: 40 }}
        rarityProgress={progress}
      />
    );

    expect(screen.getByText(/ランクアップ条件/)).toBeInTheDocument();
    expect(screen.getByText(/活性セル 500\+/)).toBeInTheDocument();
    expect(screen.getByText(/特殊形態 3種類\+/)).toBeInTheDocument();
    expect(screen.getByText(/0.850/)).toBeInTheDocument();
  });

  it('正常系: BiomeHUD が Legendary, Epic, Uncommon, Common の各レアリティに応じた装飾を正しく表示すること', () => {
    // 1. Legendary
    const { rerender } = render(
      <BiomeHUD
        generation={200}
        rarity="Legendary"
        elementBalance={{ C: 50 }}
      />
    );
    expect(screen.getByText('🔥')).toBeInTheDocument();
    const rarityBadge = screen.getByTestId('biome-rarity');
    expect(rarityBadge.style.color).toBe('var(--biome-rarity-legendary)');

    
    // 2. Epic
    rerender(
      <BiomeHUD
        generation={200}
        rarity="Epic"
        elementBalance={{ C: 50 }}
      />
    );
    expect(screen.getByText('🔮')).toBeInTheDocument(); 

    // 3. Uncommon
    rerender(
      <BiomeHUD
        generation={200}
        rarity="Uncommon"
        elementBalance={{ C: 50 }}
      />
    );
    expect(screen.getByText('🌟')).toBeInTheDocument();

    // 4. Common (default)
    rerender(
      <BiomeHUD
        generation={200}
        rarity="Common"
        elementBalance={{ C: 50 }}
      />
    );
    expect(screen.getByText('🍃')).toBeInTheDocument();
  });

  it('BiomeControls のボタン押下がハンドラーを呼び出すこと', () => {
    const onSelectElement = jest.fn();
    const onSelectCrisis = jest.fn();
    const onRewind = jest.fn();

    render(
      <BiomeControls
        selectedElement={null}
        onSelectElement={onSelectElement}
        selectedCrisis={null}
        onSelectCrisis={onSelectCrisis}
        onInjectElement={jest.fn()}
        onTriggerCrisis={jest.fn()}
        onRollSubstance={jest.fn()}
        onRewind={onRewind}
        paused={false}
        onTogglePause={jest.fn()}
      />
    );

    // 元素選択ボタンのテスト
    const injectBtn = screen.getByRole('button', { name: /C/ });
    fireEvent.click(injectBtn);
    expect(onSelectElement).toHaveBeenCalledWith('C');

    // 災害選択ボタンのテスト
    const meteorBtn = screen.getByRole('button', { name: /Meteor/i });
    fireEvent.click(meteorBtn);
    expect(onSelectCrisis).toHaveBeenCalledWith('Meteor');

    // 巻き戻しボタンのテスト
    const rewindBtn = screen.getByRole('button', { name: /Rewind/i });
    fireEvent.click(rewindBtn);
    expect(onRewind).toHaveBeenCalled();
  });

  it('BiomeControls が onNewSeed ハンドラーを持つ場合に New Seed ボタンを表示し、クリック時にハンドラーを呼び出すこと', () => {
    const onNewSeed = jest.fn();
    render(
      <BiomeControls
        selectedElement={null}
        onSelectElement={jest.fn()}
        selectedCrisis={null}
        onSelectCrisis={jest.fn()}
        onInjectElement={jest.fn()}
        onTriggerCrisis={jest.fn()}
        onRollSubstance={jest.fn()}
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
        selectedElement={null}
        onSelectElement={jest.fn()}
        selectedCrisis={null}
        onSelectCrisis={jest.fn()}
        onInjectElement={jest.fn()}
        onTriggerCrisis={jest.fn()}
        onRollSubstance={jest.fn()}
        onRewind={jest.fn()}
        paused={false}
        onTogglePause={jest.fn()}
      />
    );

    const newSeedBtn = screen.queryByRole('button', { name: /New Seed/i });
    expect(newSeedBtn).not.toBeInTheDocument();
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

    const saveBtn = screen.getByRole('button', { name: /Save/i });
    fireEvent.click(saveBtn);
    expect(onSave).toHaveBeenCalled();
  });

  it('BiomeDendou が殿堂入りのリストを正しく描画すること', () => {
    const mockList = [
      { id: '1', name: 'Legendary Specimen A', generation: 250, rarity: 'Legendary', date: '2026-06-10' }
    ];
    const onLoad = jest.fn();

    render(
      <BiomeDendou
        list={mockList}
        onLoad={onLoad}
      />
    );

    expect(screen.getByText('Legendary Specimen A')).toBeInTheDocument();

    const loadBtn = screen.getByRole('button', { name: /Load/i });
    fireEvent.click(loadBtn);
    expect(onLoad).toHaveBeenCalledWith('1');
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
      }
    ];
    const onLoad = jest.fn();

    render(
      <BiomeDendou
        list={mockList}
        onLoad={onLoad}
      />
    );

    expect(screen.getByText('Legendary Specimen A')).toBeInTheDocument();
    
    // 詳細トグルボタンをクリック
    const detailBtn = screen.getByRole('button', { name: /🔍 詳細/i });
    fireEvent.click(detailBtn);

    // 展開後に詳細が表示されていることをアサート
    expect(screen.getByText(/活性セル数: 50/i)).toBeInTheDocument();
    expect(screen.getByText('C')).toBeInTheDocument();
    expect(screen.getByText('40.0%')).toBeInTheDocument();
    expect(screen.getByText('Predator')).toBeInTheDocument();
    expect(screen.getByText('66.7%')).toBeInTheDocument();
    expect(screen.getByText(/N\+P->C\+H/i)).toBeInTheDocument();
    expect(screen.getByText(/Fe\+O->Si/i)).toBeInTheDocument();
  });

  it('BiomeResult が詳細情報（元素バランス、形態分布、発見した反応、活性セル数）を描画すること', () => {
    const onSave = jest.fn();
    render(
      <BiomeResult
        generation={300}
        rarity="Legendary"
        onSave={onSave}
        onClose={jest.fn()}
        elementBalance={{ C: 40, N: 30, P: 10, H: 20 }}
        morphologyDistribution={{ Predator: 2, Producer: 1 }}
        discoveredReactions={['N+P->C+H', 'Fe+O->Si']}
        activeCellCount={50}
      />
    );

    expect(screen.getByText(/Legendary/)).toBeInTheDocument();
    expect(screen.getByText(/300/)).toBeInTheDocument();
    expect(screen.getByText(/活性セル数: 50/)).toBeInTheDocument();
    expect(screen.getByText('C')).toBeInTheDocument();
    expect(screen.getByText('40.0%')).toBeInTheDocument();
    expect(screen.getByText('Predator')).toBeInTheDocument();
    expect(screen.getByText('66.7%')).toBeInTheDocument();
    expect(screen.getByText(/N\+P->C\+H/)).toBeInTheDocument();
    expect(screen.getByText(/Fe\+O->Si/)).toBeInTheDocument();
  });

  it('BiomeTutorial が新ステップ「元素反応の連鎖」を含むすべてのステップを正しく切り替えて表示すること', () => {
    const onClose = jest.fn();
    render(<BiomeTutorial onClose={onClose} />);

    expect(screen.getByText('🧬 生命の進化を見守る')).toBeInTheDocument();

    const nextBtn = screen.getByRole('button', { name: /次へ/i });
    fireEvent.click(nextBtn);
    fireEvent.click(nextBtn);
    fireEvent.click(nextBtn);

    expect(screen.getByText('⚗️ 元素反応の連鎖')).toBeInTheDocument();
    expect(screen.getByText(/反応は質量を保存し/)).toBeInTheDocument();
  });

  it('BiomeDendou が空の detail データや不正な JSON でもクラッシュせずにレンダリングされること', () => {
    const mockListWithInvalidData = [
      {
        id: '2',
        name: 'Faulty Specimen',
        generation: 100,
        rarity: 'Common',
        date: '2026-06-11',
        element_balance: 'invalid-json',
        morphology_distribution: '',
        discovered_reactions: 'invalid-array',
        active_cell_count: undefined,
      }
    ];

    render(
      <BiomeDendou
        list={mockListWithInvalidData}
        onLoad={jest.fn()}
      />
    );

    expect(screen.getByText('Faulty Specimen')).toBeInTheDocument();
    
    // 詳細トグルボタンをクリックして展開するが、解析失敗データのためパーセントバー等は描画されず、かつクラッシュもしないこと
    const detailBtn = screen.getByRole('button', { name: /🔍 詳細/i });
    fireEvent.click(detailBtn);

    expect(screen.queryByText(/元素比率/i)).not.toBeInTheDocument();
    expect(screen.queryByText(/形態分布/i)).not.toBeInTheDocument();
    expect(screen.queryByText(/発見した反応/i)).not.toBeInTheDocument();
    expect(screen.queryByText(/活性セル数/i)).not.toBeInTheDocument();
  });
});

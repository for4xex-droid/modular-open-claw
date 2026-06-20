import React from 'react';
import { render, fireEvent, screen } from '@testing-library/react';
import { BiomeHUD } from './BiomeHUD';
import { BiomeControls } from './BiomeControls';
import { BiomeResult } from './BiomeResult';
import { BiomeDendou } from './BiomeDendou';

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
});

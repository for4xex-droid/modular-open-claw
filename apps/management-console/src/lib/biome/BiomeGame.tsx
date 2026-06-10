import { useState, useEffect } from 'react';
import { useBiomeEngine } from '../../hooks/useBiomeEngine';
import { BiomeHUD } from './BiomeHUD';
import { BiomeControls } from './BiomeControls';
import { BiomeRenderer } from './BiomeRenderer';

export interface BiomeGameProps {
  seed?: number;
}

export function BiomeGame({ seed }: BiomeGameProps) {
  const [paused, setPaused] = useState(false);
  const [currentSeed, setCurrentSeed] = useState(() => seed ?? Math.floor(Math.random() * 1000000));

  // Props の seed が明示的に変更されたら同期する
  useEffect(() => {
    if (seed !== undefined) {
      setCurrentSeed(seed);
    }
  }, [seed]);

  const {
    loading,
    generation,
    tick,
    rewind,
    getRenderView,
    injectElement,
    applyCrisis,
    getRarity,
    getActiveCellCount,
    getElementBalance,
    getMutationBoost,
    ticksSinceMutation,
  } = useBiomeEngine({ seed: currentSeed, paused });

  // 初期セルの生成および状態更新
  useEffect(() => {
    if (loading) return;

    // テスト・デモ用に中央部にいくつかアクティブなセルを配置（炭素注入）
    for (let y = 60; y <= 68; y++) {
      for (let x = 60; x <= 68; x++) {
        injectElement(x, y, 0, 5000);
      }
    }
  }, [loading, currentSeed, injectElement]);


  // ゲームの自動進行タイマー
  useEffect(() => {
    if (loading || paused) return;

    const interval = setInterval(() => {
      tick();
    }, 100); // 100ms ごとに tick (10fps)

    return () => clearInterval(interval);
  }, [loading, paused, tick]);

  if (loading) {
    return (
      <div style={{
        display: 'flex',
        justifyContent: 'center',
        alignItems: 'center',
        height: '25rem',
        color: 'var(--white-100)',
        fontFamily: 'var(--font-main)'
      }}>
        Loading Biome Engine...
      </div>
    );
  }

  // 元素比率データを WASM から取得して HUD 向けにマップ
  const balanceRaw = getElementBalance();
  const elementBalance = {
    C: balanceRaw[0] || 0,
    N: balanceRaw[1] || 0,
    P: balanceRaw[2] || 0,
    H: balanceRaw[3] || 0,
    O: balanceRaw[4] || 0,
    S: balanceRaw[5] || 0,
    Fe: balanceRaw[6] || 0,
    Si: balanceRaw[7] || 0,
  };

  const rarities = ['Common', 'Uncommon', 'Rare', 'Epic', 'Legendary'];
  const rarity = rarities[getRarity()] || 'Common';

  const elementIndexMap: Record<string, number> = {
    C: 0,
    N: 1,
    P: 2,
    H: 3,
  };

  const handleInjectElement = (el: string) => {
    const idx = elementIndexMap[el];
    if (idx !== undefined) {
      // 画面中央部（64, 64）の 5x5 エリアに分散して元素を注入する
      for (let y = 62; y <= 66; y++) {
        for (let x = 62; x <= 66; x++) {
          injectElement(x, y, idx, 2000);
        }
      }
    }
  };

  const handleTriggerCrisis = (crisis: string) => {
    const type = crisis === 'Meteor' ? 'meteor' : 'ice_age';
    applyCrisis(type, 64, 64);
  };

  const renderView = getRenderView();

  return (
    <div style={{
      display: 'flex',
      gap: 'var(--layout-panel-gap)',
      padding: 'var(--layout-panel-gap)',
      background: 'var(--bg-primary)',
      borderRadius: 'var(--radius-md)',
      color: 'var(--white-100)',
      fontFamily: 'var(--font-main)'
    }}>
      {/* メインレンダラー */}
      <div style={{ flex: '1', display: 'flex', justifyContent: 'center', alignItems: 'center' }}>
        <BiomeRenderer width={512} height={512} renderView={renderView} />
      </div>

      {/* コントロール・情報HUDパネル */}
      <div style={{ width: 'var(--layout-right-panel-width)', display: 'flex', flexDirection: 'column', gap: 'var(--layout-panel-gap)' }}>
        <BiomeHUD
          generation={generation}
          rarity={rarity}
          elementBalance={elementBalance}
          mutationBoost={getMutationBoost()}
          ticksSinceMutation={ticksSinceMutation()}
          activeCellCount={getActiveCellCount()}
        />
        <BiomeControls
          onInjectElement={handleInjectElement}
          onTriggerCrisis={handleTriggerCrisis}
          onRewind={() => rewind(20)}
          paused={paused}
          onTogglePause={() => setPaused(!paused)}
          onNewSeed={() => setCurrentSeed(Math.floor(Math.random() * 1000000))}
        />

      </div>
    </div>
  );
}


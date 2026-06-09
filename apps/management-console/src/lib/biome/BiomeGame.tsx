import { useState, useEffect } from 'react';
import { useBiomeEngine } from '../../hooks/useBiomeEngine';
import { BiomeHUD } from './BiomeHUD';
import { BiomeControls } from './BiomeControls';
import { BiomeRenderer, CellInfo } from './BiomeRenderer';

export interface BiomeGameProps {
  seed: number;
}

export function BiomeGame({ seed }: BiomeGameProps) {
  const [paused, setPaused] = useState(false);
  const { loading, generation, tick, rewind } = useBiomeEngine({ seed, paused });
  const [cells, setCells] = useState<CellInfo[]>([]);

  // 初期セルの生成および状態更新
  useEffect(() => {
    if (loading) return;

    // 128x128 グリッドの初期セル情報を生成
    const initialCells: CellInfo[] = [];
    for (let y = 0; y < 128; y++) {
      for (let x = 0; x < 128; x++) {
        // テスト・デモ用に中央部にいくつかアクティブなセルを配置
        const isActive = x >= 60 && x <= 68 && y >= 60 && y <= 68;
        initialCells.push({
          x,
          y,
          active: isActive,
          morphology: 0,
          elements: [isActive ? 1000 : 0, 0, 0, 0, 0, 0, 0, 0], // C
        });
      }
    }
    setCells(initialCells);
  }, [loading, seed]);

  // ゲームの自動進行タイマー
  useEffect(() => {
    if (loading || paused) return;

    const interval = setInterval(() => {
      tick();
      
      // セルの元素拡散や形態変化をシミュレートして状態更新 (React側での反映)
      setCells((prevCells) =>
        prevCells.map((cell) => {
          if (!cell.active) return cell;
          
          // 元素反応・突然変異などを適当にモック（実体はWASM側で回る）
          const nextElements = [...cell.elements];
          // 代謝
          if (nextElements[0] > 10) nextElements[0] -= 10;
          return {
            ...cell,
            elements: nextElements,
          };
        })
      );
    }, 100); // 100ms ごとに tick (10fps)

    return () => clearInterval(interval);
  }, [loading, paused, tick]);

  if (loading) {
    return (
      <div style={{
        display: 'flex',
        justifyContent: 'center',
        alignItems: 'center',
        height: '400px',
        color: '#fff',
        fontFamily: 'system-ui, sans-serif'
      }}>
        Loading Biome Engine...
      </div>
    );
  }

  // 代替の元素比率データ
  const elementBalance = { C: 40, N: 30, P: 10, H: 20 };

  return (
    <div style={{
      display: 'flex',
      gap: '24px',
      padding: '24px',
      background: '#07070a',
      borderRadius: '16px',
      color: '#fff',
      fontFamily: 'system-ui, sans-serif'
    }}>
      {/* メインレンダラー */}
      <div style={{ flex: '1', display: 'flex', justifyContent: 'center', alignItems: 'center' }}>
        <BiomeRenderer width={512} height={512} cells={cells} />
      </div>

      {/* コントロール・情報HUDパネル */}
      <div style={{ width: '300px', display: 'flex', flexDirection: 'column', gap: '20px' }}>
        <BiomeHUD
          generation={generation}
          rarity="Common"
          elementBalance={elementBalance}
        />
        <BiomeControls
          onInjectElement={() => {}}
          onTriggerCrisis={() => {}}
          onRewind={() => rewind(20)}
          paused={paused}
          onTogglePause={() => setPaused(!paused)}
        />
      </div>
    </div>
  );
}

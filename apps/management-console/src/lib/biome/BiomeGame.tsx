/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

import { useState, useEffect, useCallback } from 'react';
import { useBiomeEngine } from '../../hooks/useBiomeEngine';
import { BiomeHUD } from './BiomeHUD';
import { BiomeControls } from './BiomeControls';
import { BiomeRenderer } from './BiomeRenderer';
import { CycleSelect } from './CycleSelect';
import { BiomeResult } from './BiomeResult';
import { BiomeDendou, Specimen } from './BiomeDendou';
import { API_BASE } from '../../config';
import { isAuthenticated } from '../../lib/auth';

export interface BiomeGameProps {
  seed?: number;
}

export function BiomeGame({ seed }: BiomeGameProps) {
  const [paused, setPaused] = useState(false);
  const [currentSeed, setCurrentSeed] = useState(() => seed ?? Math.floor(Math.random() * 1000000));
  const [speed, setSpeed] = useState(100); // ms (100 = 1x, 50 = 2x, 20 = 5x, 10 = 10x)
  const [showResult, setShowResult] = useState(false);
  const [showDendou, setShowDendou] = useState(false);
  const [dendouList, setDendouList] = useState<Specimen[]>([]);

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
    serializeGenome,
  } = useBiomeEngine({ seed: currentSeed, paused });

  // 殿堂入りリストをサーバーからフェッチする関数
  const fetchDendouList = useCallback(async () => {
    if (!isAuthenticated()) return;
    try {
      const token = localStorage.getItem('jwt_token');
      const res = await fetch(`${API_BASE}/api/v1/biome/specimens`, {
        headers: {
          'Authorization': `Bearer ${token}`
        }
      });
      if (res.ok) {
        const data = await res.json();
        // Specimen 構造体にマッピング
        const mapped: Specimen[] = data.map((item: any) => ({
          id: item.id,
          name: item.specimen_name,
          generation: 200, // 保存時の世代
          rarity: item.rarity,
          date: new Date(item.created_at).toLocaleDateString()
        }));
        setDendouList(mapped);
      }
    } catch (e) {
      console.error('Failed to fetch specimens', e);
    }
  }, []);

  // 初期化時に殿堂入りリストを読み込み
  useEffect(() => {
    fetchDendouList();
  }, [fetchDendouList]);

  // Props の seed が明示的に変更されたら同期する
  useEffect(() => {
    if (seed !== undefined) {
      setCurrentSeed(seed);
    }
  }, [seed]);

  // 初期セルの生成および状態更新
  useEffect(() => {
    if (loading) return;

    // テスト・デモ用に中央部にいくつかアクティブなセルを配置（炭素注入）
    for (let y = 60; y <= 68; y++) {
      for (let x = 60; x <= 68; x++) {
        injectElement(x, y, 0, 5000);
      }
    }
    // 新しいシードのときはリザルト表示などをリセット
    setShowResult(false);
    setPaused(false);
  }, [loading, currentSeed, injectElement]);

  // 200 世代に達した時点で自動停止し、リザルトダイアログを表示
  useEffect(() => {
    if (generation >= 200 && !showResult && !showDendou) {
      setPaused(true);
      setShowResult(true);
    }
  }, [generation, showResult, showDendou]);

  // ゲームの自動進行タイマー
  useEffect(() => {
    if (loading || paused) return;

    const interval = setInterval(() => {
      tick();
    }, speed);

    return () => clearInterval(interval);
  }, [loading, paused, speed, tick]);

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
  const rarityIndex = getRarity();
  const rarity = rarities[rarityIndex] || 'Common';

  const elementIndexMap: Record<string, number> = {
    C: 0,
    N: 1,
    P: 2,
    H: 3,
  };

  const handleInjectElement = (el: string) => {
    const idx = elementIndexMap[el];
    if (idx !== undefined) {
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

  const handleSaveSpecimen = async () => {
    if (!isAuthenticated()) return;
    try {
      const token = localStorage.getItem('jwt_token');
      const runId = crypto.randomUUID();
      const agentId = crypto.randomUUID();

      // 1. Run 情報を送信
      const runPayload = {
        id: runId,
        agent_id: agentId,
        generation: 200,
        score: getActiveCellCount() * 1.5, // 簡易スコア計算
        max_generation: 200,
        cell_count: getActiveCellCount(),
        is_dendou: 1
      };

      const runRes = await fetch(`${API_BASE}/api/v1/biome/runs`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'Authorization': `Bearer ${token}`
        },
        body: JSON.stringify(runPayload)
      });

      if (!runRes.ok) throw new Error('Failed to save run');

      // 2. ゲノムデータを中央付近のセル(64,64)からシリアライズして取得
      const genomeStr = serializeGenome(64, 64) || '{}';

      // 3. Specimen 情報を送信
      const specimenPayload = {
        id: crypto.randomUUID(),
        run_id: runId,
        specimen_name: `Species_${currentSeed}`,
        genome_data: genomeStr,
        rarity: rarity.toLowerCase()
      };

      const specRes = await fetch(`${API_BASE}/api/v1/biome/specimens`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'Authorization': `Bearer ${token}`
        },
        body: JSON.stringify(specimenPayload)
      });

      if (specRes.ok) {
        setShowResult(false);
        setShowDendou(true);
        fetchDendouList();
      }
    } catch (e) {
      console.error('Failed to save specimen', e);
    }
  };

  const handleLoadDendou = (id: string) => {
    console.log(`Loading specimen: ${id}`);
    // MVP: 単に読み込みメッセージを出力し、シミュレーションをリセットして再開
    setShowDendou(false);
    setCurrentSeed(Math.floor(Math.random() * 1000000));
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
        
        {/* シミュレーションサイクル選択速度パネル */}
        <CycleSelect
          speed={speed}
          onSpeedChange={setSpeed}
          paused={paused}
          onTogglePause={() => setPaused(!paused)}
        />

        <BiomeControls
          onInjectElement={handleInjectElement}
          onTriggerCrisis={handleTriggerCrisis}
          onRewind={() => rewind(20)}
          paused={paused}
          onTogglePause={() => setPaused(!paused)}
          onNewSeed={() => {
            setCurrentSeed(Math.floor(Math.random() * 1000000));
            setShowDendou(false);
          }}
        />

        {/* 殿堂入りリスト表示ボタン */}
        <button
          onClick={() => setShowDendou(!showDendou)}
          style={{
            background: 'var(--white-05)',
            border: '1px solid var(--white-10)',
            borderRadius: 'var(--radius-sm)',
            color: 'var(--white-100)',
            padding: '8px',
            cursor: 'pointer',
            fontWeight: '600'
          }}
        >
          {showDendou ? 'Back to Simulation' : 'Hall of Fame'}
        </button>

        {showDendou && (
          <BiomeDendou list={dendouList} onLoad={handleLoadDendou} />
        )}
      </div>

      {showResult && (
        <BiomeResult
          generation={generation}
          rarity={rarity}
          onSave={handleSaveSpecimen}
          onClose={() => {
            setShowResult(false);
            // 200世代を超えて続行可能にするため、一時的に進行を再開
            setPaused(false);
          }}
        />
      )}
    </div>
  );
}

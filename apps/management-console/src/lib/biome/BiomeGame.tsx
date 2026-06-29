/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

import { useState, useEffect, useCallback, useRef } from 'react';
import { useBiomeEngine } from '../../hooks/useBiomeEngine';
import { BiomeHUD } from './BiomeHUD';
import { BiomeControls } from './BiomeControls';
import { BiomeCanvas } from './BiomeCanvas';
import type { InjectionMark } from './biomeTypes';
import { CycleSelect } from './CycleSelect';
import { BiomeResult } from './BiomeResult';
import { BiomeDendou, Specimen } from './BiomeDendou';
import { BiomeTutorial } from './BiomeTutorial';
import { BiomeEventToast } from './BiomeEventToast';
import { API_BASE } from '../../config';
import { isAuthenticated } from '../../lib/auth';
import ThemeBridge from './ThemeBridge';

export interface BiomeGameProps {
  seed?: number;
  standalone?: boolean;
}

export function BiomeGame({ seed, standalone }: BiomeGameProps) {
  const [paused, setPaused] = useState(false);
  const [currentSeed, setCurrentSeed] = useState(() => seed ?? Math.floor(Math.random() * 1000000));
  const [speed, setSpeed] = useState(100); // ms (100 = 1x, 50 = 2x, 20 = 5x, 10 = 10x)
  const [showResult, setShowResult] = useState(false);
  const [resultDismissed, setResultDismissed] = useState(false);
  const [showDendou, setShowDendou] = useState(false);
  const [dendouList, setDendouList] = useState<Specimen[]>([]);
  const [showTutorial, setShowTutorial] = useState(false);

  // インタラクション用 State
  const [selectedElement, setSelectedElement] = useState<string | null>('C');
  const [selectedCrisis, setSelectedCrisis] = useState<string | null>(null);
  const [hoverCell, setHoverCell] = useState<{ x: number; y: number } | null>(null);
  const [hoverData, setHoverData] = useState<any>(null);

  const [shakeOffset, setShakeOffset] = useState({ x: 0, y: 0 });
  const [comboCount, setComboCount] = useState(0);
  const [activeEvents, setActiveEvents] = useState<any[]>([]);
  const [rarityProgress, setRarityProgress] = useState<any>(null);
  const lastInjectTimeRef = useRef(0);

  // エフェクト & Bloom用 State
  const [effectType, setEffectType] = useState<'none' | 'higgs' | 'tachyon'>('none');
  const [effectIntensity, setEffectIntensity] = useState(0.0);
  const [effectCenter, setEffectCenter] = useState<[number, number]>([0.5, 0.5]);
  const [bloomEnabled, setBloomEnabled] = useState(true);
  const [flash, setFlash] = useState(false);

  // 注入フィードバック State
  const [injectionMarks, setInjectionMarks] = useState<InjectionMark[]>([]);
  const injectionMarksRef = useRef<InjectionMark[]>([]);
  const injectionAnimRef = useRef<number | null>(null);

  // パーティクルバースト State
  interface Particle {
    id: number;
    x: number;
    y: number;
    vx: number;
    vy: number;
    life: number;
    color: string;
    size: number;
  }
  const [particles, setParticles] = useState<Particle[]>([]);
  const particleIdRef = useRef(0);

  // フローティングテキスト State
  interface FloatingText {
    id: number;
    x: number;
    y: number;
    text: string;
    color: string;
    life: number;
  }
  const [floatingTexts, setFloatingTexts] = useState<FloatingText[]>([]);

  const {
    loading,
    error,
    generation,
    tick,
    rewind,
    getRenderView,
    getCellDetail,
    injectElement,
    applyCrisis,
    getRarity,
    getActiveCellCount,
    getElementBalance,
    rollSubstance,
    getMutationBoost,
    ticksSinceMutation,

    serializeGenome,
    getRarityProgress,
    getLastTickEvents,
  } = useBiomeEngine({ seed: currentSeed, paused });

  // 初回起動時のチュートリアル自動表示
  useEffect(() => {
    const done = localStorage.getItem('biome_tutorial_done');
    if (!done) {
      setShowTutorial(true);
    }
  }, []);

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

    // テスト・デモ用に中央部に多様性のある初期配置（C/N/P 混合）
    for (let y = 58; y <= 70; y++) {
      for (let x = 58; x <= 70; x++) {
        const elementIdx = (x + y) % 3; // C (0), N (1), P (2)
        injectElement(x, y, elementIdx, 4000);
      }
    }
    // 新しいシードのときはリザルト表示などをリセット
    setShowResult(false);
    setResultDismissed(false);
    setPaused(false);
  }, [loading, currentSeed, injectElement]);

  // 世代マイルストーン演出 (50, 100, 150世代でフラッシュ)
  const prevGenerationRef = useRef(generation);
  useEffect(() => {
    const prev = prevGenerationRef.current;
    prevGenerationRef.current = generation;
    // 世代がマイルストーンを「通過」したかどうかを検出
    for (const milestone of [50, 100, 150]) {
      if (prev < milestone && generation >= milestone) {
        setFlash(true);
        break;
      }
    }
  }, [generation]);

  // flash が true になったら必ず 150ms 後に解除（generation 変化でキャンセルされない）
  useEffect(() => {
    if (!flash) return;
    const timer = setTimeout(() => setFlash(false), 150);
    return () => clearTimeout(timer);
  }, [flash]);

  // 10世代ごとにレアリティ進捗とイベントを取得 (G1-16 修正: stale closure 対策)
  useEffect(() => {
    if (generation > 0 && generation % 10 === 0) {
      setRarityProgress(getRarityProgress());
      
      const events = getLastTickEvents();
      if (events && events.length > 0) {
        const formatted = events.map((e: any) => {
          let message = 'イベントが発生しました';
          let icon = '🔔';
          if (e.type === 'MorphologyChanged') {
            const morphNames = ['基本種', '生産者', '消費者', '捕食者', '分解者'];
            const toName = morphNames[e.to] || '未知の種';
            message = `細胞が ${toName} に進化しました！`;
            icon = '🧬';
          }
          return { type: e.type, message, icon };
        });
        setActiveEvents(prev => [...prev.slice(-3), ...formatted]);
      }
    }
  }, [generation, getRarityProgress, getLastTickEvents]);

  // 200 世代に達した時点で自動停止し、リザルトダイアログを表示
  useEffect(() => {
    if (generation >= 200 && !showResult && !showDendou && !resultDismissed) {
      setPaused(true);
      setShowResult(true);
    }
  }, [generation, showResult, showDendou, resultDismissed]);

  // ゲームの自動進行タイマー
  useEffect(() => {
    if (loading || paused) return;

    const interval = setInterval(() => {
      tick();
    }, speed);

    return () => clearInterval(interval);
  }, [loading, paused, speed, tick]);

  const elementIndexMap: Record<string, number> = {
    C: 0,
    N: 1,
    P: 2,
    H: 3,
    O: 4,
    S: 5,
    Fe: 6,
    Si: 7,
  };

  // 元素に対応するカラー
  const elementColorMap: Record<string, string> = {
    C: ThemeBridge.getUiElementColor('c'),
    N: ThemeBridge.getUiElementColor('n'),
    P: ThemeBridge.getUiElementColor('p'),
    H: ThemeBridge.getUiElementColor('h'),
    O: ThemeBridge.getUiElementColor('o'),
    S: ThemeBridge.getUiElementColor('s'),
    Fe: ThemeBridge.getUiElementColor('fe'),
    Si: ThemeBridge.getUiElementColor('si'),
  };

  // 注入リップルアニメーションの起動
  const startInjectionRipple = useCallback((cx: number, cy: number, elementIdx: number) => {
    const newMark: InjectionMark = { x: cx, y: cy, age: 0, elementIdx };
    injectionMarksRef.current = [...injectionMarksRef.current.slice(-3), newMark];
    setInjectionMarks([...injectionMarksRef.current]);

    // 既にアニメーション中でなければ起動
    if (!injectionAnimRef.current) {
      let lastTime = performance.now();
      const animateMarks = (now: number) => {
        const dt = (now - lastTime) / 1000;
        lastTime = now;

        injectionMarksRef.current = injectionMarksRef.current
          .map(m => ({ ...m, age: m.age + dt * 0.67 })) // ~1.5秒で age=1.0
          .filter(m => m.age < 1.0);

        setInjectionMarks([...injectionMarksRef.current]);

        if (injectionMarksRef.current.length > 0) {
          injectionAnimRef.current = requestAnimationFrame(animateMarks);
        } else {
          injectionAnimRef.current = null;
        }
      };
      injectionAnimRef.current = requestAnimationFrame(animateMarks);
    }
  }, []);

  // パーティクルバースト生成
  const spawnParticles = useCallback((gridX: number, gridY: number, color: string) => {
    // グリッド座標からキャンバス上のピクセル座標に変換
    // canvasToGridCoords で gridY = (1 - py/height) * 128 としてY反転されているため、
    // ピクセルに戻す際は再反転: py = (1 - gridY/128) * 512
    const px = (gridX / 128) * 512;
    const py = (1 - gridY / 128) * 512;
    const count = 12;
    const newParticles: Particle[] = [];
    for (let i = 0; i < count; i++) {
      const angle = (Math.PI * 2 * i) / count + (Math.random() - 0.5) * 0.5;
      const speed = 40 + Math.random() * 60;
      newParticles.push({
        id: particleIdRef.current++,
        x: px,
        y: py,
        vx: Math.cos(angle) * speed,
        vy: Math.sin(angle) * speed,
        life: 1.0,
        color,
        size: 2 + Math.random() * 3,
      });
    }
    setParticles(prev => [...prev, ...newParticles]);
  }, []);

  // フローティングテキスト生成
  const spawnFloatingText = useCallback((gridX: number, gridY: number, text: string, color: string) => {
    // Y軸反転: gridY はWebGL座標系なのでピクセルに変換時に反転
    const px = (gridX / 128) * 512;
    const py = (1 - gridY / 128) * 512;
    setFloatingTexts(prev => [...prev, {
      id: Date.now() + Math.random(),
      x: px,
      y: py,
      text,
      color,
      life: 1.0,
    }]);
  }, []);

  // パーティクル & テキストアニメーション
  useEffect(() => {
    if (particles.length === 0 && floatingTexts.length === 0) return;
    let lastTime = performance.now();
    let animId: number;

    const animate = (now: number) => {
      const dt = (now - lastTime) / 1000;
      lastTime = now;

      setParticles(prev =>
        prev
          .map(p => ({
            ...p,
            x: p.x + p.vx * dt,
            y: p.y + p.vy * dt,
            vy: p.vy + 30 * dt, // 重力
            life: p.life - dt * 1.5,
            size: p.size * (1 - dt * 0.5),
          }))
          .filter(p => p.life > 0)
      );

      setFloatingTexts(prev =>
        prev
          .map(t => ({ ...t, y: t.y - 40 * dt, life: t.life - dt * 1.2 }))
          .filter(t => t.life > 0)
      );

      animId = requestAnimationFrame(animate);
    };
    animId = requestAnimationFrame(animate);
    return () => cancelAnimationFrame(animId);
  }, [particles.length > 0 || floatingTexts.length > 0]);

  // キャンバスクリック時の元素注入または災害発生
  const handleCanvasClick = useCallback((coord: { x: number; y: number }) => {
    if (loading) return;
    if (selectedElement) {
      const idx = elementIndexMap[selectedElement];
      if (idx !== undefined) {
        // 周囲5x5に大量注入（少量では全体比率に影響しないため、体感可能な量を投入）
        for (let dy = -2; dy <= 2; dy++) {
          for (let dx = -2; dx <= 2; dx++) {
            const tx = coord.x + dx;
            const ty = coord.y + dy;
            if (tx >= 0 && tx < 128 && ty >= 0 && ty < 128) {
              injectElement(tx, ty, idx, 15000);
            }
          }
        }

        // 注入後に数 tick 回して拡散・反応を即座に反映
        // （一時停止中でも操作の効果が見えるようにする）
        for (let i = 0; i < 5; i++) {
          tick();
        }

        // 画面シェイク & コンボ判定
        setShakeOffset({ x: (Math.random() - 0.5) * 8, y: (Math.random() - 0.5) * 8 });
        setTimeout(() => setShakeOffset({ x: 0, y: 0 }), 100);

        const now = Date.now();
        let nextCombo = 1;
        if (now - lastInjectTimeRef.current < 800) {
          nextCombo = comboCount + 1;
        }
        setComboCount(nextCombo);
        lastInjectTimeRef.current = now;

        // 注入フィードバック: リップル + パーティクル + テキスト
        startInjectionRipple(coord.x, coord.y, idx);
        const color = elementColorMap[selectedElement] || '#00f0ff';
        spawnParticles(coord.x, coord.y, color);
        const comboText = nextCombo > 1 ? ` COMBO x${nextCombo}!` : '';
        spawnFloatingText(coord.x, coord.y, `+${selectedElement} ×25${comboText}`, color);
      }
    } else if (selectedCrisis) {
      const type = selectedCrisis === 'Meteor' ? 'meteor' : 'ice_age';
      applyCrisis(type, coord.x, coord.y);

      // エフェクトトリガー
      setEffectType(selectedCrisis === 'Meteor' ? 'higgs' : 'tachyon');
      setEffectCenter([coord.x / 128, coord.y / 128]);

      const maxIntensity = selectedCrisis === 'Meteor' ? 1.0 : 0.95;
      setEffectIntensity(maxIntensity);

      // 2秒間のフェードアウトアニメーション
      let start: number | null = null;
      const duration = 2000;
      const animate = (timestamp: number) => {
        if (!start) start = timestamp;
        const progress = timestamp - start;
        const remaining = Math.max(0, maxIntensity * (1 - progress / duration));
        setEffectIntensity(remaining);
        if (progress < duration) {
          requestAnimationFrame(animate);
        } else {
          setEffectType('none');
          setEffectIntensity(0);
        }
      };
      requestAnimationFrame(animate);

      // 災害は1発モノなので発動後リセット
      setSelectedCrisis(null);
    }
  }, [loading, selectedElement, selectedCrisis, injectElement, applyCrisis, tick, getElementBalance, getActiveCellCount, startInjectionRipple, spawnParticles, spawnFloatingText]);

  // マウスホバーコールバック
  const handleHover = useCallback((coord: { x: number; y: number } | null) => {
    if (loading) return;
    setHoverCell(coord);
    if (coord) {
      const detail = getCellDetail(coord.x, coord.y);
      setHoverData(detail);
    } else {
      setHoverData(null);
    }
  }, [loading, getCellDetail]);

  const handleInjectElement = (el: string) => {
    if (loading) return;
    const idx = elementIndexMap[el];
    if (idx !== undefined) {
      for (let y = 60; y <= 68; y++) {
        for (let x = 60; x <= 68; x++) {
          injectElement(x, y, idx, 15000);
        }
      }
      // 注入後に拡散を即座に実行
      for (let i = 0; i < 5; i++) {
        tick();
      }

      // 画面シェイク & コンボ判定
      setShakeOffset({ x: (Math.random() - 0.5) * 8, y: (Math.random() - 0.5) * 8 });
      setTimeout(() => setShakeOffset({ x: 0, y: 0 }), 100);

      const now = Date.now();
      let nextCombo = 1;
      if (now - lastInjectTimeRef.current < 800) {
        nextCombo = comboCount + 1;
      }
      setComboCount(nextCombo);
      lastInjectTimeRef.current = now;

      // ボタン経由の注入にもフィードバック追加
      const color = elementColorMap[el] || '#00f0ff';
      startInjectionRipple(64, 64, idx);
      spawnParticles(64, 64, color);
      const comboText = nextCombo > 1 ? ` COMBO x${nextCombo}!` : '';
      spawnFloatingText(64, 64, `+${el} ×81${comboText}`, color);
    }
  };

  const handleTriggerCrisis = (crisis: string) => {
    if (loading) return;
    const type = crisis === 'Meteor' ? 'meteor' : 'ice_age';
    applyCrisis(type, 64, 64);
  };

  // ランダム注入の処理
  const handleRollSubstance = () => {
    if (loading) return;
    const rolled = rollSubstance();
    // 特殊演出用にフラッシュをトリガー
    setFlash(true);
    setTimeout(() => setFlash(false), 100);
    console.log("Rolled Substance: ", rolled);
  };

  const handleSaveSpecimen = async () => {
    if (loading || !isAuthenticated()) return;
    try {
      const token = localStorage.getItem('jwt_token');
      const runId = crypto.randomUUID();
      const agentId = crypto.randomUUID();

      // WASM から現在の元素バランスを取得
      const balanceRaw = getElementBalance();
      const balance = {
        C: balanceRaw[0] || 0, N: balanceRaw[1] || 0,
        P: balanceRaw[2] || 0, H: balanceRaw[3] || 0,
        O: balanceRaw[4] || 0, S: balanceRaw[5] || 0,
        Fe: balanceRaw[6] || 0, Si: balanceRaw[7] || 0,
      };

      // 1. Run 情報を送信
      const runPayload = {
        id: runId,
        agent_id: agentId,
        generation: generation,
        score: getActiveCellCount() * 1.5,
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
        rarity: rarity.toLowerCase(),
        element_balance: JSON.stringify(balance),
        active_cell_count: getActiveCellCount(),
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

  const handleLoadDendou = (_id: string) => {
    setShowDendou(false);
    setCurrentSeed(Math.floor(Math.random() * 1000000));
  };

  if (loading) {
    return (
      <div style={{
        display: 'flex',
        flexDirection: 'column',
        justifyContent: 'center',
        alignItems: 'center',
        height: '25rem',
        color: 'var(--white-100)',
        fontFamily: 'var(--font-main)',
        gap: '1rem'
      }}>
        {error ? (
          <>
            <span style={{ color: 'var(--danger, #ff4444)' }}>⚠ Biome Engine の読み込みに失敗しました</span>
            <span style={{ fontSize: '0.85rem', color: 'var(--white-60, #999)', maxWidth: '400px', textAlign: 'center' }}>{error}</span>
            <button
              onClick={() => window.location.reload()}
              style={{
                marginTop: '0.5rem',
                padding: '0.5rem 1.5rem',
                background: 'var(--primary, #6366f1)',
                color: '#fff',
                border: 'none',
                borderRadius: '0.5rem',
                cursor: 'pointer',
                fontFamily: 'var(--font-main)'
              }}
            >
              リロード
            </button>
          </>
        ) : (
          'Loading Biome Engine...'
        )}
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

  const renderView = getRenderView();

  return (
    <div style={{
      display: 'flex',
      gap: 'var(--layout-panel-gap)',
      padding: 'var(--layout-panel-gap)',
      background: 'var(--bg-primary)',
      borderRadius: 'var(--radius-md)',
      color: 'var(--white-100)',
      fontFamily: 'var(--font-main)',
      position: 'relative'
    }}>
      {/* メインレンダラー */}
      <div style={{ flex: '1', display: 'flex', justifyContent: 'center', alignItems: 'center' }}>
        {/* 512x512 固定サイズコンテナ: キャンバスとオーバーレイの座標基準を統一 */}
        <div style={{
          width: '512px',
          height: '512px',
          position: 'relative',
          flexShrink: 0,
          transform: `translate(${shakeOffset.x}px, ${shakeOffset.y}px)`,
          transition: 'transform 0.05s ease-out'
        }}>
        <BiomeCanvas
          width={512}
          height={512}
          renderView={renderView}
          rarity={rarityIndex}
          effectType={effectType}
          effectIntensity={effectIntensity}
          effectCenter={effectCenter}
          onClick={handleCanvasClick}
          onHover={handleHover}
          bloomEnabled={bloomEnabled}
          injectionMarks={injectionMarks}
        />
        
        {/* マイルストーンフラッシュ */}
        <div style={{
          position: 'absolute',
          top: 0,
          left: 0,
          width: '512px',
          height: '512px',
          borderRadius: 'var(--radius-md)',
          background: 'radial-gradient(circle at center, rgba(0, 240, 255, 0.12) 0%, transparent 70%)',
          boxShadow: flash ? '0 0 30px rgba(0, 240, 255, 0.3), inset 0 0 30px rgba(0, 240, 255, 0.1)' : 'none',
          opacity: flash ? 1 : 0,
          pointerEvents: 'none',
          transition: flash ? 'none' : 'opacity 0.3s ease-out',
          zIndex: 10
        }} />

        {/* パーティクルバースト */}
        {particles.map(p => (
          <div
            key={p.id}
            style={{
              position: 'absolute',
              left: `${p.x}px`,
              top: `${p.y}px`,
              width: `${p.size}px`,
              height: `${p.size}px`,
              borderRadius: '50%',
              background: p.color,
              boxShadow: `0 0 ${p.size * 2}px ${p.color}`,
              opacity: Math.max(0, p.life),
              pointerEvents: 'none',
              transform: 'translate(-50%, -50%)',
              zIndex: 15
            }}
          />
        ))}

        {/* フローティングテキスト */}
        {floatingTexts.map(t => (
          <div
            key={t.id}
            style={{
              position: 'absolute',
              left: `${t.x}px`,
              top: `${t.y}px`,
              color: t.color,
              fontFamily: 'var(--font-main)',
              fontWeight: 'bold',
              fontSize: '1rem',
              textShadow: `0 0 8px ${t.color}, 0 0 16px ${t.color}`,
              opacity: Math.max(0, t.life),
              pointerEvents: 'none',
              transform: 'translate(-50%, -50%)',
              zIndex: 15,
              whiteSpace: 'nowrap'
            }}
          >
            {t.text}
          </div>
        ))}

        {/* ホバー詳細ツールチップ */}
        {hoverCell && hoverData && (
          <div style={{
            position: 'absolute',
            bottom: '16px',
            left: '16px',
            background: 'rgba(10, 15, 30, 0.85)',
            border: '1px solid var(--accent-cyan, #00f0ff)',
            borderRadius: 'var(--radius-sm)',
            padding: '8px 12px',
            fontSize: '0.8rem',
            color: 'var(--white-90)',
            pointerEvents: 'none',
            zIndex: 20,
            backdropFilter: 'blur(8px)',
            display: 'flex',
            flexDirection: 'column',
            gap: '4px',
            boxShadow: '0 4px 12px rgba(0,0,0,0.5)'
          }}>
            <div style={{ fontWeight: 'bold', color: 'var(--accent-cyan)', display: 'flex', justifyContent: 'space-between', gap: '20px' }}>
              <span>座標 ({hoverCell.x}, {hoverCell.y})</span>
              <span>{hoverData.active ? '🟢 生存' : '⚫ 休眠'}</span>
            </div>
            {hoverData.active && (
              <>
                <div>形態: {
                  hoverData.morphology === 1 ? '🌲 生産者 (Producer)' :
                  hoverData.morphology === 2 ? '🌊 消費者 (Consumer)' :
                  hoverData.morphology === 3 ? '⚔️ 捕食者 (Predator)' : '🍂 分解者 (Basic)'
                }</div>
                <div>エネルギー: {hoverData.energy}</div>
                <div style={{ display: 'grid', gridTemplateColumns: 'repeat(4, 1fr)', gap: '6px', marginTop: '2px', fontSize: '0.75rem' }}>
                  <span>C: {hoverData.elements[0]}</span>
                  <span>N: {hoverData.elements[1]}</span>
                  <span>P: {hoverData.elements[2]}</span>
                  <span>H: {hoverData.elements[3]}</span>
                </div>
                {hoverData.is_frozen && (
                  <div style={{ color: '#00f0ff', fontSize: '0.75rem', marginTop: '2px' }}>❄️ 凍結状態</div>
                )}
              </>
            )}
          </div>
        )}
        </div>{/* 512x512 固定コンテナ終了 */}
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
          rarityProgress={rarityProgress}
        />
        
        {/* シミュレーションサイクル選択速度パネル */}
        <CycleSelect
          speed={speed}
          onSpeedChange={setSpeed}
          paused={paused}
          onTogglePause={() => setPaused(!paused)}
          bloomEnabled={bloomEnabled}
          onToggleBloom={() => setBloomEnabled(!bloomEnabled)}
        />

        <BiomeControls
          selectedElement={selectedElement}
          onSelectElement={(el) => {
            setSelectedElement(el);
            setSelectedCrisis(null);
          }}
          selectedCrisis={selectedCrisis}
          onSelectCrisis={(cr) => {
            setSelectedCrisis(cr);
            setSelectedElement(null);
          }}
          onInjectElement={handleInjectElement}
          onTriggerCrisis={handleTriggerCrisis}
          onRollSubstance={handleRollSubstance}
          onRewind={() => rewind(20)}
          paused={paused}
          onTogglePause={() => setPaused(!paused)}
          onNewSeed={() => {
            setCurrentSeed(Math.floor(Math.random() * 1000000));
            setShowDendou(false);
          }}
          onShowTutorial={() => setShowTutorial(true)}
        />

        {/* 別ウインドウで開くボタン */}
        {!standalone && (
          <button
            onClick={() => window.open('/biome-popup.html', 'Biome Game', 'width=1100,height=800,menubar=no,toolbar=no,location=no,status=no')}
            style={{
              background: 'var(--white-05)',
              border: '1px solid var(--white-10)',
              borderRadius: 'var(--radius-sm)',
              color: 'var(--white-100)',
              padding: '8px',
              cursor: 'pointer',
              fontWeight: '600',
              transition: 'background 0.2s'
            }}
            onMouseEnter={(e) => e.currentTarget.style.background = 'var(--white-10)'}
            onMouseLeave={(e) => e.currentTarget.style.background = 'var(--white-05)'}
            data-testid="control-open-popup"
          >
            🪟 別ウインドウで開く
          </button>
        )}

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
            fontWeight: '600',
            transition: 'background 0.2s'
          }}
          onMouseEnter={(e) => e.currentTarget.style.background = 'var(--white-10)'}
          onMouseLeave={(e) => e.currentTarget.style.background = 'var(--white-05)'}
        >
          {showDendou ? 'シミュレーションに戻る' : '殿堂入り標本'}
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
            setResultDismissed(true);
          }}
          elementBalance={elementBalance}
          activeCellCount={getActiveCellCount()}
        />
      )}

      {showTutorial && (
        <BiomeTutorial onClose={() => setShowTutorial(false)} />
      )}

      <BiomeEventToast
        events={activeEvents}
        onDismiss={(idx) => setActiveEvents(prev => prev.filter((_, i) => i !== idx))}
      />
    </div>
  );
}

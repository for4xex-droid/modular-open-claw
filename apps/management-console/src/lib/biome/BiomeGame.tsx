/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

import { useState, useEffect, useCallback, useRef } from 'react';
import { useBiomeEngine } from '../../hooks/useBiomeEngine';
import { useTranslation } from '../../i18n';
import { BiomeHUD } from './BiomeHUD';
import { BiomeControls } from './BiomeControls';
import { BiomeCanvas } from './BiomeCanvas';
import type { InjectionMark } from './biomeTypes';
import { CELL_COUNT, RENDER_STRIDE, MORPH_COUNT, MORPH_NAMES } from './biomeTypes';
import { CycleSelect } from './CycleSelect';
import { BiomeResult } from './BiomeResult';
import { BiomeDendou, Specimen } from './BiomeDendou';
import { BiomeTutorial } from './BiomeTutorial';
import { BiomeEventToast } from './BiomeEventToast';
import { fetchBiomeSpecimens, saveBiomeRun, saveBiomeSpecimen } from './biomeApi';
import { isAuthenticated } from '../../lib/auth';
import { cssVar } from '../../utils/cssVar';

export interface BiomeGameProps {
  seed?: number;
  standalone?: boolean;
}

export function BiomeGame({ seed, standalone }: BiomeGameProps) {
  const { t } = useTranslation();
  const [paused, setPaused] = useState(false);
  const [currentSeed, setCurrentSeed] = useState(() => seed ?? Math.floor(Math.random() * 1000000));
  const [speed, setSpeed] = useState(100); // ms (100 = 1x, 50 = 2x, 20 = 5x, 10 = 10x)
  const [showResult, setShowResult] = useState(false);
  const [resultDismissed, setResultDismissed] = useState(false);
  const [showDendou, setShowDendou] = useState(false);
  const [dendouList, setDendouList] = useState<Specimen[]>([]);
  const [showTutorial, setShowTutorial] = useState(false);

  // インタラクション用 State
  const [seedMode, setSeedMode] = useState(true);
  const [hoverCell, setHoverCell] = useState<{ x: number; y: number } | null>(null);
  const [hoverData, setHoverData] = useState<any>(null);

  const [clickPulse, setClickPulse] = useState(false);
  const [comboCount, setComboCount] = useState(0);
  const [activeEvents, setActiveEvents] = useState<any[]>([]);
  const [rarityProgress, setRarityProgress] = useState<any>(null);
  const lastInjectTimeRef = useRef(0);

  // エフェクト & Bloom用 State
  const [effectType] = useState<'none' | 'higgs' | 'tachyon'>('none');
  const [effectIntensity] = useState(0.0);
  const [effectCenter] = useState<[number, number]>([0.5, 0.5]);
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
    injectBrush,
    getRarity,
    getActiveCellCount,
    getElementBalance,
    getRarityProgress,
    getLastTickEvents,
    leniaMu,
    leniaSigma,
    setLeniaParams,
    paintEnv,
    clearEnv,
  } = useBiomeEngine({ seed: currentSeed, paused });

  // 環境ペン: null=種まき, それ以外は地形を描く（1=壁 2=養分 3=毒）
  const [envPen, setEnvPen] = useState<null | 1 | 2 | 3>(null);

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
      const data = await fetchBiomeSpecimens();
        setDendouList(data);
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

  // 新しいシードのときはリザルト表示などをリセット
  useEffect(() => {
    if (loading) return;
    setShowResult(false);
    setResultDismissed(false);
    setPaused(false);
  }, [loading, currentSeed]);

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
          } else if (e.type === 'PrismaticBorn') {
            message = t('biome.prismaticBorn', { x: e.x, y: e.y });
            icon = '✨';
          }
          return { type: e.type, message, icon };
        });
        setActiveEvents(prev => [...prev.slice(-3), ...formatted]);
      }
    }
  }, [generation, getRarityProgress, getLastTickEvents]);

  const countMorphologyDistribution = useCallback((view: Float32Array): Record<string, number> => {
    const counts: Record<string, number> = {};
    for (let m = 0; m < MORPH_COUNT; m++) {
      counts[MORPH_NAMES[m]] = 0;
    }
    for (let i = 0; i < CELL_COUNT; i++) {
      const offset = i * RENDER_STRIDE;
      const active = view[offset + 2];
      if (active < 0.5) continue;
      const morph = Math.floor(view[offset + 3]);
      if (morph >= 0 && morph < MORPH_COUNT) {
        counts[MORPH_NAMES[morph]]++;
      }
    }
    return counts;
  }, []);

  // 200 世代に達した時点で自動停止し、リザルトダイアログを表示
  useEffect(() => {
    if (generation >= 200 && !showResult && !showDendou && !resultDismissed) {
      setPaused(true);
      setShowResult(true);
    }
  }, [generation, showResult, showDendou, resultDismissed]);

  // ゲームの自動進行タイマー
  // 世代レート = 1000/speed を維持しつつ、interval は speed の整数倍で
  // 50ms 以上に丸める（高速時 ~20Hz 更新でカクツキ軽減、postMessage 上限 20/s）
  useEffect(() => {
    if (loading || paused) return;

    const intervalMs = speed * Math.max(1, Math.ceil(50 / speed));
    const tickCount = Math.max(1, Math.round(intervalMs / speed));

    const interval = setInterval(() => {
      tick(tickCount);
    }, intervalMs);

    return () => clearInterval(interval);
  }, [loading, paused, speed, tick]);

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

  // キャンバスクリック時の種まき
  const handleCanvasClick = useCallback((coord: { x: number; y: number }) => {
    if (loading) return;

    // 環境ペンが選択されていれば地形を描く（種まきではなく因果を与える操作）
    if (envPen !== null) {
      paintEnv(coord.x, coord.y, 4, envPen);
      setClickPulse(true);
      setTimeout(() => setClickPulse(false), 150);
      const penColor = envPen === 1 ? cssVar('--biome-element-fe') : envPen === 2 ? cssVar('--accent-emerald') : cssVar('--accent-rose');
      const penLabel = envPen === 1 ? '🧱 壁' : envPen === 2 ? '🌿 養分' : '☠️ 毒';
      startInjectionRipple(coord.x, coord.y, 0);
      spawnFloatingText(coord.x, coord.y, penLabel, penColor);
      return;
    }

    if (!seedMode) return;

    injectBrush(coord.x, coord.y, 3, 0, 20000);

    setClickPulse(true);
    setTimeout(() => setClickPulse(false), 150);

    const now = Date.now();
    let nextCombo = 1;
    if (now - lastInjectTimeRef.current < 800) {
      nextCombo = comboCount + 1;
    }
    setComboCount(nextCombo);
    lastInjectTimeRef.current = now;

    startInjectionRipple(coord.x, coord.y, 0);
    const color = cssVar('--accent-cyan');
    spawnParticles(coord.x, coord.y, color);
    const comboText = nextCombo > 1 ? ` COMBO x${nextCombo}!` : '';
    spawnFloatingText(coord.x, coord.y, `🌱 種まき${comboText}`, color);
  }, [loading, seedMode, envPen, paintEnv, injectBrush, comboCount, startInjectionRipple, spawnParticles, spawnFloatingText]);

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

  const handleSaveSpecimen = async () => {
    if (loading || !isAuthenticated()) return;
    try {
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

      await saveBiomeRun(runPayload);

      // 2. Lenia 種パラメータを genome_data に格納
      const progress = getRarityProgress();
      const genomeStr = JSON.stringify({
        mu: leniaMu,
        sigma: leniaSigma,
        species_hash: progress?.species_hash ?? 0,
        mass: progress?.mass ?? 0,
        locomotion: progress?.locomotion ?? 0,
        longevity: progress?.longevity ?? 0,
      });
      const morphologyDistribution = countMorphologyDistribution(getRenderView());
      const rarityLabels = ['Common', 'Uncommon', 'Rare', 'Epic', 'Legendary'];
      const savedRarity = (rarityLabels[getRarity()] || 'Common').toLowerCase();

      // 3. Specimen 情報を送信
      const specimenPayload = {
        id: crypto.randomUUID(),
        run_id: runId,
        specimen_name: `Species_${currentSeed}`,
        genome_data: genomeStr,
        rarity: savedRarity,
        element_balance: JSON.stringify(balance),
        morphology_distribution: JSON.stringify(morphologyDistribution),
        active_cell_count: getActiveCellCount(),
      };

      await saveBiomeSpecimen(specimenPayload);
      setShowResult(false);
      setShowDendou(true);
      fetchDendouList();
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
            <span style={{ color: 'var(--accent-rose)' }}>⚠ Biome Engine の読み込みに失敗しました</span>
            <span style={{ fontSize: '0.85rem', color: 'var(--text-muted)', maxWidth: '400px', textAlign: 'center' }}>{error}</span>
            <button
              onClick={() => window.location.reload()}
              style={{
                marginTop: '0.5rem',
                padding: '0.5rem 1.5rem',
                background: 'var(--accent-purple)',
                color: 'var(--white-100)',
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
  const morphologyDistribution = countMorphologyDistribution(renderView);
  const structureBonus = (rarityProgress?.symmetry_score ?? 0) >= 0.65;

  return (
    <div style={{
      display: 'flex',
      justifyContent: 'center',
      alignItems: 'flex-start',
      gap: 'var(--layout-panel-gap)',
      padding: 'var(--layout-panel-gap)',
      background: 'var(--bg-primary)',
      borderRadius: 'var(--radius-md)',
      color: 'var(--white-100)',
      fontFamily: 'var(--font-main)',
      position: 'relative',
      width: '100%',
      boxSizing: 'border-box'
    }}>
      {/* 1. 左カラム: ステータスHUD & サイクル速度設定 */}
      <div style={{ 
        width: '270px', 
        display: 'flex', 
        flexDirection: 'column', 
        gap: 'var(--layout-panel-gap)',
        flexShrink: 0
      }}>
        <BiomeHUD
          generation={generation}
          rarity={rarity}
          activeCellCount={getActiveCellCount()}
          rarityProgress={rarityProgress}
        />
        <CycleSelect
          speed={speed}
          onSpeedChange={setSpeed}
          paused={paused}
          onTogglePause={() => setPaused(!paused)}
          bloomEnabled={bloomEnabled}
          onToggleBloom={() => setBloomEnabled(!bloomEnabled)}
        />
      </div>

      {/* 2. 中央カラム: メイン3Dキャンバス (512x512) */}
      <div style={{ 
        display: 'flex', 
        justifyContent: 'center', 
        alignItems: 'center',
        flexShrink: 0
      }}>
        <div style={{
          width: '512px',
          height: '512px',
          position: 'relative',
          flexShrink: 0,
          boxShadow: clickPulse ? '0 0 24px 4px var(--accent-cyan-70)' : '0 0 0 0 var(--accent-cyan-05)',
          transition: 'box-shadow 0.15s ease-out'
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
            structureBonus={structureBonus}
            injectionMarks={injectionMarks}
            dragPaint={envPen !== null}
          />
          
          {/* マイルストーンフラッシュ */}
          <div style={{
            position: 'absolute',
            top: 0,
            left: 0,
            width: '512px',
            height: '512px',
            borderRadius: 'var(--radius-md)',
            background: 'radial-gradient(circle at center, var(--accent-cyan-10) 0%, transparent 70%)',
            boxShadow: flash ? '0 0 30px var(--accent-cyan-30), inset 0 0 30px var(--accent-cyan-10)' : 'none',
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
              background: 'var(--bg-deep-glass)',
              border: '1px solid var(--accent-cyan)',
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
              boxShadow: '0 4px 12px var(--black-50)'
            }}>
              <div style={{ fontWeight: 'bold', color: 'var(--accent-cyan)', display: 'flex', justifyContent: 'space-between', gap: '20px' }}>
                <span>座標 ({hoverCell.x}, {hoverCell.y})</span>
                <span>{hoverData.active ? '🟢 生存' : '⚫ 休眠'}</span>
              </div>
              {hoverData.active && (
                <>
                  <div>
                    場の強度: R {(hoverData.elements[0] / 655.35).toFixed(0)}% / G{' '}
                    {(hoverData.elements[1] / 655.35).toFixed(0)}% / B{' '}
                    {(hoverData.elements[2] / 655.35).toFixed(0)}%
                  </div>
                  {hoverData.is_frozen && (
                    <div style={{ color: 'var(--accent-cyan)', fontSize: '0.75rem', marginTop: '2px' }}>❄️ 凍結状態</div>
                  )}
                </>
              )}
            </div>
          )}
        </div>
      </div>

      {/* 3. 右カラム: 元素・災害コントロール & 各種アクション */}
      <div style={{ 
        width: '270px', 
        display: 'flex', 
        flexDirection: 'column', 
        gap: 'var(--layout-panel-gap)',
        flexShrink: 0
      }}>
        <BiomeControls
          seedMode={seedMode}
          onToggleSeedMode={() => setSeedMode(!seedMode)}
          leniaMu={leniaMu}
          leniaSigma={leniaSigma}
          onLeniaMuChange={(v) => setLeniaParams(v, leniaSigma)}
          onLeniaSigmaChange={(v) => setLeniaParams(leniaMu, v)}
          onShowCatalog={() => setShowDendou(true)}
          onRewind={() => rewind(20)}
          paused={paused}
          onTogglePause={() => setPaused(!paused)}
          onNewSeed={() => {
            setCurrentSeed(Math.floor(Math.random() * 1000000));
            setShowDendou(false);
          }}
          onShowTutorial={() => setShowTutorial(true)}
        />

        {/* 環境ペン: プレイヤーが地形を描いて生命の展開を変える */}
        <div style={{
          background: 'var(--white-05)',
          border: '1px solid var(--white-10)',
          borderRadius: 'var(--radius-sm)',
          padding: '10px',
          display: 'flex',
          flexDirection: 'column',
          gap: '6px',
        }}>
          <div style={{ fontSize: '12px', color: 'var(--white-60)', marginBottom: '2px' }}>
            🖐 環境ペン（地形を描く）
          </div>
          <div style={{ display: 'flex', gap: '6px', flexWrap: 'wrap' }}>
            {([
              { key: null, label: '🌱 種まき', color: 'var(--accent-cyan)' },
              { key: 1 as const, label: '🧱 壁', color: 'var(--biome-element-fe)' },
              { key: 2 as const, label: '🌿 養分', color: 'var(--accent-emerald)' },
              { key: 3 as const, label: '☠️ 毒', color: 'var(--accent-rose)' },
            ]).map((pen) => (
              <button
                key={String(pen.key)}
                onClick={() => setEnvPen(pen.key)}
                style={{
                  flex: '1 1 40%',
                  background: envPen === pen.key ? pen.color : 'var(--white-05)',
                  border: `1px solid ${envPen === pen.key ? pen.color : 'var(--white-10)'}`,
                  borderRadius: 'var(--radius-sm)',
                  color: envPen === pen.key ? 'var(--text-inverse)' : 'var(--white-100)',
                  padding: '8px 4px',
                  cursor: 'pointer',
                  fontSize: '12px',
                  fontWeight: envPen === pen.key ? 700 : 400,
                }}
              >
                {pen.label}
              </button>
            ))}
          </div>
          <button
            onClick={() => clearEnv()}
            style={{
              background: 'var(--white-05)',
              border: '1px solid var(--white-10)',
              borderRadius: 'var(--radius-sm)',
              color: 'var(--white-60)',
              padding: '6px',
              cursor: 'pointer',
              fontSize: '11px',
            }}
          >
            地形をすべて消去
          </button>
        </div>

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
              transition: 'background 0.2s',
              width: '100%'
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
            transition: 'background 0.2s',
            width: '100%'
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

      {/* 4. フルスクリーンダイアログ・トースト */}
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
          symmetryScore={rarityProgress?.symmetry_score}
          complexityScore={rarityProgress?.complexity_score}
          prismaticCells={rarityProgress?.prismatic_cells}
          morphologyDistribution={morphologyDistribution}
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

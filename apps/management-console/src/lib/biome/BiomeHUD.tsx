/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

import { useTranslation } from '../../i18n';

export interface BiomeHUDProps {
  generation: number;
  rarity: string;
  activeCellCount?: number;
  rarityProgress?: {
    active_cells: number;
    symmetry_score: number;
    complexity_score: number;
    mass?: number;
    locomotion?: number;
    longevity?: number;
    species_hash?: number;
    has_homeostasis?: boolean;
    condition_structure?: boolean;
  } | null;
}

export function BiomeHUD({
  generation,
  rarity,
  activeCellCount = 0,
  rarityProgress = null,
}: BiomeHUDProps) {
  const { t } = useTranslation();

  const getRarityDecor = (r: string) => {
    switch (r) {
      case 'Legendary':
        return {
          color: 'var(--biome-rarity-legendary)',
          border: 'var(--biome-rarity-legendary-border)',
          glow: 'var(--biome-rarity-legendary-glow)',
          emoji: '🔥',
          label: 'Legendary',
        };
      case 'Epic':
        return {
          color: 'var(--biome-rarity-epic)',
          border: 'var(--biome-rarity-epic-border)',
          glow: 'var(--biome-rarity-epic-glow)',
          emoji: '🔮',
          label: 'Epic',
        };
      case 'Rare':
        return {
          color: 'var(--biome-rarity-rare)',
          border: 'var(--biome-rarity-rare-border)',
          glow: 'var(--biome-rarity-rare-glow)',
          emoji: '💎',
          label: 'Rare',
        };
      case 'Uncommon':
        return {
          color: 'var(--biome-rarity-uncommon)',
          border: 'var(--biome-rarity-uncommon-border)',
          glow: 'var(--biome-rarity-uncommon-glow)',
          emoji: '🌟',
          label: 'Uncommon',
        };
      default:
        return {
          color: 'var(--biome-rarity-common)',
          border: 'var(--biome-rarity-common-border)',
          glow: 'var(--biome-rarity-common-glow)',
          emoji: '🍃',
          label: 'Common',
        };
    }
  };

  const decor = getRarityDecor(rarity);
  const mass = rarityProgress?.mass ?? 0;
  const locomotion = rarityProgress?.locomotion ?? 0;
  const longevity = rarityProgress?.longevity ?? 0;
  const symmetry = rarityProgress?.symmetry_score ?? 0;
  const progressPct = Math.min(100, (longevity / 200) * 100);

  return (
    <div
      style={{
        padding: 'var(--space-sm)',
        background: 'var(--bg-glass-heavy)',
        backdropFilter: 'blur(var(--blur-md))',
        border: '1px solid var(--border-glass)',
        borderRadius: 'var(--radius-md)',
        color: 'var(--white-100)',
        display: 'flex',
        flexDirection: 'column',
        gap: 'var(--space-xs)',
        fontFamily: 'var(--font-main)',
      }}
    >
      <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: 'var(--space-2xs)' }}>
        <div>
          <span style={{ fontSize: 'var(--font-size-2xs)', color: 'var(--text-muted)', fontWeight: '600' }}>現在の世代</span>
          <div
            style={{ fontSize: '2rem', fontWeight: 'bold', animation: 'neonGlow 2s infinite ease-in-out', color: 'var(--accent-cyan)' }}
            data-testid="biome-generation"
          >
            {generation}
          </div>
        </div>
        <div style={{ textAlign: 'right' }}>
          <span style={{ fontSize: 'var(--font-size-2xs)', color: 'var(--text-muted)', fontWeight: '600' }}>評価ランク</span>
          <div
            style={{
              fontSize: '0.85rem',
              fontWeight: 'bold',
              color: decor.color,
              background: 'var(--white-03)',
              padding: '4px 8px',
              borderRadius: 'var(--radius-sm)',
              marginTop: '4px',
              border: `1px solid ${decor.border}`,
              boxShadow: `0 0 8px ${decor.glow}`,
              display: 'flex',
              alignItems: 'center',
              gap: '4px',
            }}
            data-testid="biome-rarity"
          >
            <span>{decor.emoji}</span>
            <span>{decor.label}</span>
          </div>
        </div>
      </div>

      {rarityProgress && (
        <div
          style={{
            borderTop: '1px solid var(--border-glass)',
            paddingTop: 'var(--space-2xs)',
            display: 'flex',
            flexDirection: 'column',
            gap: '6px',
          }}
          data-testid="biome-lenia-scorecard"
        >
          <span style={{ fontSize: 'var(--font-size-2xs)', color: 'var(--text-muted)', fontWeight: '600' }}>
            Lenia 種スコア
          </span>
          <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '4px 12px', fontSize: '0.75rem' }}>
            <div>質量 mass</div>
            <div data-testid="biome-mass">{mass.toFixed(1)}</div>
            <div>移動 locomotion</div>
            <div>{locomotion.toFixed(3)}</div>
            <div>存続 longevity</div>
            <div>{longevity} tick</div>
            <div>{t('biome.hud.symmetry')}</div>
            <div>{symmetry.toFixed(2)}</div>
            <div>活性セル</div>
            <div data-testid="biome-active-cells">{activeCellCount || rarityProgress.active_cells}</div>
          </div>
          <div style={{ marginTop: '4px' }}>
            <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '0.7rem', color: 'var(--text-muted)', marginBottom: '2px' }}>
              <span>安定度 → Legendary</span>
              <span>{longevity}/200</span>
            </div>
            <div style={{ height: '6px', background: 'var(--white-05)', borderRadius: '3px', overflow: 'hidden' }}>
              <div
                style={{
                  height: '100%',
                  width: `${progressPct}%`,
                  background: 'linear-gradient(90deg, var(--accent-cyan), var(--accent-purple))',
                  borderRadius: '3px',
                  transition: 'width var(--speed-fast) ease',
                }}
              />
            </div>
          </div>
          {rarityProgress.species_hash !== undefined && (
            <div style={{ fontSize: '0.65rem', color: 'var(--text-muted)', marginTop: '2px' }}>
              種ID: {String(rarityProgress.species_hash).slice(0, 8)}…
            </div>
          )}
        </div>
      )}
    </div>
  );
}

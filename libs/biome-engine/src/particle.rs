/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
use crate::grid::BiomeGrid;
use crate::pattern;
use rand::Rng;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubstanceKind {
    None,
    Higgs,
    Tachyon,
}

const FIELD_ACTIVE_THRESHOLD: f32 = 0.1;
const HIGH_QUALITY_MASS: f32 = 200.0;
const HIGH_QUALITY_LONGEVITY: u32 = 30;
const HIGH_QUALITY_SYMMETRY: f32 = 0.65;

/// 超物質の発見を判定する（Lenia: 高質量×安定存続、または高対称で確率上昇）
pub fn roll_substance_discovery(grid: &BiomeGrid, rng: &mut impl Rng) -> SubstanceKind {
    let lenia = grid.lenia();
    let mass = lenia.mass;
    let longevity = lenia.longevity_ticks;
    let pattern = pattern::measure_field(lenia.channel(0), FIELD_ACTIVE_THRESHOLD);

    let high_quality = mass >= HIGH_QUALITY_MASS
        && (longevity >= HIGH_QUALITY_LONGEVITY || pattern.symmetry_score >= HIGH_QUALITY_SYMMETRY);

    // 高品質種なら 30 (3%)、それ以外なら 1 (0.1%)
    let threshold = if high_quality { 30 } else { 1 };
    let roll: u32 = rng.gen_range(0..1000);

    if roll < threshold {
        if rng.gen::<bool>() {
            SubstanceKind::Higgs
        } else {
            SubstanceKind::Tachyon
        }
    } else {
        SubstanceKind::None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::SmallRng;
    use rand::SeedableRng;

    #[test]
    fn test_high_quality_lenia_increases_discovery_rate() {
        // seed 3 は種ライブラリで高質量種（Pentahelicium solidus, mass≈560）を選ぶ。
        // 正典ソリトンは膨張せず質量が安定するため、高質量種でしきい値を満たす。
        let mut grid = BiomeGrid::new(3);

        // 安定した高質量ソリトンを 100 tick 育成
        for _ in 0..100 {
            grid.tick();
        }

        let lenia = grid.lenia();
        assert!(
            lenia.mass >= HIGH_QUALITY_MASS,
            "mass={} should meet threshold",
            lenia.mass
        );
        assert!(
            lenia.longevity_ticks >= HIGH_QUALITY_LONGEVITY,
            "longevity={} should meet threshold",
            lenia.longevity_ticks
        );

        let mut rng = SmallRng::seed_from_u64(100);
        let mut discoveries = 0;

        for _ in 0..1000 {
            if roll_substance_discovery(&grid, &mut rng) != SubstanceKind::None {
                discoveries += 1;
            }
        }

        assert!(
            discoveries > 5,
            "Should discover substances under high-quality Lenia condition, got {}",
            discoveries
        );
    }

    #[test]
    fn test_collapsed_field_low_discovery_rate() {
        let grid = BiomeGrid::new(42);
        // tick なし → mass あるが longevity 低、discovery threshold は mass+symmetry ベース

        let mut rng = SmallRng::seed_from_u64(200);
        let mut discoveries = 0;
        for _ in 0..1000 {
            if roll_substance_discovery(&grid, &mut rng) != SubstanceKind::None {
                discoveries += 1;
            }
        }
        // 低品質（symmetry 未達の可能性）では稀
        assert!(
            discoveries < 50,
            "Collapsed/low-quality should rarely discover, got {}",
            discoveries
        );
    }
}

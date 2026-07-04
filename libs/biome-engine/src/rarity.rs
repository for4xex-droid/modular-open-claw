/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
use crate::grid::BiomeGrid;
use crate::pattern;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BiomeRarity {
    Common,
    Uncommon,
    Rare,
    Epic,
    Legendary,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct RarityProgress {
    pub rarity: u8, // 0=Common 1=Uncommon 2=Rare 3=Epic 4=Legendary
    pub active_cells: u32,
    pub morphology_count: u8, // 共存する形態の種数 (0-5)
    pub has_homeostasis: bool,
    pub diversity_index: f32,
    /// フィールド名は HUD 互換のため維持。Lenia では mass ベース。
    pub condition_active_500: bool,
    pub condition_morph_3: bool,
    pub condition_morph_4: bool,
    pub condition_active_1000: bool,
    pub symmetry_score: f32,
    pub complexity_score: f32,
    pub cluster_count: u16,
    pub prismatic_cells: u16,
    pub condition_structure: bool,
    pub condition_prismatic: bool,
    /// Lenia 統計（Phase 3 追記）
    pub mass: f32,
    pub locomotion: f32,
    pub longevity: u32,
    pub species_hash: u64,
}

const FIELD_ACTIVE_THRESHOLD: f32 = 0.1;
const STABLE_LONGEVITY: u32 = 10;
const MASS_UNCOMMON: f32 = 50.0;
const MASS_RARE: f32 = 150.0;
const MASS_EPIC: f32 = 300.0;
const MASS_LEGENDARY: f32 = 500.0;
const LONGEVITY_RARE: u32 = 25;
const LONGEVITY_EPIC: u32 = 80;
const LONGEVITY_LEGENDARY: u32 = 200;
const LOCOMOTION_EPIC: f32 = 0.25;
const LOCOMOTION_LEGENDARY: f32 = 0.8;
const SYMMETRY_RARE: f32 = 0.50;
const SYMMETRY_EPIC: f32 = 0.65;
const SYMMETRY_LEGENDARY: f32 = 0.85;
/// 「生物的に局在している」と見なす外接矩形占有率の上限。
/// これを超える（＝一面に広がったテクスチャ）個体は Rare 止まりにし、
/// 放置で広がっただけの場が高レア化する穴（実機計測: 無操作 200 tick で Epic 到達）を塞ぐ。
const LOCALIZED_BBOX_MAX: f32 = 0.5;

/// Lenia 統計からレアリティ tier (0-4) を判定する。
///
/// `bbox_ratio` は活性領域の外接矩形占有率。Epic 以上は「局在した生物」であること
/// （`bbox_ratio < LOCALIZED_BBOX_MAX`）を必須とし、加えて実際に動いている
/// （locomotion）か強い対称/複雑性を持つことを要求する。
pub fn lenia_rarity_tier(
    mass: f32,
    locomotion: f32,
    longevity: u32,
    symmetry: f32,
    complexity: f32,
    bbox_ratio: f32,
) -> u8 {
    let stable = longevity >= STABLE_LONGEVITY && mass >= 30.0;
    // 局在＝散らばらず塊で存在する生物的な状態。広がったテクスチャを高レアから除外。
    let localized = bbox_ratio > 0.0 && bbox_ratio < LOCALIZED_BBOX_MAX;

    if stable
        && localized
        && mass >= MASS_LEGENDARY
        && longevity >= LONGEVITY_LEGENDARY
        && (symmetry >= SYMMETRY_LEGENDARY || locomotion >= LOCOMOTION_LEGENDARY)
    {
        4
    } else if stable
        && localized
        && mass >= MASS_EPIC
        && longevity >= LONGEVITY_EPIC
        && (locomotion >= LOCOMOTION_EPIC || symmetry >= SYMMETRY_EPIC || complexity >= 0.70)
    {
        3
    } else if stable
        && mass >= MASS_RARE
        && longevity >= LONGEVITY_RARE
        && symmetry >= SYMMETRY_RARE
    {
        2
    } else if stable && mass >= MASS_UNCOMMON {
        1
    } else {
        0
    }
}

/// グリッド全体の進化状態から詳細な進捗付きでレアリティを判定する
pub fn determine_rarity_with_progress(grid: &BiomeGrid) -> RarityProgress {
    let lenia = grid.lenia();
    let mass = lenia.mass;
    let locomotion = lenia.locomotion;
    let longevity = lenia.longevity_ticks;
    let species_hash = lenia.species_hash();

    let ch0 = lenia.channel(0);
    let pattern = pattern::measure_field(ch0, FIELD_ACTIVE_THRESHOLD);

    let mut active_cells = 0u32;
    for &v in ch0 {
        if v > FIELD_ACTIVE_THRESHOLD {
            active_cells += 1;
        }
    }

    let symmetry_score = pattern.symmetry_score;
    let complexity_score = pattern.complexity_score;
    let cluster_count = pattern.cluster_count;

    let rarity = lenia_rarity_tier(
        mass,
        locomotion,
        longevity,
        symmetry_score,
        complexity_score,
        pattern.bbox_ratio,
    );

    // HUD 互換フィールド（Lenia 指標へマッピング）
    let has_homeostasis = longevity >= STABLE_LONGEVITY && mass >= 30.0;
    let diversity_index = locomotion;
    let morphology_count = if has_homeostasis { 1 } else { 0 };
    let condition_active_500 = mass >= MASS_EPIC;
    let condition_morph_3 = longevity >= LONGEVITY_EPIC;
    let condition_morph_4 = longevity >= LONGEVITY_LEGENDARY;
    let condition_active_1000 = mass >= MASS_LEGENDARY;
    let condition_structure = symmetry_score >= SYMMETRY_EPIC
        || complexity_score >= 0.70
        || locomotion >= LOCOMOTION_EPIC;
    let prismatic_cells = 0u16;
    let condition_prismatic = locomotion >= LOCOMOTION_LEGENDARY;

    RarityProgress {
        rarity,
        active_cells,
        morphology_count,
        has_homeostasis,
        diversity_index,
        condition_active_500,
        condition_morph_3,
        condition_morph_4,
        condition_active_1000,
        symmetry_score,
        complexity_score,
        cluster_count,
        prismatic_cells,
        condition_structure,
        condition_prismatic,
        mass,
        locomotion,
        longevity,
        species_hash,
    }
}

/// グリッド全体の進化状態からレアリティを判定する
pub fn determine_rarity(grid: &BiomeGrid) -> BiomeRarity {
    let p = determine_rarity_with_progress(grid);
    match p.rarity {
        4 => BiomeRarity::Legendary,
        3 => BiomeRarity::Epic,
        2 => BiomeRarity::Rare,
        1 => BiomeRarity::Uncommon,
        _ => BiomeRarity::Common,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initial_grid_is_common() {
        let grid = BiomeGrid::new(42);
        let rarity = determine_rarity(&grid);
        assert_eq!(
            rarity,
            BiomeRarity::Common,
            "fresh seed has longevity=0 → Common"
        );
    }

    #[test]
    fn test_lenia_rarity_tier_legendary() {
        // 局在（bbox=0.1）した高質量・高対称・高移動 → Legendary
        assert_eq!(lenia_rarity_tier(600.0, 0.9, 250, 0.90, 0.5, 0.1), 4);
    }

    #[test]
    fn test_lenia_rarity_tier_epic_via_locomotion() {
        // 局在（bbox=0.15）した移動体 → Epic
        assert_eq!(lenia_rarity_tier(350.0, 0.4, 100, 0.40, 0.5, 0.15), 3);
    }

    #[test]
    fn test_lenia_rarity_tier_uncommon_only() {
        assert_eq!(lenia_rarity_tier(80.0, 0.0, 15, 0.3, 0.2, 0.2), 1);
    }

    #[test]
    fn test_collapsed_mass_is_common() {
        assert_eq!(lenia_rarity_tier(0.0, 0.0, 0, 0.0, 0.0, 0.0), 0);
        assert_eq!(lenia_rarity_tier(200.0, 0.5, 5, 0.8, 0.8, 0.1), 0);
    }

    #[test]
    fn test_spread_texture_capped_at_rare() {
        // Negative Test: Epic 相当の統計でも、一面に広がった（bbox=0.95）テクスチャは
        // 局在条件を満たさず Epic/Legendary に上がれない（放置膨張の高レア化を防止）。
        let spread = lenia_rarity_tier(600.0, 0.9, 250, 0.90, 0.9, 0.95);
        assert!(
            spread <= 2,
            "spread-out texture must not reach Epic+, got tier {}",
            spread
        );
        // 同じ統計でも局在していれば Legendary になる（局在性が効いている証明）
        let localized = lenia_rarity_tier(600.0, 0.9, 250, 0.90, 0.9, 0.1);
        assert_eq!(localized, 4, "localized version should be Legendary");
    }

    #[test]
    fn test_negative_zero_diversity_all_common() {
        let g1 = BiomeGrid::new(42);
        let g2 = BiomeGrid::new(42);
        let p1 = determine_rarity_with_progress(&g1);
        let p2 = determine_rarity_with_progress(&g2);
        assert_eq!(p1.rarity, 0);
        assert_eq!(p2.rarity, 0);
        assert_eq!(p1.species_hash, p2.species_hash);
    }

    #[test]
    fn test_rarity_increases_with_ticks() {
        let mut grid = BiomeGrid::new(42);
        for _ in 0..100 {
            grid.tick();
        }
        let progress = determine_rarity_with_progress(&grid);
        assert!(progress.mass > 0.0, "Orbium ring should survive");
        assert!(progress.longevity >= 10, "should accumulate longevity");
        assert!(
            progress.rarity >= 1,
            "stable orbium should reach at least Uncommon, got {}",
            progress.rarity
        );
    }

    #[test]
    fn test_determine_rarity_with_progress_has_lenia_fields() {
        let grid = BiomeGrid::new(42);
        let progress = determine_rarity_with_progress(&grid);
        assert!(progress.mass >= 0.0);
        assert!(progress.species_hash > 0);
    }
}

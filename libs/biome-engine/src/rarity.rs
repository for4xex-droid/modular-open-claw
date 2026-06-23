use crate::grid::BiomeGrid;
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
    pub morphology_count: u8,  // 共存する形態の種数 (0-5)
    pub has_homeostasis: bool, // 全元素がバランス
    pub diversity_index: f32,  // Shannon多様性指数
    pub condition_active_500: bool,
    pub condition_morph_3: bool,
    pub condition_morph_4: bool,
    pub condition_active_1000: bool,
}

/// グリッド全体の進化状態から詳細な進捗付きでレアリティを判定する
pub fn determine_rarity_with_progress(grid: &BiomeGrid) -> RarityProgress {
    use crate::evolution::{determine_morphology, CellMorphology};
    use crate::grid::GRID_SIZE;
    use std::collections::HashSet;

    let mut active_cells = 0;
    let mut morphs = HashSet::new();

    let mut basic_count = 0;
    let mut predator_count = 0;
    let mut producer_count = 0;
    let mut consumer_count = 0;
    let mut decomposer_count = 0;

    let mut element_totals = [0u64; 8];

    for i in 0..GRID_SIZE {
        let cell = &grid.current_cells()[i];
        if cell.active {
            active_cells += 1;
            let morph = determine_morphology(&cell.elements);
            morphs.insert(morph as u8);

            match morph {
                CellMorphology::Basic => basic_count += 1,
                CellMorphology::Predator => predator_count += 1,
                CellMorphology::Producer => producer_count += 1,
                CellMorphology::Consumer => consumer_count += 1,
                CellMorphology::Decomposer => decomposer_count += 1,
            }

            for (e, &val) in cell.elements.iter().enumerate() {
                element_totals[e] += val as u64;
            }
        }
    }

    let morphology_count = morphs.len() as u8;

    // Shannon多様性指数
    let mut diversity_index = 0.0;
    if active_cells > 0 {
        let counts = [
            basic_count,
            predator_count,
            producer_count,
            consumer_count,
            decomposer_count,
        ];
        for &count in &counts {
            if count > 0 {
                let p = count as f32 / active_cells as f32;
                diversity_index -= p * p.ln();
            }
        }
    }

    // Homeostasis (恒常性)
    let total_elements: u64 = element_totals.iter().sum();
    let has_homeostasis = if total_elements > 0 {
        let avg = total_elements / 8;
        let min_val = *element_totals.iter().min().unwrap_or(&0);
        // 最小の元素が平均の20%以上であること
        min_val >= avg / 5
    } else {
        false
    };

    // Legendary 条件
    let condition_active_500 = active_cells >= 500;
    let condition_morph_3 = morphology_count >= 3;
    let condition_morph_4 = morphology_count >= 4;
    let condition_active_1000 = active_cells >= 1000;

    // レアリティ判定
    let rarity = if condition_active_1000 && condition_morph_4 && has_homeostasis {
        4 // Legendary
    } else if condition_active_500 && condition_morph_3 {
        3 // Epic
    } else if active_cells >= 100 && morphology_count >= 2 {
        2 // Rare
    } else if active_cells >= 10 {
        1 // Uncommon
    } else {
        0 // Common
    };

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
        assert_eq!(rarity, BiomeRarity::Common);
    }

    #[test]
    fn test_legendary_criteria() {
        let mut grid = BiomeGrid::new(42);

        // 新しいLegendary条件を満たすようにセルのセットアップ (1024セル)
        for y in 0..32 {
            for x in 0..32 {
                let cell = grid.get_cell_mut(x, y);
                cell.active = true;
                cell.elements = [2000, 2000, 2000, 2000, 2000, 2000, 2000, 2000];
            }
        }

        // 4つの形態を作る
        // Predator (C, N)
        let c1 = grid.get_cell_mut(0, 0);
        c1.elements[0] = 45000;
        c1.elements[1] = 35000;

        // Producer (H, O)
        let c2 = grid.get_cell_mut(1, 1);
        c2.elements[3] = 45000;
        c2.elements[4] = 45000;

        // Consumer (C, P)
        let c3 = grid.get_cell_mut(2, 2);
        c3.elements[0] = 35000;
        c3.elements[2] = 25000;

        // Decomposer (S, N)
        let c4 = grid.get_cell_mut(3, 3);
        c4.elements[5] = 35000;
        c4.elements[1] = 25000;

        let rarity = determine_rarity(&grid);
        assert_eq!(
            rarity,
            BiomeRarity::Legendary,
            "Grid with highly evolved cells and homeostasis should be Legendary"
        );
    }

    #[test]
    fn test_determine_rarity_with_progress_initial() {
        let grid = BiomeGrid::new(42);
        let progress = determine_rarity_with_progress(&grid);
        assert_eq!(progress.rarity, 0); // Common
        assert_eq!(progress.active_cells, 0);
        assert_eq!(progress.morphology_count, 0);
    }

    #[test]
    fn test_determine_rarity_with_progress_legendary_full() {
        let mut grid = BiomeGrid::new(42);
        // 1000セル以上アクティブにする
        for y in 0..32 {
            for x in 0..32 {
                let cell = grid.get_cell_mut(x, y);
                cell.active = true;
                // 全元素をバランス良く注入して homeostasis を満たす
                cell.elements = [2000, 2000, 2000, 2000, 2000, 2000, 2000, 2000];
            }
        }

        // さまざまな形態のセルを作る
        // Predator: C > 40000 && N > 30000
        let p_cell = grid.get_cell_mut(0, 0);
        p_cell.elements[0] = 45000;
        p_cell.elements[1] = 35000;

        // Producer: H > 40000 && O > 40000
        let pr_cell = grid.get_cell_mut(1, 1);
        pr_cell.elements[3] = 45000;
        pr_cell.elements[4] = 45000;

        // Consumer: C > 30000 && P > 20000
        let c_cell = grid.get_cell_mut(2, 2);
        c_cell.elements[0] = 35000;
        c_cell.elements[2] = 25000;

        // Decomposer: S > 30000 && N > 20000
        let d_cell = grid.get_cell_mut(3, 3);
        d_cell.elements[5] = 35000;
        d_cell.elements[1] = 25000;

        let progress = determine_rarity_with_progress(&grid);
        // これらは実装後に満たされるはず
        assert_eq!(progress.rarity, 4); // Legendary
        assert!(progress.active_cells >= 1000);
        assert_eq!(progress.morphology_count, 5); // Basic, Predator, Producer, Consumer, Decomposer
        assert!(progress.has_homeostasis);
        assert!(progress.condition_active_1000);
        assert!(progress.condition_morph_4);
    }

    #[test]
    fn test_determine_rarity_with_progress_diversity_calculation() {
        let mut grid = BiomeGrid::new(42);

        // 形態比率を設定して、シャノン多様性指数が正しく計算されるかを検証
        // 2つの形態が同数存在する場合、多様性指数は -2 * (0.5 * ln(0.5)) = ln(2) = 0.693
        let c1 = grid.get_cell_mut(0, 0);
        c1.active = true;
        c1.elements = [0, 0, 0, 0, 0, 0, 0, 0]; // Basic

        let c2 = grid.get_cell_mut(1, 1);
        c2.active = true;
        c2.elements[0] = 45000; // Predator
        c2.elements[1] = 35000;

        let progress = determine_rarity_with_progress(&grid);
        assert_eq!(progress.morphology_count, 2); // Basic + Predator
        assert!((progress.diversity_index - 0.693).abs() < 0.01);
    }
}

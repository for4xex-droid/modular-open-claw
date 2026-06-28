/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
use crate::grid::BiomeGrid;
use rand::Rng;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubstanceKind {
    None,
    Higgs,
    Tachyon,
}

/// 超物質の発見を判定する
pub fn roll_substance_discovery(grid: &BiomeGrid, rng: &mut impl Rng) -> SubstanceKind {
    use crate::grid::GRID_SIZE;

    // グリッド内に高鉄濃度 (Fe > 40000) のセルがあるかスキャン
    let mut has_high_iron = false;
    for i in 0..GRID_SIZE {
        if grid.current_cells()[i].active && grid.current_cells()[i].elements[6] >= 40000 {
            has_high_iron = true;
            break;
        }
    }

    // 確率の決定 (1000分率)
    // 高鉄濃度なら 30 (3%)、それ以外なら 1 (0.1%)
    let threshold = if has_high_iron { 30 } else { 1 };
    let roll: u32 = rng.gen_range(0..1000);

    if roll < threshold {
        // ヒッグスかタキオンを50%ずつの確率で決定
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
    fn test_high_iron_increases_discovery_rate() {
        let mut grid = BiomeGrid::new(42);

        // 鉄 (Fe: インデックス6) を大量に注入
        for i in 0..10 {
            grid.get_cell_mut(i, i).active = true;
            grid.get_cell_mut(i, i).elements[6] = 45000;
        }

        let mut rng = SmallRng::seed_from_u64(100);
        let mut discoveries = 0;

        // 1000回ロールして確率をチェック
        for _ in 0..1000 {
            if roll_substance_discovery(&grid, &mut rng) != SubstanceKind::None {
                discoveries += 1;
            }
        }

        // 高鉄濃度なら約3% (1000回中15〜45回程度) 発見されるはず。
        // 現在は roll_substance_discovery が None のみ返すので、このテストは失敗する (RED)
        assert!(
            discoveries > 5,
            "Should discover substances under high iron condition"
        );
    }
}

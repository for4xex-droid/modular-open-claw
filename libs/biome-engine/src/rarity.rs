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

/// グリッド全体の進化状態からレアリティを判定する
pub fn determine_rarity(grid: &BiomeGrid) -> BiomeRarity {
    use crate::evolution::{determine_morphology, CellMorphology};
    use crate::grid::GRID_SIZE;

    let mut active_count = 0;
    let mut predator_count = 0;
    let mut producer_count = 0;
    let mut consumer_count = 0;
    let mut decomposer_count = 0;
    let mut max_element_val = 0;

    for i in 0..GRID_SIZE {
        let cell = &grid.current_cells()[i];
        if cell.active {
            active_count += 1;

            // 最大元素量の取得
            for &val in &cell.elements {
                if val > max_element_val {
                    max_element_val = val;
                }
            }

            // 形態の判定
            match determine_morphology(&cell.elements) {
                CellMorphology::Predator => predator_count += 1,
                CellMorphology::Producer => producer_count += 1,
                CellMorphology::Consumer => consumer_count += 1,
                CellMorphology::Decomposer => decomposer_count += 1,
                CellMorphology::Basic => {}
            }
        }
    }

    let special_count = predator_count + producer_count + consumer_count + decomposer_count;

    if special_count >= 5 && max_element_val >= 40000 {
        BiomeRarity::Legendary
    } else if special_count >= 3 {
        BiomeRarity::Epic
    } else if special_count >= 1 {
        BiomeRarity::Rare
    } else if active_count >= 10 {
        BiomeRarity::Uncommon
    } else {
        BiomeRarity::Common
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

        // グリッド上の多数のセルを非常に高い元素量でアクティブにし、形態も多様化させる
        for i in 0..10 {
            let cell = grid.get_cell_mut(i, i);
            cell.active = true;
            cell.elements[0] = 50000; // C
            cell.elements[1] = 40000; // N (Predatorの条件を満たす)
        }

        let rarity = determine_rarity(&grid);
        assert_eq!(
            rarity,
            BiomeRarity::Legendary,
            "Grid with highly evolved cells should be Legendary"
        );
    }
}

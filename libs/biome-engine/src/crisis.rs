use crate::grid::BiomeGrid;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CrisisType {
    None,
    Meteor,
    IceAge,
}

/// グリッド全体の凍結状態を解除する
pub fn thaw_grid(grid: &mut BiomeGrid) {
    use crate::grid::{GRID_HEIGHT, GRID_WIDTH};
    for y in 0..GRID_HEIGHT {
        for x in 0..GRID_WIDTH {
            grid.get_cell_mut(x, y).is_frozen = false;
        }
    }
}

/// 災害イベントを実行する
pub fn apply_crisis(grid: &mut BiomeGrid, crisis: CrisisType, target_x: usize, target_y: usize) {
    use crate::grid::{GRID_HEIGHT, GRID_WIDTH};

    match crisis {
        CrisisType::Meteor => {
            let radius = 2;
            let start_x = target_x.saturating_sub(radius);
            let end_x = (target_x + radius).min(GRID_WIDTH - 1);
            let start_y = target_y.saturating_sub(radius);
            let end_y = (target_y + radius).min(GRID_HEIGHT - 1);

            for y in start_y..=end_y {
                for x in start_x..=end_x {
                    let cell = grid.get_cell_mut(x, y);
                    if cell.active {
                        let resistance = cell.genome.get_value(8);
                        if resistance < 65535 {
                            // 元素の回転（比率の変化・多様性促進）
                            cell.elements.rotate_right(1);
                        }
                    }
                }
            }
        }
        CrisisType::IceAge => {
            for y in 0..GRID_HEIGHT {
                for x in 0..GRID_WIDTH {
                    let cell = grid.get_cell_mut(x, y);
                    if cell.active {
                        let resistance = cell.genome.get_value(8);
                        let total_elements: u32 = cell.elements.iter().map(|&e| e as u32).sum();

                        // 活性度が低く（総元素量 < 1000）、かつ耐性も低い（< 30000）セルを凍結
                        if total_elements < 1000 && resistance < 30000 {
                            cell.is_frozen = true;
                        }
                    }
                }
            }
        }
        CrisisType::None => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_meteor_impact_rotates_elements_and_retains_active() {
        let mut grid = BiomeGrid::new(42);

        // テスト用セルを配置（耐性0で、元素を設定）
        let cell = grid.get_cell_mut(10, 10);
        cell.active = true;
        cell.genome.set_value(8, 0); // 耐性0
        cell.elements = [100, 200, 300, 400, 500, 600, 700, 800];

        // 隕石を (10, 10) に落下させる
        apply_crisis(&mut grid, CrisisType::Meteor, 10, 10);

        let result_cell = grid.get_cell(10, 10);
        // セルはアクティブのままであること
        assert!(result_cell.active, "Cell should remain active");
        // 元素が回転していること (elements.rotate_right(1) なら [800, 100, 200, 300, 400, 500, 600, 700] になる)
        assert_ne!(
            result_cell.elements,
            [100, 200, 300, 400, 500, 600, 700, 800]
        );
        // 耐性がないので元素が変更されている
    }

    #[test]
    fn test_meteor_no_rotation_with_high_resistance() {
        let mut grid = BiomeGrid::new(42);

        // 耐性最大のセルを配置
        let cell = grid.get_cell_mut(10, 10);
        cell.active = true;
        cell.genome.set_value(8, 65535); // 耐性最大
        cell.elements = [100, 200, 300, 400, 500, 600, 700, 800];

        apply_crisis(&mut grid, CrisisType::Meteor, 10, 10);

        let result_cell = grid.get_cell(10, 10);
        // 耐性最大なので、元素比率が回転しないこと
        assert_eq!(
            result_cell.elements,
            [100, 200, 300, 400, 500, 600, 700, 800]
        );
    }

    #[test]
    fn test_ice_age_freezes_inactive_cells() {
        let mut grid = BiomeGrid::new(42);

        // 活性度が低いセル (元素合計 < 1000)
        let cell_low = grid.get_cell_mut(5, 5);
        cell_low.active = true;
        cell_low.genome.set_value(8, 0); // 耐性0
        cell_low.elements = [10, 10, 10, 10, 10, 10, 10, 10]; // 合計 80 < 1000
        cell_low.is_frozen = false;

        // 活性度が高いセル (元素合計 >= 1000)
        let cell_high = grid.get_cell_mut(6, 6);
        cell_high.active = true;
        cell_high.genome.set_value(8, 0); // 耐性0
        cell_high.elements = [200, 200, 200, 200, 200, 100, 100, 100]; // 合計 1300 >= 1000
        cell_high.is_frozen = false;

        // 活性度は低いが耐性が高いセル
        let cell_res = grid.get_cell_mut(7, 7);
        cell_res.active = true;
        cell_res.genome.set_value(8, 65535); // 耐性最大
        cell_res.elements = [10, 10, 10, 10, 10, 10, 10, 10];
        cell_res.is_frozen = false;

        // 氷河期を発生させる
        apply_crisis(&mut grid, CrisisType::IceAge, 0, 0);

        // 低活性セルは凍結されること
        assert!(
            grid.get_cell(5, 5).is_frozen,
            "Low activity cell should be frozen"
        );
        // 高活性セルは凍結されないこと
        assert!(
            !grid.get_cell(6, 6).is_frozen,
            "High activity cell should not be frozen"
        );
        // 耐性高セルは凍結されないこと
        assert!(
            !grid.get_cell(7, 7).is_frozen,
            "Resistant cell should not be frozen"
        );
    }

    #[test]
    fn test_thaw_grid_unfreezes_all_cells() {
        let mut grid = BiomeGrid::new(42);
        grid.get_cell_mut(5, 5).is_frozen = true;
        grid.get_cell_mut(10, 10).is_frozen = true;

        // 解凍を実行
        super::thaw_grid(&mut grid);

        assert!(!grid.get_cell(5, 5).is_frozen);
        assert!(!grid.get_cell(10, 10).is_frozen);
    }
}

use crate::grid::BiomeGrid;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CrisisType {
    None,
    Meteor,
    IceAge,
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
                    // 災害耐性 (ゲノムインデックス 8) が高いセルは生き残る
                    let resistance = cell.genome.get_value(8);
                    if resistance < 50000 {
                        cell.active = false;
                    }
                }
            }
        }
        CrisisType::IceAge => {
            for y in 0..GRID_HEIGHT {
                for x in 0..GRID_WIDTH {
                    let cell = grid.get_cell_mut(x, y);
                    if cell.active {
                        let resistance = cell.genome.get_value(8) as u32;
                        // 通常は 80% に減少、耐性が高ければ減少幅が小さくなる
                        // factor: 80% (耐性0) 〜 100% (耐性65535)
                        let factor = 80 + (20 * resistance) / 65535;
                        for i in 0..8 {
                            cell.elements[i] = ((cell.elements[i] as u32 * factor) / 100) as u16;
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
    fn test_meteor_impact_destroys_cells() {
        let mut grid = BiomeGrid::new(42);

        // インパクト周囲をアクティブにする
        for y in 8..=12 {
            for x in 8..=12 {
                grid.get_cell_mut(x, y).active = true;
            }
        }

        // 隕石を (10, 10) に落下させる
        apply_crisis(&mut grid, CrisisType::Meteor, 10, 10);

        // 中心 (10, 10) が破壊（active = false）されていることを期待
        // 現在は apply_crisis が空なので、このテストは失敗する (RED)
        assert!(
            !grid.get_cell(10, 10).active,
            "Center cell should be destroyed by Meteor"
        );
    }

    #[test]
    fn test_ice_age_reduces_elements() {
        let mut grid = BiomeGrid::new(42);
        grid.get_cell_mut(5, 5).active = true;
        grid.get_cell_mut(5, 5).elements[0] = 1000; // 炭素

        // 氷河期を発生させる
        apply_crisis(&mut grid, CrisisType::IceAge, 0, 0);

        // 元素量が減衰していることを期待
        // 現在は apply_crisis が空なので、このテストは失敗する (RED)
        assert!(
            grid.get_cell(5, 5).elements[0] < 1000,
            "Ice age should reduce element amounts"
        );
    }

    #[test]
    fn test_meteor_survival_with_high_resistance() {
        let mut grid = BiomeGrid::new(42);

        // (10, 10) に耐性最大 (65535) のセルを配置
        grid.get_cell_mut(10, 10).active = true;
        grid.get_cell_mut(10, 10).genome.set_value(8, 65535); // 災害耐性

        apply_crisis(&mut grid, CrisisType::Meteor, 10, 10);

        // 耐性が高いため、破壊されずに生き残ることを期待
        assert!(
            grid.get_cell(10, 10).active,
            "Resistant cell should survive Meteor"
        );
    }

    #[test]
    fn test_ice_age_survival_with_high_resistance() {
        let mut grid = BiomeGrid::new(42);

        // 耐性最大のセルと、耐性初期値 (10000) のセルを配置
        grid.get_cell_mut(5, 5).active = true;
        grid.get_cell_mut(5, 5).elements[0] = 1000;
        grid.get_cell_mut(5, 5).genome.set_value(8, 65535); // 耐性最大

        grid.get_cell_mut(6, 6).active = true;
        grid.get_cell_mut(6, 6).elements[0] = 1000;
        grid.get_cell_mut(6, 6).genome.set_value(8, 10000); // 耐性低

        apply_crisis(&mut grid, CrisisType::IceAge, 0, 0);

        let res_cell_val = grid.get_cell(5, 5).elements[0];
        let low_cell_val = grid.get_cell(6, 6).elements[0];

        // 耐性最大のセルは元素減少が完全に防がれる (factor = 100%)
        assert_eq!(
            res_cell_val, 1000,
            "Resistant cell should not lose elements"
        );
        // 耐性低のセルは元素が減少する (80% + 20*10000/65535 = 約83% に減少)
        assert!(
            low_cell_val < 1000,
            "Low resistance cell should lose elements"
        );
        assert!(
            res_cell_val > low_cell_val,
            "Resistant cell should retain more elements than low resistance cell"
        );
    }
}

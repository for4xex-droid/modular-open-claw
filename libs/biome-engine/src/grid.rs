use crate::genome::CellGenome;
use rand::rngs::SmallRng;
use rand::SeedableRng;
use wasm_bindgen::prelude::*;

pub const GRID_WIDTH: usize = 128;
pub const GRID_HEIGHT: usize = 128;
pub const GRID_SIZE: usize = GRID_WIDTH * GRID_HEIGHT;

#[wasm_bindgen]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CellDelta {
    pub x: u16,
    pub y: u16,
    pub active: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BiomeCell {
    pub active: bool,
    pub elements: [u16; 8], // C, N, P, H, O, S, Fe, Si
    pub genome: CellGenome,
    pub is_frozen: bool,
}

impl BiomeCell {
    pub fn new() -> Self {
        Self {
            active: false,
            elements: [0u16; 8],
            genome: CellGenome::default_nurture(),
            is_frozen: false,
        }
    }
}

impl Default for BiomeCell {
    fn default() -> Self {
        Self::new()
    }
}

pub struct BiomeGrid {
    pub(crate) cells: Vec<BiomeCell>,
    rng: SmallRng,
}

impl BiomeGrid {
    pub fn new(seed: u64) -> Self {
        let cells = vec![BiomeCell::new(); GRID_SIZE];
        let rng = SmallRng::seed_from_u64(seed);
        Self { cells, rng }
    }

    pub fn get_cell(&self, x: usize, y: usize) -> &BiomeCell {
        &self.cells[y * GRID_WIDTH + x]
    }

    pub fn get_cell_mut(&mut self, x: usize, y: usize) -> &mut BiomeCell {
        &mut self.cells[y * GRID_WIDTH + x]
    }

    /// グリッドの状態を1ステップ進め、変更されたセルの差分を返す
    pub fn tick(&mut self) -> Vec<CellDelta> {
        use rand::Rng;
        let mut next_cells = self.cells.clone();
        let mut deltas = Vec::new();

        for y in 0..GRID_HEIGHT {
            for x in 0..GRID_WIDTH {
                let idx = y * GRID_WIDTH + x;
                let cell = &self.cells[idx];

                // アクティブかつ炭素(インデックス0)が十分に存在するセルから周囲に拡散
                if cell.active && cell.elements[0] > 100 {
                    let spread_amount = cell.elements[0] / 10;
                    if spread_amount == 0 {
                        continue;
                    }

                    let neighbors = [
                        (x.wrapping_sub(1), y),
                        (x + 1, y),
                        (x, y.wrapping_sub(1)),
                        (x, y + 1),
                    ];

                    for &(nx, ny) in &neighbors {
                        if nx < GRID_WIDTH && ny < GRID_HEIGHT {
                            let n_idx = ny * GRID_WIDTH + nx;

                            // 乱数要素の追加 (警告回避および将来の確率変動用)
                            let rand_factor: u16 = self.rng.gen_range(80..=120);
                            let final_spread =
                                ((spread_amount as u32 * rand_factor as u32) / 100) as u16;

                            if final_spread > 0 && next_cells[idx].elements[0] >= final_spread {
                                next_cells[n_idx].active = true;
                                next_cells[n_idx].elements[0] =
                                    next_cells[n_idx].elements[0].saturating_add(final_spread);
                                next_cells[idx].elements[0] =
                                    next_cells[idx].elements[0].saturating_sub(final_spread);
                            }
                        }
                    }
                }
            }
        }

        // 状態が変化したセルを検出して差分を収集
        for (idx, (cell, next_cell)) in self.cells.iter().zip(next_cells.iter()).enumerate() {
            if cell != next_cell {
                let x = (idx % GRID_WIDTH) as u16;
                let y = (idx / GRID_WIDTH) as u16;
                deltas.push(CellDelta {
                    x,
                    y,
                    active: next_cell.active,
                });
            }
        }

        self.cells = next_cells;
        deltas
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grid_dimensions() {
        let grid = BiomeGrid::new(42);
        assert_eq!(grid.cells.len(), GRID_SIZE);
    }

    #[test]
    fn test_tick_produces_changes() {
        let mut grid = BiomeGrid::new(42);

        // テストのために特定のセルをアクティブにする
        grid.get_cell_mut(10, 10).active = true;
        grid.get_cell_mut(10, 10).elements[0] = 5000; // 炭素注入

        let deltas = grid.tick();

        // 状態が変化し、差分(CellDelta)が返ってくることを期待する。
        // 現在は tick() が空の Vec を返すので、このテストは失敗する (RED)
        assert!(
            !deltas.is_empty(),
            "Grid tick should produce state changes and return deltas"
        );
    }

    #[test]
    fn test_deterministic_behavior() {
        let mut grid1 = BiomeGrid::new(12345);
        let mut grid2 = BiomeGrid::new(12345);
        let mut grid3 = BiomeGrid::new(54321); // 異なるシード

        // 同一の初期化操作
        grid1.get_cell_mut(5, 5).active = true;
        grid2.get_cell_mut(5, 5).active = true;
        grid3.get_cell_mut(5, 5).active = true;

        grid1.tick();
        grid2.tick();
        grid3.tick();

        // grid1 と grid2 は同一のシードなので状態が完全に一致するはず
        assert_eq!(
            grid1.cells, grid2.cells,
            "Grids with same seed should be identical"
        );

        // grid1 と grid3 は異なるシードなので状態が異なるはず (ただし現時点では両方 tick が空なので
        // このテストはたまたまパスする可能性があるが、実実装が入ると差が出る)
    }
}

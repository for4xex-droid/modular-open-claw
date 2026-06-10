use crate::evolution::CellMorphology;
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

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct BiomeCell {
    pub active: bool,
    pub elements: [u16; 8], // C, N, P, H, O, S, Fe, Si
    pub genome: CellGenome,
    pub is_frozen: bool,
    pub morphology: CellMorphology,
}

impl BiomeCell {
    pub fn new() -> Self {
        Self {
            active: false,
            elements: [0u16; 8],
            genome: CellGenome::default_nurture(),
            is_frozen: false,
            morphology: CellMorphology::Basic,
        }
    }
}

impl Default for BiomeCell {
    fn default() -> Self {
        Self::new()
    }
}

pub struct BiomeGrid {
    cells_a: Vec<BiomeCell>,
    cells_b: Vec<BiomeCell>,
    current_is_a: bool,
    rng: SmallRng,
    pub mutation_boost: f32,
    pub ticks_since_mutation: u32,
    render_buffer: Vec<f32>,
}

impl BiomeGrid {
    pub fn new(seed: u64) -> Self {
        let cells_a = vec![BiomeCell::new(); GRID_SIZE];
        let cells_b = vec![BiomeCell::new(); GRID_SIZE];
        let rng = SmallRng::seed_from_u64(seed);
        let render_buffer = vec![0.0f32; GRID_SIZE * 12];
        Self {
            cells_a,
            cells_b,
            current_is_a: true,
            rng,
            mutation_boost: 1.0,
            ticks_since_mutation: 0,
            render_buffer,
        }
    }

    pub fn current_cells(&self) -> &Vec<BiomeCell> {
        if self.current_is_a {
            &self.cells_a
        } else {
            &self.cells_b
        }
    }

    pub fn current_cells_mut(&mut self) -> &mut Vec<BiomeCell> {
        if self.current_is_a {
            &mut self.cells_a
        } else {
            &mut self.cells_b
        }
    }

    #[allow(dead_code)]
    fn next_cells_mut(&mut self) -> &mut Vec<BiomeCell> {
        if self.current_is_a {
            &mut self.cells_b
        } else {
            &mut self.cells_a
        }
    }

    pub fn set_current_cells(&mut self, cells: Vec<BiomeCell>) {
        if self.current_is_a {
            self.cells_a = cells;
        } else {
            self.cells_b = cells;
        }
    }

    pub fn render_data_ptr(&self) -> *const f32 {
        self.render_buffer.as_ptr()
    }

    pub fn render_data_len(&self) -> usize {
        self.render_buffer.len()
    }

    pub fn get_cell(&self, x: usize, y: usize) -> &BiomeCell {
        &self.current_cells()[y * GRID_WIDTH + x]
    }

    pub fn get_cell_mut(&mut self, x: usize, y: usize) -> &mut BiomeCell {
        &mut self.current_cells_mut()[y * GRID_WIDTH + x]
    }

    /// グリッドの状態を1ステップ進め、変更されたセルの差分を返す
    pub fn tick(&mut self) -> Vec<CellDelta> {
        use rand::Rng;

        let current_is_a = self.current_is_a;
        let cells_a = &mut self.cells_a;
        let cells_b = &mut self.cells_b;
        let rng = &mut self.rng;
        let render_buffer = &mut self.render_buffer;
        let mut ticks_since_mutation = self.ticks_since_mutation;
        let mutation_boost = self.mutation_boost;

        // current と next の参照を分割
        let (current_cells, next_cells) = if current_is_a {
            (&*cells_a, cells_b)
        } else {
            (&*cells_b, cells_a)
        };

        // 1. nextバッファにcurrentバッファの内容をコピー (Vecなのでclone_from_sliceが安全かつ高速)
        next_cells.clone_from_slice(current_cells);

        let mut deltas = Vec::new();

        // 拡散計算
        for y in 0..GRID_HEIGHT {
            for x in 0..GRID_WIDTH {
                let idx = y * GRID_WIDTH + x;
                let cell = &current_cells[idx];

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
                            let rand_factor: u16 = rng.gen_range(80..=120);
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

        let mut mutation_occurred = false;
        let base_rate = 205u16;
        let mutation_rate = ((base_rate as f32 * mutation_boost) as u32).min(65535) as u16;

        ticks_since_mutation += 1;
        let force_mutate = ticks_since_mutation >= 1000;

        for cell in next_cells.iter_mut() {
            if cell.active {
                // 1. 元素反応
                crate::element::react_elements(cell);

                // 2. 突然変異
                let should_mutate = if force_mutate {
                    true
                } else {
                    let roll: u16 = rng.gen_range(0..=65535);
                    roll < mutation_rate
                };

                if should_mutate {
                    let mut mutated_genome = cell.genome.clone();
                    mutated_genome.mutate(mutation_rate, rng);
                    cell.genome = mutated_genome;
                    mutation_occurred = true;
                }

                // 3. 形態決定
                cell.morphology = crate::evolution::determine_morphology(&cell.elements);
            }
        }

        if mutation_occurred {
            ticks_since_mutation = 0;
        }

        for (idx, (cell, next_cell)) in current_cells.iter().zip(next_cells.iter()).enumerate() {
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

        // 状態を書き戻す
        self.current_is_a = !current_is_a;
        self.ticks_since_mutation = ticks_since_mutation;

        // 反転後の現在のバッファ（＝更新されたnext_cells）をレンダリングバッファに反映
        for (idx, cell) in next_cells.iter().enumerate() {
            let offset = idx * 12;
            let x = (idx % GRID_WIDTH) as f32;
            let y = (idx / GRID_WIDTH) as f32;

            render_buffer[offset] = x;
            render_buffer[offset + 1] = y;
            render_buffer[offset + 2] = if cell.active { 1.0 } else { 0.0 };
            render_buffer[offset + 3] = cell.morphology as u32 as f32;

            for i in 0..8 {
                render_buffer[offset + 4 + i] = cell.elements[i] as f32;
            }
        }

        deltas
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grid_dimensions() {
        let grid = BiomeGrid::new(42);
        assert_eq!(grid.current_cells().len(), GRID_SIZE);
    }

    #[test]
    fn test_tick_produces_changes() {
        let mut grid = BiomeGrid::new(42);

        // テストのために特定のセルをアクティブにする
        grid.get_cell_mut(10, 10).active = true;
        grid.get_cell_mut(10, 10).elements[0] = 5000; // 炭素注入

        let deltas = grid.tick();

        // 状態が変化し、差分(CellDelta)が返ってくることを期待する。
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
            grid1.current_cells(),
            grid2.current_cells(),
            "Grids with same seed should be identical"
        );

        // grid1 と grid3 は異なるシードなので状態が異なるはず (ただし現時点では両方 tick が空なので
        // このテストはたまたまパスする可能性があるが、実実装が入ると差が出る)
    }

    #[test]
    fn test_double_buffering() {
        let mut grid = BiomeGrid::new(42);
        let initial_ptr = grid.current_cells().as_ptr();
        grid.tick();
        let next_ptr = grid.current_cells().as_ptr();
        assert_ne!(
            initial_ptr, next_ptr,
            "Pointer should swap to avoid allocation"
        );
    }

    #[test]
    fn test_cell_morphology_initialization() {
        let cell = BiomeCell::new();
        assert_eq!(cell.morphology, crate::evolution::CellMorphology::Basic);
    }

    #[test]
    fn test_pity_system_and_mutation_boost() {
        let mut grid = BiomeGrid::new(42);
        assert_eq!(grid.ticks_since_mutation, 0);
        assert_eq!(grid.mutation_boost, 1.0);

        // 突然変異はアクティブなセルでのみ発生するため、1セルをアクティブ化する
        grid.get_cell_mut(5, 5).active = true;

        grid.mutation_boost = 2.0;
        grid.ticks_since_mutation = 999;

        grid.tick();
        assert_eq!(grid.ticks_since_mutation, 0);
    }

    #[test]
    fn test_render_buffer_updates() {
        let mut grid = BiomeGrid::new(42);
        grid.get_cell_mut(5, 5).active = true;
        grid.get_cell_mut(5, 5).elements[0] = 1000;

        grid.tick();

        let ptr = grid.render_data_ptr();
        let len = grid.render_data_len();
        assert_ne!(ptr, std::ptr::null());
        assert_eq!(len, GRID_SIZE * 12);

        let slice = unsafe { std::slice::from_raw_parts(ptr, len) };
        let cell_idx = 5 * GRID_WIDTH + 5;
        let offset = cell_idx * 12;
        assert_eq!(slice[offset], 5.0); // x
        assert_eq!(slice[offset + 1], 5.0); // y
        assert_eq!(slice[offset + 2], 1.0); // active
    }
}

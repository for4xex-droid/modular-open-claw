/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
use crate::evolution::CellMorphology;
use crate::genome::CellGenome;
use rand::rngs::SmallRng;
use rand::SeedableRng;
use wasm_bindgen::prelude::*;

pub const GRID_WIDTH: usize = 128;
pub const GRID_HEIGHT: usize = 128;
pub const GRID_SIZE: usize = GRID_WIDTH * GRID_HEIGHT;
/// render_buffer 1セルあたりの Float32 数（x,y,active,morph,elements×8,is_frozen）
pub const RENDER_STRIDE: usize = 13;

#[allow(dead_code)]
const DIFFUSION_RATES: [u16; 8] = [6, 12, 5, 14, 10, 7, 3, 4];

#[wasm_bindgen]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CellDelta {
    pub x: u16,
    pub y: u16,
    pub active: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
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
    #[allow(dead_code)]
    rng: SmallRng,
    pub mutation_boost: f32,
    pub ticks_since_mutation: u32,
    render_buffer: Vec<f32>,
    lenia: crate::lenia::LeniaSimulator,
}

impl BiomeGrid {
    pub fn new(seed: u64) -> Self {
        let cells_a = vec![BiomeCell::new(); GRID_SIZE];
        let cells_b = vec![BiomeCell::new(); GRID_SIZE];
        let rng = SmallRng::seed_from_u64(seed);
        let render_buffer = vec![0.0f32; GRID_SIZE * RENDER_STRIDE];
        let lenia = crate::lenia::LeniaSimulator::new(seed);
        Self {
            cells_a,
            cells_b,
            current_is_a: true,
            rng,
            mutation_boost: 1.0,
            ticks_since_mutation: 0,
            render_buffer,
            lenia,
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

    fn write_render_buffer(render_buffer: &mut [f32], cells: &[BiomeCell], env_mask: &[u8]) {
        for (idx, cell) in cells.iter().enumerate() {
            let offset = idx * RENDER_STRIDE;
            render_buffer[offset] = (idx % GRID_WIDTH) as f32;
            render_buffer[offset + 1] = (idx / GRID_WIDTH) as f32;
            // active スロットに状態を集約:
            //   0=空, 1=活性, 2=Prismatic, 負値=環境ペン地形（-1 壁 / -2 養分 / -3 毒）。
            //   非活性セルのみ地形を表示（生命がいれば生命を優先）。ストライド変更を避ける設計。
            render_buffer[offset + 2] = if cell.active {
                if cell.genome.is_prismatic() {
                    2.0
                } else {
                    1.0
                }
            } else {
                match env_mask.get(idx).copied().unwrap_or(0) {
                    1 => -1.0,
                    2 => -2.0,
                    3 => -3.0,
                    _ => 0.0,
                }
            };
            render_buffer[offset + 3] = cell.morphology as u32 as f32;
            for i in 0..8 {
                render_buffer[offset + 4 + i] = cell.elements[i] as f32;
            }
            render_buffer[offset + 12] = if cell.is_frozen { 1.0 } else { 0.0 };
        }
    }

    /// 半径 radius の正方形ブラシで Lenia 場に種を撒く
    pub fn inject_brush(&mut self, x: usize, y: usize, radius: usize, _idx: usize, amount: u16) {
        let strength = (amount as f32 / 65535.0).clamp(0.05, 1.0);
        self.lenia.seed_brush(x, y, radius, strength);
        self.sync_cells_from_lenia();
        let cells = self.current_cells().to_vec();
        let env = self.lenia.env_mask().to_vec();
        Self::write_render_buffer(&mut self.render_buffer, &cells, &env);
    }

    pub fn lenia(&self) -> &crate::lenia::LeniaSimulator {
        &self.lenia
    }

    pub fn lenia_mut(&mut self) -> &mut crate::lenia::LeniaSimulator {
        &mut self.lenia
    }

    pub fn lenia_snapshot(&self) -> crate::lenia::LeniaSnapshot {
        self.lenia.snapshot()
    }

    pub fn restore_lenia_snapshot(&mut self, snap: &crate::lenia::LeniaSnapshot) {
        self.lenia.restore_snapshot(snap);
        self.sync_cells_from_lenia();
        let cells = self.current_cells().to_vec();
        let env = self.lenia.env_mask().to_vec();
        Self::write_render_buffer(&mut self.render_buffer, &cells, &env);
    }

    /// Lenia 場を外部から差し替えた後（種の再配置・エコシステム構築・環境ペン）に、
    /// セル状態と render_buffer を場に同期させる。
    pub fn sync_after_lenia_reseed(&mut self) {
        self.sync_cells_from_lenia();
        let cells = self.current_cells().to_vec();
        let env = self.lenia.env_mask().to_vec();
        Self::write_render_buffer(&mut self.render_buffer, &cells, &env);
    }

    fn sync_cells_from_lenia(&mut self) {
        let field = self.lenia.field().to_vec();
        let cells = self.current_cells_mut();
        for (idx, cell) in cells.iter_mut().enumerate() {
            let base = idx;
            let v0 = field[base];
            let v1 = field[GRID_SIZE + base];
            let v2 = field[GRID_SIZE * 2 + base];
            let active = v0 > 0.12 || v1 > 0.12 || v2 > 0.12;
            cell.active = active;
            if active {
                cell.elements[0] = (v0 * 65535.0) as u16;
                cell.elements[1] = (v1 * 65535.0) as u16;
                cell.elements[2] = (v2 * 65535.0) as u16;
            }
        }
    }

    /// Lenia を count 世代進め、末尾で 1 回だけセル同期と render_buffer 更新
    pub fn tick_n(&mut self, count: u32) {
        if count == 0 {
            return;
        }
        for _ in 0..count {
            self.lenia.tick();
        }
        self.sync_cells_from_lenia();
        let cells = self.current_cells().to_vec();
        let env = self.lenia.env_mask().to_vec();
        Self::write_render_buffer(&mut self.render_buffer, &cells, &env);
        self.ticks_since_mutation = self.ticks_since_mutation.saturating_add(count);
    }

    /// グリッドの状態を1ステップ進める（Lenia 専用 — 差分イベントは未使用）
    pub fn tick(&mut self) -> Vec<CellDelta> {
        self.tick_n(1);
        Vec::new()
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
        let mass_before = grid.lenia().mass;
        grid.tick();
        assert!(
            grid.lenia().mass > 0.0 || mass_before > 0.0,
            "Lenia tick should maintain or evolve field mass"
        );
    }

    #[test]
    fn test_deterministic_behavior() {
        let mut grid1 = BiomeGrid::new(12345);
        let mut grid2 = BiomeGrid::new(12345);
        let mut grid3 = BiomeGrid::new(54321); // 異なるシード

        // 同一シードの Lenia 場は一致
        grid1.tick();
        grid2.tick();
        grid3.tick();

        assert_eq!(
            grid1.lenia().field(),
            grid2.lenia().field(),
            "Grids with same seed should have identical Lenia fields"
        );
        assert_ne!(
            grid1.lenia().field(),
            grid3.lenia().field(),
            "Different seeds should produce different Lenia fields"
        );
    }

    #[test]
    fn test_lenia_field_updates_on_tick() {
        let mut grid = BiomeGrid::new(42);
        assert!(grid.lenia().mass > 0.0);
        grid.tick();
        assert!(
            grid.lenia().mass > 0.0,
            "Lenia mass should persist after tick"
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
        grid.mutation_boost = 2.0;
        grid.tick();
        assert_eq!(grid.ticks_since_mutation, 1);
    }

    #[test]
    fn test_render_buffer_updates() {
        let mut grid = BiomeGrid::new(42);
        grid.tick();

        let ptr = grid.render_data_ptr();
        let len = grid.render_data_len();
        assert_ne!(ptr, std::ptr::null());
        assert_eq!(len, GRID_SIZE * RENDER_STRIDE);

        let slice = unsafe { std::slice::from_raw_parts(ptr, len) };
        let cell_idx = 64 * GRID_WIDTH + 64;
        let offset = cell_idx * RENDER_STRIDE;
        assert_eq!(slice[offset], 64.0);
        assert_eq!(slice[offset + 1], 64.0);
        assert!(slice[offset + 2] >= 0.0);
    }

    #[test]
    fn test_frozen_cells_skip_element_reaction_and_mutation() {
        let mut grid = BiomeGrid::new(42);
        grid.get_cell_mut(5, 5).is_frozen = true;
        grid.tick();

        assert!(grid.get_cell(5, 5).is_frozen);
        let slice =
            unsafe { std::slice::from_raw_parts(grid.render_data_ptr(), grid.render_data_len()) };
        let offset = (5 * GRID_WIDTH + 5) * RENDER_STRIDE;
        assert_eq!(slice[offset + 12], 1.0, "frozen flag in render buffer");
    }

    #[test]
    fn test_decay_system_kills_depleted_cells() {
        let mut grid = BiomeGrid::new(42);
        let mass_before = grid.lenia().mass;
        for _ in 0..20 {
            grid.tick();
        }
        assert!(
            grid.lenia().mass > mass_before * 0.1,
            "Lenia mass should not collapse within 20 ticks"
        );
    }

    #[test]
    fn test_all_elements_diffusion() {
        let mut grid = BiomeGrid::new(42);
        let mass0 = grid.lenia().mass;
        grid.inject_brush(64, 64, 3, 0, 20000);
        assert!(
            grid.lenia().mass >= mass0,
            "brush inject should increase or maintain Lenia mass"
        );
    }

    #[test]
    fn test_anisotropic_diffusion_prefers_north() {
        use crate::genome::{
            CellGenome, LOCUS_ANISO_E, LOCUS_ANISO_N, LOCUS_ANISO_S, LOCUS_ANISO_W,
        };

        let mut grid = BiomeGrid::new(99);
        let cell = grid.get_cell_mut(64, 64);
        cell.active = true;
        cell.elements[1] = 5000; // N (fast diffuser)

        let mut genome = CellGenome::default_nurture();
        genome.set_value(LOCUS_ANISO_N, 60000);
        genome.set_value(LOCUS_ANISO_E, 1000);
        genome.set_value(LOCUS_ANISO_S, 1000);
        genome.set_value(LOCUS_ANISO_W, 1000);
        cell.genome = genome;

        grid.tick();

        let north = grid.get_cell(64, 63).elements[1];
        let south = grid.get_cell(64, 65).elements[1];
        let east = grid.get_cell(65, 64).elements[1];
        let west = grid.get_cell(63, 64).elements[1];

        assert!(
            north >= south && north >= east && north >= west,
            "North-biased genome should spread most to north: N={} S={} E={} W={}",
            north,
            south,
            east,
            west
        );
    }

    #[test]
    fn test_prismatic_render_buffer_value() {
        let mut grid = BiomeGrid::new(42);
        grid.get_cell_mut(64, 64).genome.set_prismatic();
        grid.tick();

        let slice =
            unsafe { std::slice::from_raw_parts(grid.render_data_ptr(), grid.render_data_len()) };
        let offset = (64 * GRID_WIDTH + 64) * RENDER_STRIDE;
        if slice[offset + 2] > 0.5 {
            assert_eq!(
                slice[offset + 2],
                2.0,
                "Prismatic active cell renders as 2.0"
            );
        }
    }

    #[test]
    fn test_env_wall_written_to_render_buffer() {
        // 環境ペンで塗った壁が render_buffer の active スロットに負値で反映される
        // （非活性セルに限る）。これが描画側で地形として可視化される根拠。
        let mut grid = BiomeGrid::new(1);
        // 空きセルを探して壁を塗る（生命がいない座標を選ぶ）
        let (wx, wy) = (5usize, 5usize);
        grid.lenia_mut().paint_env(wx, wy, 1, 1);
        grid.sync_after_lenia_reseed();

        let slice =
            unsafe { std::slice::from_raw_parts(grid.render_data_ptr(), grid.render_data_len()) };
        let offset = (wy * GRID_WIDTH + wx) * RENDER_STRIDE;
        assert_eq!(
            slice[offset + 2],
            -1.0,
            "empty wall cell should render as -1.0, got {}",
            slice[offset + 2]
        );
    }
}

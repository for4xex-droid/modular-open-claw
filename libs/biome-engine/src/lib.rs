use wasm_bindgen::prelude::*;

pub mod crisis;
pub mod element;
pub mod evolution;
pub mod genome;
pub mod grid;
pub mod particle;
pub mod rarity;

use crate::grid::BiomeGrid;
use crate::particle::SubstanceKind;
use crate::rarity::BiomeRarity;
use std::collections::VecDeque;

#[derive(Debug, Clone)]
enum HistoryEntry {
    Keyframe(Vec<crate::grid::BiomeCell>),
    Delta(Vec<(u16, crate::grid::BiomeCell)>),
}

struct BiomeHistory {
    entries: VecDeque<(u32, HistoryEntry)>,
    keyframe_interval: u32,
    max_entries: usize,
}

impl BiomeHistory {
    fn new(keyframe_interval: u32, max_entries: usize) -> Self {
        Self {
            entries: VecDeque::new(),
            keyframe_interval,
            max_entries,
        }
    }

    fn push(
        &mut self,
        gen: u32,
        current_cells: &[crate::grid::BiomeCell],
        prev_cells: &[crate::grid::BiomeCell],
    ) {
        let entry = if self.entries.is_empty() || gen.is_multiple_of(self.keyframe_interval) {
            HistoryEntry::Keyframe(current_cells.to_vec())
        } else {
            let mut diffs = Vec::new();
            for i in 0..current_cells.len() {
                if current_cells[i] != prev_cells[i] {
                    diffs.push((i as u16, current_cells[i].clone()));
                }
            }
            HistoryEntry::Delta(diffs)
        };

        self.entries.push_back((gen, entry));
        if self.entries.len() > self.max_entries {
            self.entries.pop_front();
        }
    }

    fn get_state(&self, target_gen: u32) -> Option<Vec<crate::grid::BiomeCell>> {
        let pos = self.entries.iter().position(|(g, _)| *g == target_gen)?;
        let keyframe_pos = self
            .entries
            .iter()
            .take(pos + 1)
            .rposition(|(_, entry)| matches!(entry, HistoryEntry::Keyframe(_)))?;

        let mut current_state = match &self.entries[keyframe_pos].1 {
            HistoryEntry::Keyframe(cells) => cells.clone(),
            _ => unreachable!(),
        };

        for i in (keyframe_pos + 1)..=pos {
            if let HistoryEntry::Delta(diffs) = &self.entries[i].1 {
                for &(idx, ref cell) in diffs {
                    current_state[idx as usize] = cell.clone();
                }
            }
        }

        Some(current_state)
    }

    fn truncate_after(&mut self, target_gen: u32) {
        if let Some(pos) = self.entries.iter().position(|(g, _)| *g == target_gen) {
            self.entries.truncate(pos);
        }
    }
}

#[wasm_bindgen]
pub struct BiomeEngine {
    grid: BiomeGrid,
    generation: u32,
    history: BiomeHistory,
    prev_tick_cells: Vec<crate::grid::BiomeCell>,
    forced_substance: Option<SubstanceKind>,
    forced_rarity: Option<BiomeRarity>,
}

#[wasm_bindgen]
impl BiomeEngine {
    #[wasm_bindgen(constructor)]
    pub fn new(seed: u64) -> Self {
        let grid = BiomeGrid::new(seed);
        let initial_cells = grid.current_cells().clone();
        Self {
            grid,
            generation: 0,
            history: BiomeHistory::new(20, 100),
            prev_tick_cells: initial_cells,
            forced_substance: None,
            forced_rarity: None,
        }
    }

    pub fn generation(&self) -> u32 {
        self.generation
    }

    pub fn tick(&mut self) {
        let current_cells = self.grid.current_cells().clone();
        self.history
            .push(self.generation, &current_cells, &self.prev_tick_cells);
        self.prev_tick_cells = current_cells;

        let _deltas = self.grid.tick();
        self.generation += 1;
    }

    pub fn apply_tachyon_rewind(&mut self, generations: u32) -> bool {
        if generations > self.generation {
            return false;
        }

        let target_gen = self.generation - generations;

        if let Some(restored_state) = self.history.get_state(target_gen) {
            self.grid.set_current_cells(restored_state.clone());
            self.prev_tick_cells = restored_state;
            self.generation = target_gen;
            self.history.truncate_after(target_gen);
            true
        } else {
            false
        }
    }

    pub fn render_data_ptr(&self) -> *const f32 {
        self.grid.render_data_ptr()
    }

    pub fn render_data_len(&self) -> usize {
        self.grid.render_data_len()
    }

    pub fn get_cell_detail(&self, x: usize, y: usize) -> JsValue {
        let cell = self.grid.get_cell(x, y);
        serde_wasm_bindgen::to_value(cell).unwrap_or(JsValue::NULL)
    }

    pub fn inject_element(&mut self, x: usize, y: usize, idx: usize, amount: u16) {
        if idx < 8 {
            let cell = self.grid.get_cell_mut(x, y);
            cell.elements[idx] = cell.elements[idx].saturating_add(amount);
            cell.active = true;
        }
    }

    pub fn apply_crisis(&mut self, crisis_type: &str, x: usize, y: usize) {
        let crisis = match crisis_type {
            "meteor" => crate::crisis::CrisisType::Meteor,
            "ice_age" => crate::crisis::CrisisType::IceAge,
            _ => crate::crisis::CrisisType::None,
        };
        crate::crisis::apply_crisis(&mut self.grid, crisis, x, y);
    }

    pub fn get_rarity(&self) -> BiomeRarity {
        if let Some(forced) = self.forced_rarity {
            return forced;
        }
        crate::rarity::determine_rarity(&self.grid)
    }

    pub fn debug_force_rarity(&mut self, rarity: BiomeRarity) {
        self.forced_rarity = Some(rarity);
    }

    pub fn get_active_cell_count(&self) -> u32 {
        let mut count = 0;
        let cells = self.grid.current_cells();
        for cell in cells {
            if cell.active {
                count += 1;
            }
        }
        count
    }

    pub fn get_element_balance(&self) -> Box<[u16]> {
        let mut totals = [0u64; 8];
        let cells = self.grid.current_cells();
        for cell in cells {
            if cell.active {
                for (i, total) in totals.iter_mut().enumerate() {
                    *total += cell.elements[i] as u64;
                }
            }
        }
        let sum: u64 = totals.iter().sum();
        let mut balance = vec![0u16; 8];
        if sum > 0 {
            for i in 0..8 {
                balance[i] = ((totals[i] * 100) / sum) as u16;
            }
        }
        balance.into_boxed_slice()
    }

    pub fn roll_substance(&self) -> SubstanceKind {
        if let Some(forced) = self.forced_substance {
            return forced;
        }
        let mut temp_rng = rand::thread_rng();
        crate::particle::roll_substance_discovery(&self.grid, &mut temp_rng)
    }

    pub fn debug_force_substance(&mut self, kind: SubstanceKind) {
        self.forced_substance = Some(kind);
    }

    pub fn serialize_genome(&self, x: usize, y: usize) -> String {
        let cell = self.grid.get_cell(x, y);
        serde_json::to_string(&cell.genome).unwrap_or_default()
    }

    pub fn set_mutation_boost(&mut self, val: f32) {
        self.grid.mutation_boost = val.clamp(1.0, 2.0);
    }

    pub fn get_mutation_boost(&self) -> f32 {
        self.grid.mutation_boost
    }

    pub fn ticks_since_mutation(&self) -> u32 {
        self.grid.ticks_since_mutation
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initial_generation() {
        let engine = BiomeEngine::new(42);
        assert_eq!(engine.generation(), 0);
    }

    #[test]
    fn test_tick_increments_generation() {
        let mut engine = BiomeEngine::new(42);
        engine.tick();
        assert_eq!(engine.generation(), 1);
    }

    #[test]
    fn test_tachyon_rewind_restores_state() {
        let mut engine = BiomeEngine::new(42);

        // 30世代進める
        for _ in 0..30 {
            engine.tick();
        }
        assert_eq!(engine.generation(), 30);

        // 20世代巻き戻す
        let success = engine.apply_tachyon_rewind(20);

        // 巻き戻しが成功し、世代が 10 に戻っていることを期待。
        // 現在はスタブなので、このテストは失敗する (RED)
        assert!(success, "Rewind should succeed");
        assert_eq!(
            engine.generation(),
            10,
            "Generation should be restored to 10"
        );
    }
}

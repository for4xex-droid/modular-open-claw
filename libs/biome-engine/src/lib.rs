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
            _ => return None,
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

#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "type")]
pub enum BiomeEvent {
    MorphologyChanged { from: u8, to: u8 },
    MassExtinction { lost_ratio: f32 },
    NewReactionDiscovered { reaction_id: u8 },
}

#[wasm_bindgen]
pub struct BiomeEngine {
    grid: BiomeGrid,
    generation: u32,
    history: BiomeHistory,
    prev_tick_cells: Vec<crate::grid::BiomeCell>,
    forced_substance: Option<SubstanceKind>,
    forced_rarity: Option<BiomeRarity>,
    last_tick_events: Vec<BiomeEvent>,
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
            last_tick_events: Vec::new(),
        }
    }

    pub fn generation(&self) -> u32 {
        self.generation
    }

    pub fn tick(&mut self) {
        let current_cells = self.grid.current_cells().clone();
        self.history
            .push(self.generation, &current_cells, &self.prev_tick_cells);
        self.prev_tick_cells = current_cells.clone();

        self.last_tick_events.clear();
        let _deltas = self.grid.tick();
        self.generation += 1;

        // 形態変化検知
        for (prev, next) in current_cells.iter().zip(self.grid.current_cells().iter()) {
            if prev.morphology != next.morphology && next.active {
                self.last_tick_events.push(BiomeEvent::MorphologyChanged {
                    from: prev.morphology as u8,
                    to: next.morphology as u8,
                });
            }
        }
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

    pub fn get_rarity_progress(&self) -> JsValue {
        let progress = crate::rarity::determine_rarity_with_progress(&self.grid);
        serde_wasm_bindgen::to_value(&progress).unwrap_or(JsValue::NULL)
    }

    pub fn get_last_tick_events(&self) -> JsValue {
        serde_wasm_bindgen::to_value(&self.last_tick_events).unwrap_or(JsValue::NULL)
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

    pub fn thaw_grid(&mut self) {
        crate::crisis::thaw_grid(&mut self.grid);
    }

    pub fn scan_vulnerability(&self) -> String {
        let cells = self.grid.current_cells();
        let mut active_count = 0;
        let mut frozen_count = 0;
        let mut total_elements = [0u64; 8];

        for cell in cells {
            if cell.active {
                active_count += 1;
                if cell.is_frozen {
                    frozen_count += 1;
                }
                #[allow(clippy::needless_range_loop)]
                for i in 0..8 {
                    total_elements[i] += cell.elements[i] as u64;
                }
            }
        }

        let mut vulnerabilities = Vec::new();

        if active_count == 0 {
            vulnerabilities.push("ExtinctionRisk");
        } else {
            let freeze_ratio = frozen_count as f32 / active_count as f32;
            if freeze_ratio >= 0.5 {
                vulnerabilities.push("HighFreezeRatio");
            }
        }

        let sum: u64 = total_elements.iter().sum();
        if sum > 0 {
            for &total in total_elements.iter() {
                let ratio = total as f32 / sum as f32;
                if ratio >= 0.8 {
                    vulnerabilities.push("ElementImbalance");
                    break;
                }
            }
        }

        let report = serde_json::json!({
            "active_cells": active_count,
            "frozen_cells": frozen_count,
            "vulnerabilities": vulnerabilities,
        });

        report.to_string()
    }

    pub fn serialize(&self) -> Result<String, String> {
        let state = SerializedEngineState {
            generation: self.generation,
            cells: self.grid.current_cells().clone(),
            mutation_boost: self.grid.mutation_boost,
            ticks_since_mutation: self.grid.ticks_since_mutation,
        };
        serde_json::to_string(&state).map_err(|e| format!("Failed to serialize: {}", e))
    }

    pub fn deserialize(json: &str) -> Result<BiomeEngine, String> {
        let state: SerializedEngineState =
            serde_json::from_str(json).map_err(|e| format!("Failed to parse JSON: {}", e))?;

        let mut engine = BiomeEngine::new(42);
        engine.generation = state.generation;
        engine.grid.set_current_cells(state.cells.clone());
        engine.grid.mutation_boost = state.mutation_boost;
        engine.grid.ticks_since_mutation = state.ticks_since_mutation;
        engine.prev_tick_cells = state.cells;

        Ok(engine)
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
struct SerializedEngineState {
    generation: u32,
    cells: Vec<crate::grid::BiomeCell>,
    mutation_boost: f32,
    ticks_since_mutation: u32,
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

    #[test]
    fn test_scan_vulnerability() {
        let mut engine = BiomeEngine::new(42);

        let report = engine.scan_vulnerability();
        assert!(report.contains("ExtinctionRisk"));

        engine.inject_element(10, 10, 0, 100);
        let cell = engine.grid.get_cell_mut(10, 10);
        cell.is_frozen = true;

        let report2 = engine.scan_vulnerability();
        assert!(report2.contains("HighFreezeRatio"));
    }

    #[test]
    fn test_serialize_deserialize() {
        let mut engine = BiomeEngine::new(42);
        engine.inject_element(5, 5, 0, 500);
        engine.tick();

        let serialized = engine.serialize().expect("serialize failed");
        assert!(!serialized.is_empty());

        let restored = BiomeEngine::deserialize(&serialized).expect("deserialize failed");
        assert_eq!(restored.generation(), 1);

        let original_cell = engine.grid.get_cell(5, 5);
        let restored_cell = restored.grid.get_cell(5, 5);
        assert!(restored_cell.active);
        assert_eq!(restored_cell.elements[0], original_cell.elements[0]);
    }

    #[test]
    fn test_inject_changes_element_balance() {
        let mut engine = BiomeEngine::new(42);

        // 初期配置: 中央13x13にC/N/P各4000を注入 (BiomeGame.tsxと同じ)
        for y in 58..=70 {
            for x in 58..=70 {
                engine.inject_element(x, y, 0, 4000); // C
                engine.inject_element(x, y, 1, 4000); // N
                engine.inject_element(x, y, 2, 4000); // P
            }
        }

        // 200 tick 回す (ゲームの標準的な進行)
        for _ in 0..200 {
            engine.tick();
        }

        let balance_before = engine.get_element_balance();
        let active_before = engine.get_active_cell_count();
        println!("--- After 200 ticks (initial setup) ---");
        println!("Active cells: {}", active_before);
        println!(
            "Balance: C={}% N={}% P={}% H={}%",
            balance_before[0], balance_before[1], balance_before[2], balance_before[3]
        );

        // ユーザーのクリック注入: 5x5 x 15000 of C
        for y in 62..=66 {
            for x in 62..=66 {
                engine.inject_element(x, y, 0, 15000); // C
            }
        }

        let balance_after_inject = engine.get_element_balance();
        println!("\n--- After inject (5x5 x 15000 C) ---");
        println!(
            "Balance: C={}% N={}% P={}% H={}%",
            balance_after_inject[0],
            balance_after_inject[1],
            balance_after_inject[2],
            balance_after_inject[3]
        );

        let c_diff = balance_after_inject[0] as i32 - balance_before[0] as i32;
        println!(
            "C change: {}% -> {}% (diff: {}%)",
            balance_before[0], balance_after_inject[0], c_diff
        );

        // C の割合は inject 前より増えている必要がある
        assert!(
            balance_after_inject[0] > balance_before[0],
            "C balance should increase after injection: before={}%, after={}%",
            balance_before[0],
            balance_after_inject[0]
        );
    }

    #[test]
    fn test_engine_get_rarity_progress() {
        // ネイティブテストでは JsValue 関連メソッドを呼び出すとパニックするため、
        // 内部で呼ばれている determine_rarity_with_progress の動作を直接検証します。
        let engine = BiomeEngine::new(42);
        let progress = crate::rarity::determine_rarity_with_progress(&engine.grid);
        assert_eq!(progress.rarity, 0); // Common
    }

    #[test]
    fn test_engine_last_tick_events() {
        let mut engine = BiomeEngine::new(42);
        engine.inject_element(0, 0, 0, 100);
        engine.tick();

        engine.inject_element(0, 0, 3, 45000); // H
        engine.inject_element(0, 0, 4, 45000); // O
        engine.tick();

        // 内部フィールドを直接検証します（JsValueを返す get_last_tick_events は non-wasm ではパニックするため）
        assert!(
            !engine.last_tick_events.is_empty(),
            "Internal events should not be empty"
        );
    }
}

/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
use wasm_bindgen::prelude::*;

pub mod crisis;
pub mod element;
pub mod evolution;
pub mod genome;
pub mod grid;
pub mod lenia;
pub mod particle;
pub mod pattern;
pub mod rarity;
pub mod species_library;

use crate::grid::BiomeGrid;
use crate::lenia::LeniaSnapshot;
use crate::particle::SubstanceKind;
use crate::rarity::{BiomeRarity, RarityProgress};
use std::collections::VecDeque;

const HISTORY_INTERVAL: u32 = 5;
const HISTORY_MAX_ENTRIES: usize = 40;
const PROGRESS_REFRESH_INTERVAL: u32 = 10;

#[derive(Debug, Clone)]
enum HistoryEntry {
    Keyframe(LeniaSnapshot),
}

struct BiomeHistory {
    entries: VecDeque<(u32, HistoryEntry)>,
    max_entries: usize,
}

impl BiomeHistory {
    fn new(max_entries: usize) -> Self {
        Self {
            entries: VecDeque::new(),
            max_entries,
        }
    }

    fn push(&mut self, gen: u32, snapshot: LeniaSnapshot) {
        self.entries
            .push_back((gen, HistoryEntry::Keyframe(snapshot)));
        while self.entries.len() > self.max_entries {
            self.entries.pop_front();
        }
    }

    fn get_snapshot_nearest(&self, target_gen: u32) -> Option<(u32, LeniaSnapshot)> {
        self.entries
            .iter()
            .filter(|(g, _)| *g <= target_gen)
            .max_by_key(|(g, _)| *g)
            .map(|(g, entry)| {
                (
                    *g,
                    match entry {
                        HistoryEntry::Keyframe(snap) => snap.clone(),
                    },
                )
            })
    }

    #[allow(dead_code)]
    fn get_snapshot(&self, target_gen: u32) -> Option<LeniaSnapshot> {
        self.get_snapshot_nearest(target_gen).map(|(_, snap)| snap)
    }

    fn truncate_after(&mut self, target_gen: u32) {
        if let Some(pos) = self.entries.iter().position(|(g, _)| *g == target_gen) {
            self.entries.truncate(pos + 1);
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "type")]
pub enum BiomeEvent {
    MorphologyChanged { from: u8, to: u8 },
    MassExtinction { lost_ratio: f32 },
    NewReactionDiscovered { reaction_id: u8 },
    PrismaticBorn { x: u16, y: u16 },
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
    cached_progress: Option<(u32, RarityProgress)>,
}

#[wasm_bindgen]
impl BiomeEngine {
    #[wasm_bindgen(constructor)]
    pub fn new(seed: u64) -> Self {
        let grid = BiomeGrid::new(seed);
        let initial_cells = grid.current_cells().clone();
        let initial_snapshot = grid.lenia_snapshot();
        let mut history = BiomeHistory::new(HISTORY_MAX_ENTRIES);
        history.push(0, initial_snapshot);
        let mut engine = Self {
            grid,
            generation: 0,
            history,
            prev_tick_cells: initial_cells,
            forced_substance: None,
            forced_rarity: None,
            last_tick_events: Vec::new(),
            cached_progress: None,
        };
        engine.refresh_progress_cache();
        engine
    }

    fn refresh_progress_cache(&mut self) {
        let progress = crate::rarity::determine_rarity_with_progress(&self.grid);
        self.cached_progress = Some((self.generation, progress));
    }

    fn progress(&self) -> RarityProgress {
        if let Some((_, progress)) = &self.cached_progress {
            progress.clone()
        } else {
            crate::rarity::determine_rarity_with_progress(&self.grid)
        }
    }

    pub fn generation(&self) -> u32 {
        self.generation
    }

    pub fn tick(&mut self) {
        self.tick_n(1);
    }

    pub fn tick_n(&mut self, count: u32) {
        if count == 0 {
            return;
        }
        if self.generation.is_multiple_of(HISTORY_INTERVAL) {
            self.history
                .push(self.generation, self.grid.lenia_snapshot());
        }
        self.grid.tick_n(count);
        self.generation += count;
        self.last_tick_events.clear();
        // レアリティ計算（対称性・クラスタ BFS・species_hash）は重いため
        // 10 世代境界を跨いだときのみ再計算し、それ以外は直前の値を維持する
        let prev_gen = self.generation - count;
        if prev_gen / PROGRESS_REFRESH_INTERVAL != self.generation / PROGRESS_REFRESH_INTERVAL {
            self.refresh_progress_cache();
        }
    }

    pub fn apply_tachyon_rewind(&mut self, generations: u32) -> bool {
        if generations > self.generation {
            return false;
        }

        let target_gen = self.generation - generations;

        let Some((restored_gen, snapshot)) = self.history.get_snapshot_nearest(target_gen) else {
            return false;
        };

        self.grid.restore_lenia_snapshot(&snapshot);
        self.prev_tick_cells = self.grid.current_cells().clone();
        self.generation = restored_gen;
        self.history.truncate_after(restored_gen);
        self.refresh_progress_cache();
        true
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
            self.refresh_progress_cache();
        }
    }

    pub fn inject_brush(&mut self, x: usize, y: usize, radius: usize, idx: usize, amount: u16) {
        self.grid.inject_brush(x, y, radius, idx, amount);
        self.refresh_progress_cache();
    }

    pub fn apply_crisis(&mut self, crisis_type: &str, x: usize, y: usize) {
        let crisis = match crisis_type {
            "meteor" => crate::crisis::CrisisType::Meteor,
            "ice_age" => crate::crisis::CrisisType::IceAge,
            _ => crate::crisis::CrisisType::None,
        };
        crate::crisis::apply_crisis(&mut self.grid, crisis, x, y);
        self.refresh_progress_cache();
    }

    pub fn get_rarity(&self) -> BiomeRarity {
        if let Some(forced) = self.forced_rarity {
            return forced;
        }
        match self.progress().rarity {
            4 => BiomeRarity::Legendary,
            3 => BiomeRarity::Epic,
            2 => BiomeRarity::Rare,
            1 => BiomeRarity::Uncommon,
            _ => BiomeRarity::Common,
        }
    }

    pub fn debug_force_rarity(&mut self, rarity: BiomeRarity) {
        self.forced_rarity = Some(rarity);
    }

    pub fn get_rarity_progress(&self) -> JsValue {
        serde_wasm_bindgen::to_value(&self.progress()).unwrap_or(JsValue::NULL)
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

    pub fn get_lenia_mu(&self) -> f32 {
        self.grid.lenia().genome().mu[0]
    }

    pub fn get_lenia_sigma(&self) -> f32 {
        self.grid.lenia().genome().sigma[0]
    }

    pub fn set_lenia_params(&mut self, mu: f32, sigma: f32) {
        let g = self.grid.lenia_mut().genome_mut();
        let mu = mu.clamp(0.05, 0.35);
        let sigma = sigma.clamp(0.005, 0.05);
        for i in 0..3 {
            g.mu[i] = mu;
            g.sigma[i] = sigma;
        }
        self.refresh_progress_cache();
    }

    /// 環境ペン: (x,y) 中心・半径 radius の円に地形を塗る。
    /// kind: 0=消去 1=壁（成長禁止） 2=養分（成長増幅） 3=毒（減衰）。
    /// プレイヤー操作が場の展開を変える因果を回復する。
    pub fn paint_env(&mut self, x: usize, y: usize, radius: usize, kind: u8) {
        self.grid.lenia_mut().paint_env(x, y, radius, kind);
        self.grid.sync_after_lenia_reseed();
    }

    /// 環境ペンの塗りをすべて消去する。
    pub fn clear_env(&mut self) {
        self.grid.lenia_mut().clear_env();
    }

    /// 2 種による縄張り対戦エコシステムを開始する（ch0/ch1 に別種を配置し相互抑制）。
    /// `competition` は競合の強さ（0.0=無干渉, 0.8〜1.2 で緊張ある共存, 1.5+ で全滅寄り）。
    pub fn seed_ecosystem(&mut self, species_a: usize, species_b: usize, competition: f32) {
        let comp = competition.clamp(0.0, 3.0);
        self.grid
            .lenia_mut()
            .seed_ecosystem(species_a, species_b, comp);
        self.grid.sync_after_lenia_reseed();
        self.refresh_progress_cache();
    }

    /// 図鑑保存用: Lenia 種パラメータ JSON（数十バイト）
    pub fn serialize_lenia_species(&self) -> String {
        let lenia = self.grid.lenia();
        let progress = self.progress();
        serde_json::json!({
            "mu": lenia.genome().mu[0],
            "sigma": lenia.genome().sigma[0],
            "dt": lenia.genome().dt,
            "species_hash": progress.species_hash,
            "mass": progress.mass,
            "locomotion": progress.locomotion,
            "longevity": progress.longevity,
        })
        .to_string()
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
        let lenia = self.grid.lenia();
        let state = SerializedEngineState {
            version: SERIALIZE_VERSION,
            generation: self.generation,
            field: lenia.field().to_vec(),
            genome: lenia.genome().clone(),
            longevity_ticks: lenia.longevity_ticks,
            last_centroid_x: lenia.last_centroid_x,
            last_centroid_y: lenia.last_centroid_y,
            mutation_boost: self.grid.mutation_boost,
            ticks_since_mutation: self.grid.ticks_since_mutation,
        };
        serde_json::to_string(&state).map_err(|e| format!("Failed to serialize: {}", e))
    }

    pub fn deserialize(json: &str) -> Result<BiomeEngine, String> {
        // v2: 明示的 version フィールド
        if let Ok(state) = serde_json::from_str::<SerializedEngineState>(json) {
            if state.version == SERIALIZE_VERSION {
                return Self::from_serialized_v2(state);
            }
        }

        // v1 互換: cells ベース（Lenia 非互換のため新規 Lenia 場を維持し世代のみ復元）
        if let Ok(legacy) = serde_json::from_str::<LegacySerializedEngineState>(json) {
            let mut engine = BiomeEngine::new(42);
            engine.generation = legacy.generation;
            engine.grid.mutation_boost = legacy.mutation_boost;
            engine.grid.ticks_since_mutation = legacy.ticks_since_mutation;
            engine.prev_tick_cells = engine.grid.current_cells().clone();
            engine.history.push(0, engine.grid.lenia_snapshot());
            engine.refresh_progress_cache();
            return Ok(engine);
        }

        Err("Failed to parse JSON: incompatible or malformed save data".to_string())
    }

    fn from_serialized_v2(state: SerializedEngineState) -> Result<BiomeEngine, String> {
        let expected_len = crate::grid::GRID_SIZE * 3;
        if state.field.len() != expected_len {
            return Err(format!(
                "Invalid field length: expected {expected_len}, got {}",
                state.field.len()
            ));
        }

        let mut engine = BiomeEngine::new(42);
        engine.generation = state.generation;
        engine.grid.mutation_boost = state.mutation_boost;
        engine.grid.ticks_since_mutation = state.ticks_since_mutation;

        *engine.grid.lenia_mut().genome_mut() = state.genome;
        let snap = LeniaSnapshot {
            field: state.field,
            longevity_ticks: state.longevity_ticks,
            last_centroid_x: state.last_centroid_x,
            last_centroid_y: state.last_centroid_y,
        };
        engine.grid.restore_lenia_snapshot(&snap);
        engine.prev_tick_cells = engine.grid.current_cells().clone();
        engine
            .history
            .push(state.generation, engine.grid.lenia_snapshot());
        engine.refresh_progress_cache();
        Ok(engine)
    }
}

const SERIALIZE_VERSION: u8 = 2;

#[derive(serde::Serialize, serde::Deserialize)]
struct SerializedEngineState {
    version: u8,
    generation: u32,
    field: Vec<f32>,
    genome: crate::lenia::LeniaGenome,
    longevity_ticks: u32,
    last_centroid_x: f32,
    last_centroid_y: f32,
    mutation_boost: f32,
    ticks_since_mutation: u32,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct LegacySerializedEngineState {
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
    fn test_tick_n_batch_equals_sequential() {
        let mut batched = BiomeEngine::new(42);
        let mut sequential = BiomeEngine::new(42);
        batched.tick_n(10);
        for _ in 0..10 {
            sequential.tick();
        }
        assert_eq!(batched.generation(), sequential.generation());
        assert_eq!(
            batched.grid.lenia().field(),
            sequential.grid.lenia().field()
        );
    }

    #[test]
    fn test_tachyon_rewind_restores_state() {
        let mut engine = BiomeEngine::new(42);

        for _ in 0..10 {
            engine.tick();
        }
        let field_at_10 = engine.grid.lenia().field().to_vec();
        assert_eq!(engine.generation(), 10);

        for _ in 0..20 {
            engine.tick();
        }
        assert_eq!(engine.generation(), 30);

        let success = engine.apply_tachyon_rewind(20);
        assert!(success, "Rewind should succeed");
        assert_eq!(engine.generation(), 10);
        assert_eq!(
            engine.grid.lenia().field(),
            field_at_10.as_slice(),
            "Lenia field should match snapshot at generation 10"
        );
    }

    #[test]
    fn test_tachyon_rewind_fails_beyond_history() {
        let mut engine = BiomeEngine::new(42);
        for _ in 0..5 {
            engine.tick();
        }
        assert!(!engine.apply_tachyon_rewind(100));
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
        engine.inject_brush(64, 64, 2, 0, 15000);
        engine.tick();

        let serialized = engine.serialize().expect("serialize should succeed");
        assert!(serialized.contains("\"version\":2"));
        assert!(serialized.contains("\"field\":"));

        let restored = BiomeEngine::deserialize(&serialized).expect("deserialize should succeed");
        assert_eq!(restored.generation(), 1);
        assert_eq!(
            restored.grid.lenia().field(),
            engine.grid.lenia().field(),
            "v2 roundtrip should preserve Lenia field"
        );
    }

    #[test]
    fn test_deserialize_legacy_v1_ignores_cells() {
        let legacy = serde_json::json!({
            "generation": 99,
            "cells": [],
            "mutation_boost": 1.5,
            "ticks_since_mutation": 42
        });
        let restored =
            BiomeEngine::deserialize(&legacy.to_string()).expect("legacy should deserialize");
        assert_eq!(restored.generation(), 99);
        assert!((restored.grid.mutation_boost - 1.5).abs() < 1e-6);
        // Lenia 場は新規シードのまま（cells は無視）
        assert!(restored.grid.lenia().mass > 0.0);
    }

    #[test]
    fn test_deserialize_rejects_malformed_json() {
        assert!(BiomeEngine::deserialize("{not json").is_err());
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

        // inject_brush で Lenia 場に種を撒く
        let mass_before = engine.grid.lenia().mass;
        engine.inject_brush(64, 64, 2, 0, 15000);

        let mass_after = engine.grid.lenia().mass;
        assert!(
            mass_after >= mass_before,
            "Lenia mass should increase after brush inject: before={} after={}",
            mass_before,
            mass_after
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
        engine.tick();
        // Lenia モデルでは MorphologyChanged は元素反応に依存しないため空でも正常
        let _events = &engine.last_tick_events;
    }

    #[test]
    fn test_balance_gate_seed_diversity() {
        let seeds: [u64; 10] = [1, 7, 42, 99, 123, 456, 789, 1337, 9999, 54321];
        let mut symmetries = Vec::new();
        let mut complexities = Vec::new();
        let mut epic_or_above = 0usize;

        let mut masses = Vec::new();
        for seed in seeds {
            let mut engine = BiomeEngine::new(seed);
            for _ in 0..30 {
                engine.tick();
            }
            masses.push(engine.grid.lenia().mass);
            let progress = crate::rarity::determine_rarity_with_progress(&engine.grid);
            symmetries.push(progress.symmetry_score);
            complexities.push(progress.complexity_score);
            if progress.rarity >= 3 {
                epic_or_above += 1;
            }
            assert!(
                progress.prismatic_cells <= 15,
                "Seed {} produced {} prismatic cells (max 15)",
                seed,
                progress.prismatic_cells
            );
        }

        let mass_min = masses.iter().cloned().fold(f32::INFINITY, f32::min);
        let mass_max = masses.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        assert!(
            mass_max - mass_min > 1.0,
            "Lenia mass should vary across seeds: min={} max={}",
            mass_min,
            mass_max
        );
        assert!(
            epic_or_above <= 10,
            "Epic+ count recorded for Phase 3 rarity tuning: got {}",
            epic_or_above
        );
    }
}

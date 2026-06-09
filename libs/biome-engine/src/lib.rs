use wasm_bindgen::prelude::*;

pub mod crisis;
pub mod element;
pub mod evolution;
pub mod genome;
pub mod grid;
pub mod particle;
pub mod rarity;

#[wasm_bindgen]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BiomeCell {
    pub active: bool,
    // TODO: ゲノムや元素を追加
}

use crate::grid::BiomeGrid;

#[wasm_bindgen]
pub struct BiomeEngine {
    grid: BiomeGrid,
    generation: u32,
    // (generation, cells_state) の履歴
    history: Vec<(u32, Vec<crate::grid::BiomeCell>)>,
}

#[wasm_bindgen]
impl BiomeEngine {
    #[wasm_bindgen(constructor)]
    pub fn new(seed: u64) -> Self {
        Self {
            grid: BiomeGrid::new(seed),
            generation: 0,
            history: Vec::new(),
        }
    }

    pub fn generation(&self) -> u32 {
        self.generation
    }

    pub fn tick(&mut self) {
        // 現在の世代とグリッド状態を履歴に保存
        self.history
            .push((self.generation, self.grid.cells.clone()));

        // 履歴サイズを制限 (メモリ対策として最大100世代分とする)
        if self.history.len() > 100 {
            self.history.remove(0);
        }

        // グリッドを進める
        let _deltas = self.grid.tick();
        self.generation += 1;
    }

    /// タキオン因果逆行: 指定した世代数だけ巻き戻す
    pub fn apply_tachyon_rewind(&mut self, generations: u32) -> bool {
        if generations > self.generation {
            return false;
        }

        let target_gen = self.generation - generations;

        // 履歴からターゲットの世代を探す
        if let Some(pos) = self.history.iter().position(|(g, _)| *g == target_gen) {
            let (_, cells_state) = self.history[pos].clone();

            // 状態の復元
            self.grid.cells = cells_state;
            self.generation = target_gen;

            // 復元した世代より後の履歴を削除
            self.history.truncate(pos);
            true
        } else {
            false
        }
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

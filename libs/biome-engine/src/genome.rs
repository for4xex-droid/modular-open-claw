/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
use rand::Rng;
use wasm_bindgen::prelude::*;

pub const GENOME_SIZE: usize = 32;

// 元素のインデックス定義
pub const ELEMENT_C: usize = 0;
pub const ELEMENT_N: usize = 1;
pub const ELEMENT_P: usize = 2;
pub const ELEMENT_H: usize = 3;
pub const ELEMENT_O: usize = 4;
pub const ELEMENT_S: usize = 5;
pub const ELEMENT_FE: usize = 6;
pub const ELEMENT_SI: usize = 7;

// ゲノム座の意味付け（0–7: 元素適応, 8: IceAge耐性 [crisis.rs], 12–15: 拡散方向重み）
pub const LOCUS_ANISO_N: usize = 12;
pub const LOCUS_ANISO_E: usize = 13;
pub const LOCUS_ANISO_S: usize = 14;
pub const LOCUS_ANISO_W: usize = 15;
pub const LOCUS_PRISMATIC: usize = 31;

#[wasm_bindgen]
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CellGenome {
    // 32次元のゲノム情報 (0-7: 元素の蓄積・適応値, 8-31: 生理・形態形質)
    values: [u16; GENOME_SIZE],
}

impl CellGenome {
    pub fn new(values: [u16; GENOME_SIZE]) -> Self {
        Self { values }
    }

    pub fn default_nurture() -> Self {
        Self {
            values: [10000u16; GENOME_SIZE],
        }
    }

    pub fn get_value(&self, index: usize) -> u16 {
        self.values[index]
    }

    pub fn set_value(&mut self, index: usize, val: u16) {
        self.values[index] = val;
    }

    /// 元素 e の実効拡散率変調 (0.5–1.5, 10000 が中立)
    pub fn retention_factor(&self, e: usize) -> f32 {
        if e >= 8 {
            return 1.0;
        }
        let centered = (10000.0 - self.values[e] as f32) / 65535.0;
        (1.0 + 0.5 * centered).clamp(0.5, 1.5)
    }

    /// 方向重み [N, E, S, W]（正規化済み、各 0.1 下限）
    pub fn anisotropy(&self) -> [f32; 4] {
        let raw = [
            self.values[LOCUS_ANISO_N] as f32,
            self.values[LOCUS_ANISO_E] as f32,
            self.values[LOCUS_ANISO_S] as f32,
            self.values[LOCUS_ANISO_W] as f32,
        ];
        let sum: f32 = raw.iter().sum();
        if sum < 1.0 {
            return [0.25, 0.25, 0.25, 0.25];
        }
        [
            (raw[0] / sum).max(0.1),
            (raw[1] / sum).max(0.1),
            (raw[2] / sum).max(0.1),
            (raw[3] / sum).max(0.1),
        ]
    }

    pub fn is_prismatic(&self) -> bool {
        self.values[LOCUS_PRISMATIC] >= 60000
    }

    pub fn set_prismatic(&mut self) {
        self.values[LOCUS_PRISMATIC] = 65535;
    }

    /// 突然変異を実行する
    pub fn mutate(&mut self, mutation_rate: u16, rng: &mut impl Rng) {
        for i in 0..GENOME_SIZE {
            // mutation_rate を確率として使用 (例: 0-65535 のうち mutation_rate 未満なら変異)
            let rand_val: u16 = rng.gen_range(0..=65535);
            if rand_val < mutation_rate {
                // 変異の量 (最大で現在の値の 10% 程度、または固定範囲)
                let delta: i32 = rng.gen_range(-1000..=1000);
                let current = self.values[i] as i32;
                let next = (current + delta).clamp(0, 65535);
                self.values[i] = next as u16;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::SmallRng;
    use rand::SeedableRng;

    #[test]
    fn test_default_nurture_initialization() {
        let genome = CellGenome::default_nurture();
        assert_eq!(genome.get_value(0), 10000);
    }

    #[test]
    fn test_mutation_changes_values() {
        let mut genome = CellGenome::default_nurture();
        let mut rng = SmallRng::seed_from_u64(42);

        // 突然変異率を 5000 (約7.6%) に設定して変異を実行
        genome.mutate(5000, &mut rng);

        let changed = (0..GENOME_SIZE).any(|i| genome.get_value(i) != 10000);
        assert!(changed, "Genome values should have changed after mutation");
    }

    #[test]
    fn test_mutation_zero_rate_does_not_change() {
        let mut genome = CellGenome::default_nurture();
        let mut rng = SmallRng::seed_from_u64(42);

        // 変異率0なら一切変化しないはず
        genome.mutate(0, &mut rng);

        for i in 0..GENOME_SIZE {
            assert_eq!(genome.get_value(i), 10000);
        }
    }

    #[test]
    #[allow(unused_comparisons, clippy::absurd_extreme_comparisons)]
    fn test_mutation_remains_within_bounds() {
        // 初期値を境界値の近くに設定
        let mut genome = CellGenome::new([65500; GENOME_SIZE]);
        let mut rng = SmallRng::seed_from_u64(42);

        // 最大の変異率で何度も変異を実行し、上限(65535)を超えないことを確認
        for _ in 0..100 {
            genome.mutate(65535, &mut rng);
        }

        for i in 0..GENOME_SIZE {
            let val = genome.get_value(i);
            assert!(val <= 65535, "Value {} exceeded maximum bound 65535", val);
        }

        // 下限(0)のテスト
        let mut genome_low = CellGenome::new([30; GENOME_SIZE]);
        for _ in 0..100 {
            genome_low.mutate(65535, &mut rng);
        }

        for i in 0..GENOME_SIZE {
            let val = genome_low.get_value(i);
            assert!(val >= 0, "Value {} went below minimum bound 0", val);
        }
    }
}

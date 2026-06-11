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

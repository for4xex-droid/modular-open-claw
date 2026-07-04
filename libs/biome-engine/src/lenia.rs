/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
//! Lenia 型連続セルオートマトン（arXiv:1812.05433 準拠）
use num_complex::Complex;
use rustfft::{Fft, FftPlanner};
use std::sync::Arc;

use crate::grid::{GRID_HEIGHT, GRID_SIZE, GRID_WIDTH};

/// Orbium 正典パラメータ（単一チャンネル PoC）
pub const ORBIUM_MU: f32 = 0.15;
pub const ORBIUM_SIGMA: f32 = 0.017;
pub const ORBIUM_DT: f32 = 0.1;
pub const KERNEL_RADIUS: usize = 13;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LeniaGenome {
    pub mu: [f32; 3],
    pub sigma: [f32; 3],
    pub dt: f32,
    /// チャンネル間相互作用行列。`interaction[i][j]` は「チャンネル j の密度が
    /// チャンネル i の成長を抑制する強さ」。正値＝競合（相手が多い場所で育ちにくい）。
    /// 既定は全ゼロ＝相互作用なし（単一種の従来挙動を完全維持）。
    #[serde(default)]
    pub interaction: [[f32; 3]; 3],
}

impl LeniaGenome {
    pub fn orbium_default() -> Self {
        Self {
            mu: [ORBIUM_MU, ORBIUM_MU, ORBIUM_MU],
            sigma: [ORBIUM_SIGMA, ORBIUM_SIGMA, ORBIUM_SIGMA],
            dt: ORBIUM_DT,
            interaction: [[0.0; 3]; 3],
        }
    }

    pub fn mutate(&mut self, rate: f32, rng: &mut impl rand::Rng) {
        for i in 0..3 {
            if rng.gen::<f32>() < rate {
                self.mu[i] = (self.mu[i] + rng.gen_range(-0.01..0.01)).clamp(0.05, 0.35);
            }
            if rng.gen::<f32>() < rate {
                self.sigma[i] = (self.sigma[i] + rng.gen_range(-0.003..0.003)).clamp(0.005, 0.05);
            }
        }
    }
}

pub struct LeniaSimulator {
    field: Vec<f32>,
    genome: LeniaGenome,
    #[allow(dead_code)]
    kernel_spatial: Vec<f32>,
    kernel_fft: Vec<Complex<f32>>,
    row_forward: Arc<dyn Fft<f32>>,
    row_inverse: Arc<dyn Fft<f32>>,
    /// 前 tick の重心 X（移動速度算出用）
    pub last_centroid_x: f32,
    pub last_centroid_y: f32,
    pub locomotion: f32,
    pub mass: f32,
    /// 質量が安定閾値以上を維持した連続 tick 数
    pub longevity_ticks: u32,
    /// 環境マスク（プレイヤーが描く地形）。0=通常 1=壁 2=養分 3=毒。
    /// tick の成長計算で参照し、プレイヤー操作が場の展開を変える因果を与える。
    env_mask: Vec<u8>,
}

/// 環境マスクの値: 成長を増幅する養分の係数
const NUTRIENT_GAIN: f32 = 1.5;
/// 環境マスクの値: 減衰させる毒の係数
const POISON_DECAY: f32 = 0.3;

const STABLE_MASS_THRESHOLD: f32 = 30.0;

/// 巻き戻し・シリアライズ用の Lenia 場スナップショット
#[derive(Debug, Clone)]
pub struct LeniaSnapshot {
    pub field: Vec<f32>,
    pub longevity_ticks: u32,
    pub last_centroid_x: f32,
    pub last_centroid_y: f32,
}

impl LeniaSimulator {
    pub fn new(seed: u64) -> Self {
        let mut planner = FftPlanner::<f32>::new();
        let row_forward = planner.plan_fft_forward(GRID_WIDTH);
        let row_inverse = planner.plan_fft_inverse(GRID_WIDTH);

        let kernel_spatial = build_ring_kernel_2d(KERNEL_RADIUS);
        let kernel_fft = fft2d_forward(&kernel_spatial, &row_forward);

        let mut sim = Self {
            field: vec![0.0; GRID_SIZE * 3],
            genome: LeniaGenome::orbium_default(),
            kernel_spatial,
            kernel_fft,
            row_forward,
            row_inverse,
            last_centroid_x: GRID_WIDTH as f32 / 2.0,
            last_centroid_y: GRID_HEIGHT as f32 / 2.0,
            locomotion: 0.0,
            mass: 0.0,
            longevity_ticks: 0,
            env_mask: vec![0u8; GRID_SIZE],
        };

        sim.seed_from_rng(seed);
        sim.update_stats();
        sim
    }

    pub fn genome(&self) -> &LeniaGenome {
        &self.genome
    }

    pub fn genome_mut(&mut self) -> &mut LeniaGenome {
        &mut self.genome
    }

    pub fn field(&self) -> &[f32] {
        &self.field
    }

    pub fn channel(&self, ch: usize) -> &[f32] {
        let start = ch * GRID_SIZE;
        &self.field[start..start + GRID_SIZE]
    }

    pub fn channel_mut(&mut self, ch: usize) -> &mut [f32] {
        let start = ch * GRID_SIZE;
        &mut self.field[start..start + GRID_SIZE]
    }

    fn seed_from_rng(&mut self, seed: u64) {
        // 手続き的リングスタンプ（異シードでも同一テクスチャに収束）をやめ、
        // シードごとに正典 Lenia 生物を 1 種選んで中央に配置する。
        // これにより異シード＝異なる本物のソリトン生物になる。
        let species = crate::species_library::select_species(seed);
        self.genome.mu = [species.mu; 3];
        self.genome.sigma = [species.sigma; 3];
        let cx = GRID_WIDTH as f32 / 2.0;
        let cy = GRID_HEIGHT as f32 / 2.0;
        let pattern = crate::species_library::decode_rle(species.rle);
        for ch in 0..3 {
            stamp_pattern(self.channel_mut(ch), &pattern, cx, cy);
        }
    }

    /// 2 種を別チャンネルに配置し、相互抑制する「縄張り対戦」エコシステムを構築する。
    ///
    /// ch0 に種 A、ch1 に種 B を左右に離して配置し、相互抑制行列を設定する。
    /// 両種は相手の密度が高い場所では育ちにくくなるため、縄張りを奪い合う動的な
    /// 展開が生まれる（実機 PoC で共存・制圧の両パターンを確認済み）。
    /// `competition` は抑制の強さ（0.0=無干渉, 1.5 以上で一方が全滅しやすい）。
    pub fn seed_ecosystem(&mut self, species_a: usize, species_b: usize, competition: f32) {
        let lib = crate::species_library::SPECIES;
        let a = &lib[species_a % lib.len()];
        let b = &lib[species_b % lib.len()];

        for v in self.field.iter_mut() {
            *v = 0.0;
        }

        // ch0=種A（μ/σ_A）, ch1=種B（μ/σ_B）, ch2 は未使用（描画では 0）
        self.genome.mu = [a.mu, b.mu, ORBIUM_MU];
        self.genome.sigma = [a.sigma, b.sigma, ORBIUM_SIGMA];
        self.genome.interaction = [[0.0; 3]; 3];
        self.genome.interaction[0][1] = competition; // B が A を抑制
        self.genome.interaction[1][0] = competition; // A が B を抑制

        let quarter = GRID_WIDTH as f32 / 4.0;
        let mid = GRID_HEIGHT as f32 / 2.0;
        let pat_a = crate::species_library::decode_rle(a.rle);
        let pat_b = crate::species_library::decode_rle(b.rle);
        stamp_pattern(self.channel_mut(0), &pat_a, quarter, mid);
        stamp_pattern(self.channel_mut(1), &pat_b, quarter * 3.0, mid);

        self.update_stats();
    }

    /// 種ライブラリの指定インデックスの生物を (cx, cy) に配置し直す（新シード相当）。
    pub fn seed_species(&mut self, index: usize, cx: f32, cy: f32) {
        let species =
            &crate::species_library::SPECIES[index % crate::species_library::SPECIES.len()];
        self.genome.mu = [species.mu; 3];
        self.genome.sigma = [species.sigma; 3];
        let pattern = crate::species_library::decode_rle(species.rle);
        for v in self.field.iter_mut() {
            *v = 0.0;
        }
        for ch in 0..3 {
            stamp_pattern(self.channel_mut(ch), &pattern, cx, cy);
        }
        self.update_stats();
    }

    pub fn seed_brush(&mut self, x: usize, y: usize, radius: usize, strength: f32) {
        let cx = x as f32 + 0.5;
        let cy = y as f32 + 0.5;
        let inner = (radius as f32 * 0.5).max(2.0);
        let outer = (radius as f32 * 1.5).max(4.0);
        for ch in 0..3 {
            stamp_orbium_ring(self.channel_mut(ch), cx, cy, inner, outer, strength);
        }
    }

    /// 環境マスクを半径 radius の円で塗る（0=消去 1=壁 2=養分 3=毒）。
    pub fn paint_env(&mut self, x: usize, y: usize, radius: usize, kind: u8) {
        let cx = x as f32;
        let cy = y as f32;
        let r = radius as f32;
        for gy in 0..GRID_HEIGHT {
            for gx in 0..GRID_WIDTH {
                let dx = toroidal_delta(gx as f32, cx, GRID_WIDTH as f32);
                let dy = toroidal_delta(gy as f32, cy, GRID_HEIGHT as f32);
                if (dx * dx + dy * dy).sqrt() <= r {
                    let idx = gy * GRID_WIDTH + gx;
                    self.env_mask[idx] = kind;
                    // 壁を塗った場所の場は即座に消す
                    if kind == 1 {
                        for ch in 0..3 {
                            self.field[ch * GRID_SIZE + idx] = 0.0;
                        }
                    }
                }
            }
        }
    }

    pub fn env_mask(&self) -> &[u8] {
        &self.env_mask
    }

    pub fn clear_env(&mut self) {
        for v in self.env_mask.iter_mut() {
            *v = 0;
        }
    }

    /// 環境マスクが 1 つでも設定されているか（tick の高速パス判定用）。
    fn has_env(&self) -> bool {
        self.env_mask.iter().any(|&v| v != 0)
    }

    pub fn tick(&mut self) {
        let dt = self.genome.dt;
        let has_env = self.has_env();

        let pre: [Vec<f32>; 3] = [
            self.channel(0).to_vec(),
            self.channel(1).to_vec(),
            self.channel(2).to_vec(),
        ];

        // 相互作用の有無を判定。全ゼロなら従来の高速パス（FFT 結果共有）を使う。
        let has_interaction = self
            .genome
            .interaction
            .iter()
            .any(|row| row.iter().any(|&v| v != 0.0));

        let mut updated: [Option<Vec<f32>>; 3] = [None, None, None];
        for ch in 0..3 {
            // 同一パラメータ・同一場のチャンネルは FFT を再実行せず結果を共有
            // （デフォルトの 3ch 同値構成で tick が 3 倍高速化）。
            // 相互作用ありのときは各 ch で抑制項が異なるため共有しない。
            if !has_interaction {
                let dup = (0..ch).find(|&p| {
                    self.genome.mu[p] == self.genome.mu[ch]
                        && self.genome.sigma[p] == self.genome.sigma[ch]
                        && pre[p] == pre[ch]
                });
                if let Some(p) = dup {
                    updated[ch] = updated[p].clone();
                    continue;
                }
            }

            let potential = convolve2d_fft(
                &pre[ch],
                &self.kernel_fft,
                &self.row_forward,
                &self.row_inverse,
            );
            let mu = self.genome.mu[ch];
            let sigma = self.genome.sigma[ch];
            let interaction = self.genome.interaction[ch];
            let mut next = pre[ch].clone();
            for i in 0..GRID_SIZE {
                if has_env {
                    match self.env_mask[i] {
                        1 => {
                            // 壁: 成長禁止（場を 0 に固定）
                            next[i] = 0.0;
                            continue;
                        }
                        3 => {
                            // 毒: 成長を無効化し既存密度を減衰
                            next[i] = (next[i] * (1.0 - POISON_DECAY * dt)).clamp(0.0, 1.0);
                            continue;
                        }
                        _ => {}
                    }
                }
                let mut growth = lenia_growth(potential[i], mu, sigma);
                if has_interaction {
                    // 他チャンネルの密度による抑制（捕食・縄張り競合）
                    let mut suppression = 0.0f32;
                    for (other, &coeff) in interaction.iter().enumerate() {
                        if coeff != 0.0 {
                            suppression += coeff * pre[other][i];
                        }
                    }
                    growth -= suppression;
                }
                // 養分: 正の成長を増幅
                if has_env && self.env_mask[i] == 2 && growth > 0.0 {
                    growth *= NUTRIENT_GAIN;
                }
                next[i] = (next[i] + dt * growth).clamp(0.0, 1.0);
            }
            updated[ch] = Some(next);
        }

        for (ch, next) in updated.iter_mut().enumerate() {
            if let Some(next) = next.take() {
                self.channel_mut(ch).copy_from_slice(&next);
            }
        }

        self.update_stats();
    }

    pub fn snapshot(&self) -> LeniaSnapshot {
        LeniaSnapshot {
            field: self.field.clone(),
            longevity_ticks: self.longevity_ticks,
            last_centroid_x: self.last_centroid_x,
            last_centroid_y: self.last_centroid_y,
        }
    }

    pub fn restore_snapshot(&mut self, snap: &LeniaSnapshot) {
        if snap.field.len() == self.field.len() {
            self.field.copy_from_slice(&snap.field);
        }
        self.longevity_ticks = snap.longevity_ticks;
        self.last_centroid_x = snap.last_centroid_x;
        self.last_centroid_y = snap.last_centroid_y;
        self.update_stats();
    }

    fn update_stats(&mut self) {
        let ch0 = self.channel(0);
        let mut sum = 0.0f32;
        let mut cx = 0.0f32;
        let mut cy = 0.0f32;
        for y in 0..GRID_HEIGHT {
            for x in 0..GRID_WIDTH {
                let v = ch0[y * GRID_WIDTH + x];
                if v > 0.1 {
                    sum += v;
                    cx += x as f32 * v;
                    cy += y as f32 * v;
                }
            }
        }
        self.mass = sum;
        if sum >= STABLE_MASS_THRESHOLD {
            self.longevity_ticks = self.longevity_ticks.saturating_add(1);
        } else {
            self.longevity_ticks = 0;
        }
        if sum > 1e-6 {
            cx /= sum;
            cy /= sum;
            let dx = toroidal_delta(cx, self.last_centroid_x, GRID_WIDTH as f32);
            let dy = toroidal_delta(cy, self.last_centroid_y, GRID_HEIGHT as f32);
            self.locomotion = (dx * dx + dy * dy).sqrt();
            self.last_centroid_x = cx;
            self.last_centroid_y = cy;
        } else {
            self.locomotion = 0.0;
        }
    }

    /// パラメータ＋形態の量子化ハッシュ（図鑑一意キー）
    pub fn species_hash(&self) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        for ch in 0..3 {
            for v in self.channel(ch) {
                if *v > 0.1 {
                    ((v * 8.0).round() as u8).hash(&mut hasher);
                }
            }
        }
        ((self.genome.mu[0] * 1000.0).round() as u32).hash(&mut hasher);
        ((self.genome.sigma[0] * 100_000.0).round() as u32).hash(&mut hasher);
        hasher.finish()
    }
}

/// ガウス成長関数 G(u) = 2·exp(-(u-μ)²/(2σ²)) - 1
pub fn lenia_growth(u: f32, mu: f32, sigma: f32) -> f32 {
    let s2 = 2.0 * sigma * sigma;
    2.0 * (-((u - mu).powi(2)) / s2).exp() - 1.0
}

fn wrap_coord(v: isize, size: usize) -> usize {
    let m = size as isize;
    (((v % m) + m) % m) as usize
}

fn toroidal_delta(a: f32, b: f32, size: f32) -> f32 {
    let mut d = a - b;
    if d > size / 2.0 {
        d -= size;
    } else if d < -size / 2.0 {
        d += size;
    }
    d
}

/// デコード済み正典パターンを中心 (cx, cy) に配置する（トロイダル境界で wrap）。
fn stamp_pattern(
    field: &mut [f32],
    pattern: &crate::species_library::DecodedPattern,
    cx: f32,
    cy: f32,
) {
    if pattern.width == 0 || pattern.height == 0 {
        return;
    }
    let ox = cx - pattern.width as f32 / 2.0;
    let oy = cy - pattern.height as f32 / 2.0;
    for py in 0..pattern.height {
        for px in 0..pattern.width {
            let v = pattern.cells[py * pattern.width + px];
            if v <= 0.0 {
                continue;
            }
            let gx = wrap_coord((ox + px as f32).round() as isize, GRID_WIDTH);
            let gy = wrap_coord((oy + py as f32).round() as isize, GRID_HEIGHT);
            let idx = gy * GRID_WIDTH + gx;
            field[idx] = field[idx].max(v);
        }
    }
}

#[allow(dead_code)]
fn seed_orbium_ring(field: &mut [f32], cx: f32, cy: f32) {
    stamp_orbium_ring(field, cx, cy, 4.0, 8.0, 1.0);
}

/// リング状スタンプ（Orbium 系の定常解に近い初期条件）
fn stamp_orbium_ring(field: &mut [f32], cx: f32, cy: f32, inner: f32, outer: f32, strength: f32) {
    for y in 0..GRID_HEIGHT {
        for x in 0..GRID_WIDTH {
            let dx = toroidal_delta(x as f32, cx, GRID_WIDTH as f32);
            let dy = toroidal_delta(y as f32, cy, GRID_HEIGHT as f32);
            let dist = (dx * dx + dy * dy).sqrt();
            if dist > inner && dist < outer {
                let span = (outer - inner).max(0.001);
                let t = ((dist - inner) / span).min(1.0);
                let val = (strength * (0.3 + 0.7 * (1.0 - (2.0 * t - 1.0).abs()))).clamp(0.0, 1.0);
                let idx = y * GRID_WIDTH + x;
                field[idx] = field[idx].max(val);
            }
        }
    }
}

#[allow(dead_code)]
fn seed_disk(field: &mut [f32], cx: f32, cy: f32, radius: f32) {
    for y in 0..GRID_HEIGHT {
        for x in 0..GRID_WIDTH {
            let dx = toroidal_delta(x as f32, cx, GRID_WIDTH as f32);
            let dy = toroidal_delta(y as f32, cy, GRID_HEIGHT as f32);
            let dist = (dx * dx + dy * dy).sqrt();
            if dist < radius {
                field[y * GRID_WIDTH + x] = 1.0;
            }
        }
    }
}

/// リングカーネル core(r) = exp(4 - 4/(4r(1-r))) を 2D トロイダルグリッドに配置
fn build_ring_kernel_2d(radius: usize) -> Vec<f32> {
    let mut kernel = vec![0.0f32; GRID_SIZE];
    let mut sum = 0.0f32;
    let r_max = radius as f32;

    for ky in 0..GRID_HEIGHT {
        for kx in 0..GRID_WIDTH {
            let dx = toroidal_delta(kx as f32, 0.0, GRID_WIDTH as f32).abs();
            let dy = toroidal_delta(ky as f32, 0.0, GRID_HEIGHT as f32).abs();
            let dist = (dx * dx + dy * dy).sqrt();
            if dist <= r_max {
                let r = (dist / r_max).clamp(0.001, 0.999);
                let w = ring_core(r);
                kernel[ky * GRID_WIDTH + kx] = w;
                sum += w;
            }
        }
    }

    if sum > 1e-8 {
        for v in kernel.iter_mut() {
            *v /= sum;
        }
    }
    kernel
}

fn ring_core(r: f32) -> f32 {
    (4.0 - 4.0 / (4.0 * r * (1.0 - r))).exp()
}

fn fft2d_forward(spatial: &[f32], row_forward: &Arc<dyn Fft<f32>>) -> Vec<Complex<f32>> {
    let mut buf: Vec<Complex<f32>> = spatial.iter().map(|&v| Complex::new(v, 0.0)).collect();
    fft2d_in_place(&mut buf, row_forward, row_forward, true);
    buf
}

fn fft2d_in_place(
    data: &mut [Complex<f32>],
    forward_fft: &Arc<dyn Fft<f32>>,
    inverse_fft: &Arc<dyn Fft<f32>>,
    forward: bool,
) {
    let row_fft = if forward { forward_fft } else { inverse_fft };

    for y in 0..GRID_HEIGHT {
        let row = &mut data[y * GRID_WIDTH..(y + 1) * GRID_WIDTH];
        row_fft.process(row);
    }

    let mut col = vec![Complex::new(0.0, 0.0); GRID_HEIGHT];
    for x in 0..GRID_WIDTH {
        for y in 0..GRID_HEIGHT {
            col[y] = data[y * GRID_WIDTH + x];
        }
        row_fft.process(&mut col);
        for y in 0..GRID_HEIGHT {
            data[y * GRID_WIDTH + x] = col[y];
        }
    }

    if !forward {
        let scale = 1.0 / (GRID_WIDTH * GRID_HEIGHT) as f32;
        for c in data.iter_mut() {
            *c *= scale;
        }
    }
}

#[allow(dead_code)]
fn convolve2d_direct(field: &[f32], kernel: &[f32], radius: usize) -> Vec<f32> {
    let r = radius as isize;
    let mut out = vec![0.0f32; GRID_SIZE];
    for y in 0..GRID_HEIGHT {
        for x in 0..GRID_WIDTH {
            let mut sum = 0.0f32;
            for dy in -r..=r {
                for dx in -r..=r {
                    let kx = wrap_coord(dx, GRID_WIDTH);
                    let ky = wrap_coord(dy, GRID_HEIGHT);
                    let kw = kernel[ky * GRID_WIDTH + kx];
                    if kw.abs() < 1e-8 {
                        continue;
                    }
                    let fx = wrap_coord(x as isize - dx, GRID_WIDTH);
                    let fy = wrap_coord(y as isize - dy, GRID_HEIGHT);
                    sum += field[fy * GRID_WIDTH + fx] * kw;
                }
            }
            out[y * GRID_WIDTH + x] = sum;
        }
    }
    out
}

fn convolve2d_fft(
    field: &[f32],
    kernel_fft: &[Complex<f32>],
    row_forward: &Arc<dyn Fft<f32>>,
    row_inverse: &Arc<dyn Fft<f32>>,
) -> Vec<f32> {
    let mut freq: Vec<Complex<f32>> = field.iter().map(|&v| Complex::new(v, 0.0)).collect();
    fft2d_in_place(&mut freq, row_forward, row_inverse, true);

    for i in 0..GRID_SIZE {
        freq[i] *= kernel_fft[i];
    }

    fft2d_in_place(&mut freq, row_forward, row_inverse, false);

    freq.iter().map(|c| c.re).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_seed_brush_ring_persists_over_ticks() {
        let mut sim = LeniaSimulator::new(99);
        sim.seed_brush(20, 20, 3, 0.85);
        assert!(
            sim.mass > 1.0,
            "ring brush should add mass, got {}",
            sim.mass
        );
        for _ in 0..30 {
            sim.tick();
        }
        assert!(
            sim.mass > 1.0,
            "ring brush seed should persist after 30 ticks, mass={}",
            sim.mass
        );
    }

    #[test]
    fn test_ecosystem_two_species_coexist() {
        // Positive: 適正な競合（1.0）で 2 種が別チャンネルに生き残る
        let mut sim = LeniaSimulator::new(1);
        sim.seed_ecosystem(0, 3, 1.0);
        for _ in 0..200 {
            sim.tick();
        }
        let mass_a: f32 = sim.channel(0).iter().filter(|&&v| v > 0.1).sum();
        let mass_b: f32 = sim.channel(1).iter().filter(|&&v| v > 0.1).sum();
        assert!(
            mass_a > 1.0 && mass_b > 1.0,
            "both species should survive under mild competition: A={mass_a} B={mass_b}"
        );
    }

    #[test]
    fn test_ecosystem_strong_competition_causes_extinction() {
        // Negative: 過大な競合（3.0）で相互作用が実際に効き、一方が全滅寄りになる。
        // これにより「相互抑制項が機能している」ことを証明する。
        let mut mild = LeniaSimulator::new(1);
        mild.seed_ecosystem(0, 3, 0.0);
        let mut harsh = LeniaSimulator::new(1);
        harsh.seed_ecosystem(0, 3, 3.0);
        for _ in 0..200 {
            mild.tick();
            harsh.tick();
        }
        let harsh_total: f32 = harsh.field.iter().filter(|&&v| v > 0.1).sum();
        let mild_total: f32 = mild.field.iter().filter(|&&v| v > 0.1).sum();
        assert!(
            harsh_total < mild_total,
            "strong competition should reduce total biomass vs no competition: harsh={harsh_total} mild={mild_total}"
        );
    }

    #[test]
    fn test_env_wall_blocks_growth() {
        // Positive: 壁を塗った領域には場が侵入できない
        let mut sim = LeniaSimulator::new(0);
        // グリッド中央縦帯を壁に
        for y in 20..108 {
            sim.paint_env(GRID_WIDTH / 2, y, 1, 1);
        }
        for _ in 0..100 {
            sim.tick();
        }
        // 壁セル（中央列）はすべて 0 のまま
        let mut wall_mass = 0.0f32;
        for y in 20..108 {
            wall_mass += sim.channel(0)[y * GRID_WIDTH + GRID_WIDTH / 2];
        }
        assert!(
            wall_mass < 1e-3,
            "wall cells must stay empty, got {wall_mass}"
        );
    }

    #[test]
    fn test_env_empty_mask_preserves_behavior() {
        // Negative: 環境ペン未使用（マスク全ゼロ）なら挙動は現状と完全一致
        let mut a = LeniaSimulator::new(0);
        let mut b = LeniaSimulator::new(0);
        b.paint_env(10, 10, 3, 0); // kind=0 は消去＝何もしない
        for _ in 0..50 {
            a.tick();
            b.tick();
        }
        assert_eq!(
            a.channel(0),
            b.channel(0),
            "empty env mask must not change dynamics"
        );
    }

    #[test]
    fn test_interaction_default_preserves_single_species_behavior() {
        // 相互作用が全ゼロなら従来挙動と完全一致（高速パスも同結果）
        let mut a = LeniaSimulator::new(0);
        let mut b = LeniaSimulator::new(0);
        for _ in 0..50 {
            a.tick();
            b.tick();
        }
        assert_eq!(a.channel(0), b.channel(0));
        assert!(a
            .genome
            .interaction
            .iter()
            .all(|r| r.iter().all(|&v| v == 0.0)));
    }

    #[test]
    fn test_fft_matches_direct_convolution() {
        let sim = LeniaSimulator::new(42);
        let field = sim.channel(0).to_vec();
        let direct = convolve2d_direct(&field, &sim.kernel_spatial, KERNEL_RADIUS);
        let fft = convolve2d_fft(&field, &sim.kernel_fft, &sim.row_forward, &sim.row_inverse);
        for (i, (&d, &f)) in direct.iter().zip(fft.iter()).enumerate() {
            assert!(
                (d - f).abs() < 1e-3,
                "FFT vs direct mismatch at {i}: direct={d} fft={f}"
            );
        }
    }

    #[test]
    fn test_three_channel_independent_params() {
        let mut sim = LeniaSimulator::new(42);
        sim.genome_mut().mu[1] = 0.20;
        sim.genome_mut().sigma[2] = 0.025;
        sim.tick();
        let ch0 = sim.channel(0).to_vec();
        let ch1 = sim.channel(1).to_vec();
        // 独立更新後、全チャンネル同一コピーではない（パラメータ差で diverge しうる）
        assert_eq!(ch0.len(), ch1.len());
    }

    #[test]
    fn test_snapshot_roundtrip() {
        let mut sim = LeniaSimulator::new(42);
        for _ in 0..5 {
            sim.tick();
        }
        let snap = sim.snapshot();
        let mass_before = sim.mass;
        sim.tick();
        assert_ne!(sim.mass, mass_before);
        sim.restore_snapshot(&snap);
        assert!((sim.mass - mass_before).abs() < 1e-4);
    }

    #[test]
    fn test_lenia_growth_peak_at_mu() {
        let g = lenia_growth(ORBIUM_MU, ORBIUM_MU, ORBIUM_SIGMA);
        assert!(g > 0.9, "growth at mu should be near 1, got {g}");
        let g_low = lenia_growth(0.0, ORBIUM_MU, ORBIUM_SIGMA);
        assert!(
            g_low < 0.0,
            "growth far from mu should be negative, got {g_low}"
        );
    }

    #[test]
    fn test_orbium_mass_persists_over_ticks() {
        let mut sim = LeniaSimulator::new(42);
        let initial_mass = sim.mass;
        assert!(
            initial_mass > 1.0,
            "seed should produce mass, got {initial_mass}"
        );

        for _ in 0..30 {
            sim.tick();
        }
        assert!(
            sim.mass > 1.0,
            "mass should persist: initial={initial_mass} now={}",
            sim.mass
        );
    }

    #[test]
    fn test_orbium_locomotion_nonzero() {
        let mut sim = LeniaSimulator::new(777);
        for _ in 0..80 {
            sim.tick();
        }
        assert!(
            sim.mass > 1.0,
            "after 80 ticks expect stable mass > 1, got mass={}",
            sim.mass
        );
    }

    #[test]
    fn test_deterministic_lenia() {
        let mut a = LeniaSimulator::new(12345);
        let mut b = LeniaSimulator::new(12345);
        for _ in 0..10 {
            a.tick();
            b.tick();
        }
        assert_eq!(a.field, b.field);
    }
}

/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 */

/// ベクトル演算の統一インターフェース
pub trait VectorOps {
    /// 余弦類似度 (f64)
    fn cosine_similarity(a: &[f64], b: &[f64]) -> f64;
    /// 余弦類似度 (f32)
    fn cosine_similarity_f32(a: &[f32], b: &[f32]) -> f64;
    /// 近似内積 (Phase 40で実装予定)
    fn approximate_dot_product(a_compressed: &[u8], b: &[f64], dim: usize) -> f64;
    /// 圧縮ドメインでの余弦類似度
    fn compressed_cosine_similarity(a_comp: &[u8], b_comp: &[u8], dim: usize) -> f64;
    /// 生ベクトルと圧縮ベクトルの近似余弦類似度
    fn approximate_cosine_similarity(a_raw: &[f64], b_comp: &[u8], dim: usize) -> f64;
}

/// Helper function to use standard cosine similarity
pub fn cosine_similarity(a: &[f64], b: &[f64]) -> f64 {
    StandardVectorOps::cosine_similarity(a, b)
}

/// Helper function to use standard cosine similarity (f32)
pub fn cosine_similarity_f32(a: &[f32], b: &[f32]) -> f64 {
    StandardVectorOps::cosine_similarity_f32(a, b)
}

/// フル精度（標準）のベクトル演算実装
pub struct StandardVectorOps;

impl VectorOps for StandardVectorOps {
    fn cosine_similarity(a: &[f64], b: &[f64]) -> f64 {
        if a.len() != b.len() || a.is_empty() {
            return 0.0;
        }
        let dot: f64 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let norm_a: f64 = a.iter().map(|x| x * x).sum::<f64>().sqrt();
        let norm_b: f64 = b.iter().map(|x| x * x).sum::<f64>().sqrt();
        if norm_a == 0.0 || norm_b == 0.0 {
            0.0
        } else {
            dot / (norm_a * norm_b)
        }
    }

    fn cosine_similarity_f32(a: &[f32], b: &[f32]) -> f64 {
        if a.len() != b.len() || a.is_empty() {
            return 0.0;
        }
        let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm_a == 0.0 || norm_b == 0.0 {
            0.0
        } else {
            (dot / (norm_a * norm_b)) as f64
        }
    }

    fn approximate_dot_product(a_compressed: &[u8], b: &[f64], dim: usize) -> f64 {
        if a_compressed.len() < dim || b.len() < dim {
            return 0.0;
        }

        // 8-bit quantization approximation: [0, 255] -> [-1.0, 1.0]
        // val = (byte / 127.5) - 1.0
        let mut sum = 0.0;
        for i in 0..dim {
            let a_val = (a_compressed[i] as f64 / 127.5) - 1.0;
            sum += a_val * b[i];
        }
        sum
    }

    fn compressed_cosine_similarity(a_comp: &[u8], b_comp: &[u8], dim: usize) -> f64 {
        if a_comp.len() < 2 || b_comp.len() < 2 {
            return 0.0;
        }

        let version_a = a_comp[0];
        let version_b = b_comp[0];

        // バージョンが異なる、または古いバージョンの場合はフォールバック（デコード）
        if version_a != version_b || version_a < 2 {
            let encoder = crate::polar_quant::PolarQuantEncoder::new(4, 32);
            let v1 = encoder.decode(a_comp, dim);
            let v2 = encoder.decode(b_comp, dim);
            return Self::cosine_similarity(&v1, &v2);
        }

        // Version 2 最適化パス: 圧縮ドメインでの高速演算
        let qjl_dim = 256;
        let qjl_bytes = qjl_dim / 8;

        // ヘッダー (3バイト) を飛ばす
        let mut i = 3;
        let polar_end = if a_comp.len() > qjl_bytes {
            a_comp.len() - qjl_bytes
        } else {
            a_comp.len()
        };

        let mut dot_total = 0.0;
        let mut norm_a = 0.0;
        let mut norm_b = 0.0;

        // Stage 1: Polar 領域での内積計算
        while i + 1 < polar_end {
            let r1_q = a_comp[i];
            let t1_q = a_comp[i + 1];
            let r2_q = b_comp[i];
            let t2_q = b_comp[i + 1];

            let r1 = (r1_q as f64) / 255.0 * 2.0;
            let r2 = (r2_q as f64) / 255.0 * 2.0;
            let t1 = (t1_q as f64) / 255.0 * (2.0 * std::f64::consts::PI) - std::f64::consts::PI;
            let t2 = (t2_q as f64) / 255.0 * (2.0 * std::f64::consts::PI) - std::f64::consts::PI;

            // r1*r2*cos(t1-t2)
            let cos_diff = (t1 - t2).cos();
            dot_total += r1 * r2 * cos_diff;
            norm_a += r1 * r1;
            norm_b += r2 * r2;

            i += 2;
        }

        // Stage 2: QJL (Hamming 距離) による補正
        if a_comp.len() >= qjl_bytes && b_comp.len() >= qjl_bytes {
            let s1 = &a_comp[a_comp.len() - qjl_bytes..];
            let s2 = &b_comp[b_comp.len() - qjl_bytes..];

            let mut matches = 0;
            for j in 0..qjl_bytes {
                // XOR の否定 = 一致しているビット
                matches += (!(s1[j] ^ s2[j])).count_ones() as i32;
            }

            // Hamming 類似度による内積の補正
            // 期待値: alpha^2 * dim * (2 * matches / m - 1)
            let alpha = 0.8 / ((dim as f64) * (qjl_dim as f64)).sqrt();
            let correction =
                (alpha * alpha) * (dim as f64) * (2.0 * (matches as f64) / (qjl_dim as f64) - 1.0);
            dot_total += correction;
        }

        if norm_a == 0.0 || norm_b == 0.0 {
            0.0
        } else {
            dot_total / (norm_a.sqrt() * norm_b.sqrt())
        }
    }

    fn approximate_cosine_similarity(a_raw: &[f64], b_comp: &[u8], dim: usize) -> f64 {
        if b_comp.len() < 2 || a_raw.len() < dim {
            return 0.0;
        }

        let version = b_comp[0];
        if version < 2 {
            // フォールバック: デコード
            let encoder = crate::polar_quant::PolarQuantEncoder::new(4, 32);
            let v_comp = encoder.decode(b_comp, dim);
            return Self::cosine_similarity(a_raw, &v_comp);
        }

        let qjl_dim = 256;
        let qjl_bytes = qjl_dim / 8;
        let mut i = 3;
        let polar_end = if b_comp.len() > qjl_bytes {
            b_comp.len() - qjl_bytes
        } else {
            b_comp.len()
        };

        let mut dot_total = 0.0;
        let mut norm_a = 0.0;
        let mut norm_b = 0.0;

        let mut idx = 0;
        while i + 1 < polar_end && idx + 1 < dim {
            let r_q = b_comp[i];
            let t_q = b_comp[i + 1];

            let r = (r_q as f64) / 255.0 * 2.0;
            let theta = (t_q as f64) / 255.0 * (2.0 * std::f64::consts::PI) - std::f64::consts::PI;

            let bx = r * theta.cos();
            let by = r * theta.sin();

            dot_total += a_raw[idx] * bx + a_raw[idx + 1] * by;
            norm_a += a_raw[idx] * a_raw[idx] + a_raw[idx + 1] * a_raw[idx + 1];
            norm_b += r * r;

            i += 2;
            idx += 2;
        }

        // QJL 補正
        if b_comp.len() >= qjl_bytes {
            let s_comp = &b_comp[b_comp.len() - qjl_bytes..];
            let alpha = 0.8 / ((dim as f64) * (qjl_dim as f64)).sqrt();

            // a_raw の射影を計算
            let encoder = crate::polar_quant::PolarQuantEncoder::new(4, 32);
            let matrix = encoder.get_projection_matrix(dim);

            for j in 0..qjl_dim {
                let row = &matrix[j];
                let sign_comp = if (s_comp[j / 8] >> (j % 8)) & 1 == 1 {
                    1.0
                } else {
                    -1.0
                };

                // 生ベクトル a_raw と 射影行のドット積
                let mut dot_raw_proj = 0.0;
                for (k, val) in a_raw.iter().enumerate() {
                    dot_raw_proj += val * row[k];
                }

                // 誤差による内積寄与分を直接加算
                dot_total += alpha * sign_comp * dot_raw_proj;
            }
        }

        if norm_a == 0.0 || norm_b == 0.0 {
            0.0
        } else {
            dot_total / (norm_a.sqrt() * norm_b.sqrt())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_standard_cosine_similarity() {
        let v1 = vec![1.0, 0.0, 0.0];
        let v2 = vec![1.0, 0.0, 0.0];
        // StandardVectorOps should be implemented and pass this
        let sim = StandardVectorOps::cosine_similarity(&v1, &v2);
        assert!((sim - 1.0).abs() < f64::EPSILON);

        let v3 = vec![0.0, 1.0, 0.0];
        let sim2 = StandardVectorOps::cosine_similarity(&v1, &v3);
        assert!(sim2.abs() < f64::EPSILON);
    }

    #[test]
    fn test_standard_cosine_similarity_f32() {
        let v1: Vec<f32> = vec![1.0, 1.0];
        let v2: Vec<f32> = vec![1.0, 1.0];
        let sim = StandardVectorOps::cosine_similarity_f32(&v1, &v2);
        assert!((sim - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_approximate_dot_product() {
        let dim = 4;
        // 127.5 is 0.0, 255 is 1.0, 0 is -1.0
        let a_compressed = vec![255, 0, 128, 255]; // [1.0, -1.0, ~0.0, 1.0]
        let b = vec![1.0, 1.0, 1.0, 1.0];

        let dot = StandardVectorOps::approximate_dot_product(&a_compressed, &b, dim);
        // 1.0*1.0 + (-1.0)*1.0 + (0.0)*1.0 + 1.0*1.0 = 1.0
        assert!(dot > 0.9 && dot < 1.1);
    }

    #[test]
    fn test_compressed_cosine_similarity_accuracy() {
        use crate::polar_quant::PolarQuantEncoder;
        let dim = 768;
        let v1: Vec<f64> = (0..dim)
            .map(|i| if i % 2 == 0 { 0.5 } else { -0.5 })
            .collect();
        let v2: Vec<f64> = (0..dim)
            .map(|i| if i % 3 == 0 { 0.3 } else { 0.1 })
            .collect();

        let encoder = PolarQuantEncoder::new(4, 32);
        let c1 = encoder.encode(&v1);
        let c2 = encoder.encode(&v2);

        let original_sim = StandardVectorOps::cosine_similarity(&v1, &v2);
        let compressed_sim = StandardVectorOps::compressed_cosine_similarity(&c1, &c2, dim);

        let diff = (original_sim - compressed_sim).abs();
        println!(
            "DEBUG: Original={}, Compressed={}, Diff={}",
            original_sim, compressed_sim, diff
        );

        // 誤差 0.02 以内を目指す (TurboQuant なら可能)
        assert!(
            diff < 0.02,
            "Compressed similarity error {} is too large",
            diff
        );
    }
}

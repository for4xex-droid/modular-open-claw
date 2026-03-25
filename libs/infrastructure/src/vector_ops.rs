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
}

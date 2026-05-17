/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 * Licensed under the Business Source License 1.1.
 */

use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::Arc;

/// 射影行列のキャッシュ。次元ごとに 256 行の射影ベクトルを保持。
static PROJECTION_CACHE: Lazy<parking_lot::Mutex<HashMap<usize, Arc<Vec<Vec<f64>>>>>> =
    Lazy::new(|| parking_lot::Mutex::new(HashMap::new()));

/// TurboQuant PolarQuant エンコーダ
pub struct PolarQuantEncoder {
    bits: u8,
    min_dim: usize,
}

const QJL_SEED: u64 = 0x7777_AAAA_BBBB_CCCC;
const QJL_PROJECTION_DIM: usize = 256;

impl PolarQuantEncoder {
    /// 新規インスタンス生成
    pub fn new(bits: u8, min_dim: usize) -> Self {
        Self { bits, min_dim }
    }

    /// 射影行列を取得（キャッシュ利用）
    pub(crate) fn get_projection_matrix(&self, dim: usize) -> Arc<Vec<Vec<f64>>> {
        let mut cache = PROJECTION_CACHE.lock();
        if let Some(matrix) = cache.get(&dim) {
            return matrix.clone();
        }

        let matrix: Arc<Vec<Vec<f64>>> = Arc::new(
            (0..QJL_PROJECTION_DIM)
                .map(|j| {
                    let mut rng = QJL_SEED ^ (j as u64).wrapping_mul(0x9E3779B9);
                    (0..dim)
                        .map(|_| {
                            rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1);
                            if (rng >> 32) & 1 == 0 {
                                -1.0
                            } else {
                                1.0
                            }
                        })
                        .collect()
                })
                .collect(),
        );

        cache.insert(dim, matrix.clone());
        matrix
    }

    /// ベクトルのエンコード
    pub fn encode(&self, vector: &[f64]) -> Vec<u8> {
        let dim = vector.len();
        let mut result = Vec::new();

        // 1. バージョンタグ (Version 2: Polar + QJL)
        result.push(2);

        if dim < self.min_dim {
            result.push(1); // Flags: Raw
            for &val in vector {
                result.extend_from_slice(&val.to_le_bytes());
            }
        } else {
            result.push(0); // Flags: Compressed
            result.push(self.bits);

            // Stage 1: PolarQuant
            let mut polar_reconstructed = Vec::with_capacity(dim);
            for chunk in vector.chunks(2) {
                if chunk.len() == 2 {
                    let r = (chunk[0].powi(2) + chunk[1].powi(2)).sqrt();
                    let theta = chunk[1].atan2(chunk[0]);

                    // 8-bit each for r and theta -> 2 bytes per 2 dims = 1 byte/dim
                    let r_q = ((r / 2.0).clamp(0.0, 1.0) * 255.0).round() as u8;
                    let theta_norm = (theta + std::f64::consts::PI) / (2.0 * std::f64::consts::PI);
                    let theta_q = (theta_norm.clamp(0.0, 1.0) * 255.0).round() as u8;

                    result.push(r_q);
                    result.push(theta_q);

                    let r_rec = ((r_q as f64) / 255.0) * 2.0;
                    let theta_rec = (theta_q as f64 / 255.0) * (2.0 * std::f64::consts::PI)
                        - std::f64::consts::PI;
                    polar_reconstructed.push(r_rec * theta_rec.cos());
                    polar_reconstructed.push(r_rec * theta_rec.sin());
                } else {
                    result.extend_from_slice(&chunk[0].to_le_bytes());
                    polar_reconstructed.push(chunk[0]);
                }
            }

            // Stage 2: QJL (残留誤差補正)
            let matrix = self.get_projection_matrix(dim);
            let mut qjl_bits = vec![0u8; QJL_PROJECTION_DIM / 8];
            for j in 0..QJL_PROJECTION_DIM {
                let row = &matrix[j];
                let mut dot_err = 0.0;
                for (i, v_orig) in vector.iter().enumerate() {
                    dot_err += (v_orig - polar_reconstructed[i]) * row[i];
                }
                if dot_err > 0.0 {
                    qjl_bits[j / 8] |= 1 << (j % 8);
                }
            }
            result.extend_from_slice(&qjl_bits);
        }

        result
    }

    /// ベクトルのデコード
    pub fn decode(&self, compressed: &[u8], dim: usize) -> Vec<f64> {
        if compressed.len() < 2 {
            return vec![0.0; dim];
        }

        let version = compressed[0];
        let flags = compressed[1];
        let mut result = Vec::with_capacity(dim);

        if flags == 1 {
            let mut i = 2;
            while i + 8 <= compressed.len() && result.len() < dim {
                let bytes: [u8; 8] = compressed[i..i + 8].try_into().unwrap_or([0; 8]);
                result.push(f64::from_le_bytes(bytes));
                i += 8;
            }
        } else {
            // QJL ビットの位置を特定
            let mut i = 3;
            let qjl_num_bytes = if version >= 2 {
                QJL_PROJECTION_DIM / 8
            } else {
                0
            };
            let qjl_start = if compressed.len() >= qjl_num_bytes {
                compressed.len() - qjl_num_bytes
            } else {
                compressed.len()
            };

            while result.len() < dim && i < qjl_start {
                if version == 1 {
                    // Version 1: 4-bit r + 4-bit theta (1 byte per 2 dims)
                    if i < qjl_start {
                        let byte = compressed[i];
                        let r_q = (byte >> 4) & 0x0F;
                        let theta_q = byte & 0x0F;
                        let r = ((r_q as f64) / 15.0) * 2.0;
                        let theta = (theta_q as f64 / 15.0) * (2.0 * std::f64::consts::PI)
                            - std::f64::consts::PI;
                        result.push(r * theta.cos());
                        result.push(r * theta.sin());
                        i += 1;
                    }
                } else {
                    // Version 2: 8-bit r + 8-bit theta (2 bytes per 2 dims)
                    if i + 1 < qjl_start {
                        let r_q = compressed[i];
                        let theta_q = compressed[i + 1];
                        let r = ((r_q as f64) / 255.0) * 2.0;
                        let theta = (theta_q as f64 / 255.0) * (2.0 * std::f64::consts::PI)
                            - std::f64::consts::PI;
                        result.push(r * theta.cos());
                        result.push(r * theta.sin());
                        i += 2;
                    } else if i + 8 <= qjl_start {
                        let bytes: [u8; 8] = compressed[i..i + 8].try_into().unwrap_or([0; 8]);
                        result.push(f64::from_le_bytes(bytes));
                        i += 8;
                    } else {
                        break;
                    }
                }
            }

            // Stage 2 デコード (QJL 補正)
            if version >= 2
                && qjl_num_bytes > 0
                && compressed.len() >= qjl_num_bytes
                && !result.is_empty()
            {
                let qjl_bits_data = &compressed[compressed.len() - qjl_num_bytes..];
                let matrix = self.get_projection_matrix(dim);

                // 補正量 α
                let alpha = 0.8 / ((dim as f64) * (QJL_PROJECTION_DIM as f64)).sqrt();

                for j in 0..QJL_PROJECTION_DIM {
                    let row = &matrix[j];
                    let sign_actual = if (qjl_bits_data[j / 8] >> (j % 8)) & 1 == 1 {
                        1.0
                    } else {
                        -1.0
                    };

                    for (v, r) in result.iter_mut().zip(row.iter()) {
                        *v += r * alpha * sign_actual;
                    }
                }
            }
        }

        while result.len() < dim {
            result.push(0.0);
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_v1_v2_compatibility_manual() {
        // v1 encoder: version=1, flags=0 (compressed), bits=4
        // For a 2-dim vector [0.5, 0.5]: r=0.707, theta=0.785
        // r_q = (0.707/2.0 * 15) = 5.3 -> 5
        // theta_q = ((0.785+PI)/(2PI) * 15) = (3.92/6.28 * 15) = 9.3 -> 9
        // Result byte: (5 << 4) | 9 = 0x59
        let v1_data = vec![1, 0, 4, 0x59]; // Version 1, Compressed, 4bits, Data

        let encoder = PolarQuantEncoder::new(4, 32);
        let decoded = encoder.decode(&v1_data, 2);
        assert_eq!(decoded.len(), 2);
        // Just verify it decodes SOMETHING without crashing in V1 mode
        assert!(decoded[0] > 0.0);
    }

    #[test]
    fn test_qjl_accuracy_improvement_768d() {
        let encoder = PolarQuantEncoder::new(4, 32);
        // Using a fixed seed for reproducible test vectors
        let mut rng_val: u64 = 42;
        let vector: Vec<f64> = (0..768)
            .map(|_| {
                rng_val = rng_val.wrapping_mul(6364136223846793005).wrapping_add(1);
                (rng_val as f64 / u64::MAX as f64) * 2.0 - 1.0
            })
            .collect();

        let encoded = encoder.encode(&vector);
        let decoded = encoder.decode(&encoded, 768);

        let mut mse = 0.0;
        for (a, b) in vector.iter().zip(decoded.iter()) {
            mse += (a - b).powi(2);
        }
        mse /= 768.0;

        // Current PolarQuant-only MSE is ~0.008.
        // QJL should bring this down significantly (< 0.001)
        println!("DEBUG: MSE = {}", mse);
        assert!(
            mse < 0.001,
            "MSE {} is too high. QJL might be missing or ineffective.",
            mse
        );
    }
}

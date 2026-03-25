/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 */

/// TurboQuant PolarQuant エンコーダ
pub struct PolarQuantEncoder {
    bits: u8,
    min_dim: usize,
}

impl PolarQuantEncoder {
    /// 新規インスタンス生成
    pub fn new(bits: u8, min_dim: usize) -> Self {
        Self { bits, min_dim }
    }

    /// ベクトルのエンコード
    pub fn encode(&self, vector: &[f64]) -> Vec<u8> {
        let dim = vector.len();
        let mut result = Vec::new();

        // 1. バージョンタグとフラグ
        // [0]: Version (1)
        // [1]: Flags (0: Compressed, 1: Raw)
        result.push(1);

        if dim < self.min_dim {
            // 低次元フォールバック (Raw)
            result.push(1); // Flags: Raw
            for &val in vector {
                result.extend_from_slice(&val.to_le_bytes());
            }
        } else {
            // PolarQuant 圧縮
            result.push(0); // Flags: Compressed
            result.push(self.bits); // Bits used

            // ペアで処理するため、奇数の場合は最後をそのまま残すかパディング
            for chunk in vector.chunks(2) {
                if chunk.len() == 2 {
                    let r = (chunk[0].powi(2) + chunk[1].powi(2)).sqrt();
                    let theta = chunk[1].atan2(chunk[0]); // -PI to PI

                    // r: 0..2.0 range (covers unit vectors and some overhead)
                    let r_q = ((r / 2.0).clamp(0.0, 1.0) * 15.0).round() as u8;
                    // theta: -PI..PI -> 0..15
                    let theta_norm = (theta + std::f64::consts::PI) / (2.0 * std::f64::consts::PI);
                    let theta_q = (theta_norm.clamp(0.0, 1.0) * 15.0).round() as u8;

                    result.push((r_q << 4) | theta_q);
                } else {
                    // 奇数端数: そのまま f64 で保存 (簡易)
                    result.extend_from_slice(&chunk[0].to_le_bytes());
                }
            }
        }

        result
    }

    /// ベクトルのデコード
    pub fn decode(&self, compressed: &[u8], dim: usize) -> Vec<f64> {
        if compressed.len() < 2 {
            return vec![0.0; dim];
        }

        let _version = compressed[0];
        let flags = compressed[1];
        let mut result = Vec::with_capacity(dim);

        if flags == 1 {
            // Raw
            let mut i = 2;
            while i + 8 <= compressed.len() && result.len() < dim {
                let bytes: [u8; 8] = compressed[i..i + 8].try_into().unwrap_or([0; 8]);
                result.push(f64::from_le_bytes(bytes));
                i += 8;
            }
        } else {
            // Compressed
            let _bits = compressed[2];
            let mut i = 3;
            while result.len() < dim {
                if dim - result.len() >= 2 && i < compressed.len() {
                    let byte = compressed[i];
                    let r_q = (byte >> 4) & 0x0F;
                    let theta_q = byte & 0x0F;

                    let r = ((r_q as f64) / 15.0) * 2.0;
                    let theta = (theta_q as f64 / 15.0) * (2.0 * std::f64::consts::PI)
                        - std::f64::consts::PI;

                    result.push(r * theta.cos());
                    result.push(r * theta.sin());
                    i += 1;
                } else if i + 8 <= compressed.len() {
                    // 端数
                    let bytes: [u8; 8] = compressed[i..i + 8].try_into().unwrap_or([0; 8]);
                    result.push(f64::from_le_bytes(bytes));
                    i += 8;
                } else {
                    break;
                }
            }
        }

        // 足りない分をパディング
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
    fn test_low_dim_fallback() {
        let encoder = PolarQuantEncoder::new(3, 32);
        let vector = vec![1.0, 2.0, 3.0]; // 3 dims < 32
        let encoded = encoder.encode(&vector);
        // Fallback should return raw bytes (8 bytes * 3 = 24 bytes) + version tag etc.
        // For now, let's just assert it's not empty and has expected size ratio
        assert!(encoded.len() >= 24);

        let decoded = encoder.decode(&encoded, 3);
        assert_eq!(vector, decoded);
    }

    #[test]
    fn test_encode_decode_roundtrip_768d() {
        let encoder = PolarQuantEncoder::new(4, 32);
        let vector: Vec<f64> = (0..768).map(|i| i as f64 / 768.0).collect();
        let encoded = encoder.encode(&vector);

        // 4 bits per dimension ideally, so 768 * 4 / 8 = 384 bytes + metadata
        assert!(encoded.len() < 500);

        let decoded = encoder.decode(&encoded, 768);
        assert_eq!(decoded.len(), 768);

        // Accuracy check: standard error should be low
        let mut mse = 0.0;
        for (a, b) in vector.iter().zip(decoded.iter()) {
            mse += (a - b).powi(2);
        }
        mse /= 768.0;
        assert!(mse < 0.01);
    }
}

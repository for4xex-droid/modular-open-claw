/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use anyhow::{anyhow, Result};
use rubato::{Resampler, SincFixedIn, SincInterpolationParameters, SincInterpolationType};

/// PCM 音声データのサンプリングレートを変換するためのエンジン
pub struct PcmResampler {
    resampler: SincFixedIn<f32>,
    input_rate: u32,
    output_rate: u32,
}

impl PcmResampler {
    /// 新しいリサンプラーを作成します
    pub fn new(input_rate: u32, output_rate: u32, chunk_size: usize) -> Result<Self> {
        let params = SincInterpolationParameters {
            sinc_len: 256,
            f_cutoff: 0.95,
            interpolation: SincInterpolationType::Nearest,
            oversampling_factor: 256,
            window: rubato::WindowFunction::Blackman2,
        };

        let resampler = SincFixedIn::<f32>::new(
            output_rate as f64 / input_rate as f64,
            2.0, // max ratio error
            params,
            chunk_size,
            1, // mono
        )
        .map_err(|e| anyhow!("Failed to initialize resampler: {}", e))?;

        Ok(Self {
            resampler,
            input_rate,
            output_rate,
        })
    }

    /// i16 PCM バイト列を受け取り、リサンプリング後の i16 バイト列を返します
    pub fn resample(&mut self, input_bytes: &[u8]) -> Result<Vec<u8>> {
        // バイト列を f32 に変換 (Mono想定)
        let mut input_f32 = input_bytes
            .chunks_exact(2)
            .map(|c| {
                let s = i16::from_le_bytes([c[0], c[1]]);
                s as f32 / 32768.0
            })
            .collect::<Vec<f32>>();

        // rubato は固定チャンクサイズを期待するため、入力をパディングまたは分割する必要がある
        let needed = self.resampler.input_frames_next();
        if input_f32.len() < needed {
            input_f32.resize(needed, 0.0);
        } else if input_f32.len() > needed {
            input_f32.truncate(needed);
        }

        let output = self
            .resampler
            .process(&[input_f32], None)
            .map_err(|e| anyhow!("Resampling process failed: {}", e))?;

        // f32 を i16 バイト列に戻す
        let resampled_bytes = output[0]
            .iter()
            .flat_map(|sample: &f32| {
                let s = (sample.clamp(-1.0, 1.0) * 32767.0) as i16;
                s.to_le_bytes().to_vec()
            })
            .collect();

        Ok(resampled_bytes)
    }

    pub fn input_rate(&self) -> u32 {
        self.input_rate
    }
    pub fn output_rate(&self) -> u32 {
        self.output_rate
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resampler_creation() {
        let r = PcmResampler::new(16000, 48000, 1024);
        assert!(r.is_ok());
    }
}

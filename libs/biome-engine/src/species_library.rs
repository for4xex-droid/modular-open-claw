/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
//! Lenia 正典生物の種ライブラリ。
//!
//! 手続き的なリングスタンプ（`lenia::seed_orbium_ring`）は「異シードでも同一の
//! ベタ塗りテクスチャに収束する」問題があった（実機計測: 8 シードすべて mass≈3400）。
//! ここでは Chakazul/Lenia の `animals.json` から抽出した正典パターン（RLE）を
//! 静的に埋め込み、シードごとに異なる「本物のソリトン生物」を初期配置する。
//!
//! 選定 5 種はいずれもカーネル半径 R=13（既存 `KERNEL_RADIUS` と一致）で、
//! 128×128 トロイダル・dt=0.1 で生存することを PoC で検証済み。

/// 正典 Lenia 生物 1 種の定義。
///
/// `rle` は Lenia 正典 RLE 形式（255 段階・`pqrstuvwxy` プレフィックス）。
/// パターンサイズはグリッド（128×128）に収まる中小型のみを採用している。
pub struct SpeciesSeed {
    pub name: &'static str,
    /// 成長関数の中心 μ
    pub mu: f32,
    /// 成長関数の幅 σ
    pub sigma: f32,
    /// カーネル半径（全種 13 = KERNEL_RADIUS）
    pub radius: usize,
    /// 正典 RLE パターン
    pub rle: &'static str,
}

/// 埋め込み種テーブル。PoC で「128 グリッドで生存・非膨張」を確認した種のみ。
pub const SPECIES: &[SpeciesSeed] = &[
    SpeciesSeed {
        name: "Orbium unicaudatus",
        mu: 0.15,
        sigma: 0.015,
        radius: 13,
        rle: include_str!("species/orbium_unicaudatus.rle"),
    },
    SpeciesSeed {
        name: "Gyrorbium gyrans",
        mu: 0.156,
        sigma: 0.0224,
        radius: 13,
        rle: include_str!("species/gyrorbium_gyrans.rle"),
    },
    SpeciesSeed {
        name: "Parorbium dividuus",
        mu: 0.174,
        sigma: 0.022,
        radius: 13,
        rle: include_str!("species/parorbium_dividuus.rle"),
    },
    SpeciesSeed {
        name: "Pentahelicium solidus",
        mu: 0.34,
        sigma: 0.045,
        radius: 13,
        rle: include_str!("species/pentahelicium_solidus.rle"),
    },
    SpeciesSeed {
        name: "Pentascutium solidus",
        mu: 0.422,
        sigma: 0.0858,
        radius: 13,
        rle: include_str!("species/pentascutium_solidus.rle"),
    },
];

/// 2D パターン（行優先の値配列 [0.0, 1.0]）。
pub struct DecodedPattern {
    pub width: usize,
    pub height: usize,
    /// height * width、行優先。
    pub cells: Vec<f32>,
}

/// Lenia 正典 RLE をデコードする。
///
/// 仕様（`LeniaND.py::rle2arr` 準拠）:
/// - 数字は直後の 1 セル値の繰り返し回数
/// - `.` / `b` = 0
/// - `A`..`Y`（プレフィックスなし）= 1..25 の値（`(c-'A'+1)/255`）
/// - `pqrstuvwxy` はプレフィックス。`p`+`X` = `(p_index*24 + (X-'A'+25))/255`
/// - `$` = 行末
/// - `!` = 終端
///
/// 範囲外・未知の文字はスキップ（0 として扱う）ことで、不正入力でも panic せず
/// 空/部分パターンにフォールバックする（Negative Test で担保）。
pub fn decode_rle(rle: &str) -> DecodedPattern {
    let mut rows: Vec<Vec<f32>> = vec![Vec::new()];
    let mut count = String::new();
    let mut last: Option<char> = None;

    for ch in rle.chars() {
        match ch {
            '0'..='9' => count.push(ch),
            'p'..='y' => last = Some(ch),
            '$' | '\n' => {
                if ch == '$' {
                    rows.push(Vec::new());
                }
                count.clear();
                last = None;
            }
            '!' => break,
            _ => {
                let value = char_to_value(last, ch);
                let rep: usize = if count.is_empty() {
                    1
                } else {
                    count.parse().unwrap_or(1)
                };
                if let Some(row) = rows.last_mut() {
                    for _ in 0..rep {
                        row.push(value);
                    }
                }
                count.clear();
                last = None;
            }
        }
    }

    rows.retain(|r| !r.is_empty());
    let height = rows.len();
    let width = rows.iter().map(|r| r.len()).max().unwrap_or(0);
    let mut cells = vec![0.0f32; width * height];
    for (y, row) in rows.iter().enumerate() {
        for (x, &v) in row.iter().enumerate() {
            cells[y * width + x] = v;
        }
    }

    DecodedPattern {
        width,
        height,
        cells,
    }
}

/// RLE の 1 文字（＋任意のプレフィックス）を [0.0, 1.0] のセル値に変換。
fn char_to_value(prefix: Option<char>, ch: char) -> f32 {
    let raw: i32 = match (prefix, ch) {
        (_, '.') | (_, 'b') => 0,
        (_, 'o') => 255,
        (None, 'A'..='Y') => ch as i32 - 'A' as i32 + 1,
        (Some(p), 'A'..='X') => (p as i32 - 'p' as i32) * 24 + (ch as i32 - 'A' as i32 + 25),
        // 未知の組み合わせは 0（不正入力フォールバック）
        _ => 0,
    };
    (raw.clamp(0, 255) as f32) / 255.0
}

/// シード値から種を 1 つ決定的に選ぶ（同一シード＝同一種）。
pub fn select_species(seed: u64) -> &'static SpeciesSeed {
    let idx = (seed % SPECIES.len() as u64) as usize;
    &SPECIES[idx]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_species_decode_nonempty() {
        for sp in SPECIES {
            let p = decode_rle(sp.rle);
            assert!(
                p.width > 0 && p.height > 0,
                "{} should decode to a non-empty pattern, got {}x{}",
                sp.name,
                p.width,
                p.height
            );
            assert_eq!(p.cells.len(), p.width * p.height);
            let max = p.cells.iter().cloned().fold(0.0f32, f32::max);
            assert!(
                max > 0.5,
                "{} should contain live cells (max value {})",
                sp.name,
                max
            );
        }
    }

    #[test]
    fn test_species_fit_grid() {
        // 全種が 128×128 グリッドに余裕を持って収まること
        for sp in SPECIES {
            let p = decode_rle(sp.rle);
            assert!(
                p.width < 110 && p.height < 110,
                "{} ({}x{}) must fit within the 128 grid",
                sp.name,
                p.width,
                p.height
            );
        }
    }

    #[test]
    fn test_decode_rejects_garbage_without_panic() {
        // Negative Test: 不正な RLE を投入しても panic せず空/部分にフォールバック
        let p = decode_rle("###@@@%%%zzz");
        assert!(p.cells.iter().all(|&v| v == 0.0));
        let p2 = decode_rle("");
        assert_eq!(p2.width, 0);
        assert_eq!(p2.height, 0);
    }

    #[test]
    fn test_select_species_deterministic() {
        assert_eq!(select_species(0).name, select_species(0).name);
        assert_eq!(
            select_species(SPECIES.len() as u64).name,
            SPECIES[0].name,
            "seed wrapping should map back to the first species"
        );
    }
}

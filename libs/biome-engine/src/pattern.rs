/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
use crate::grid::{BiomeCell, GRID_HEIGHT, GRID_SIZE, GRID_WIDTH};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PatternMetrics {
    pub symmetry_score: f32,
    pub complexity_score: f32,
    pub cluster_count: u16,
    /// 活性領域の外接矩形がグリッド全体に占める割合 [0.0, 1.0]。
    /// 小さいほど「局在した生物」、1.0 に近いほど「一面に広がったテクスチャ」。
    /// レアリティ判定で「生物的な局在性」を要求するために使う。
    pub bbox_ratio: f32,
}

/// 活性マスクから外接矩形の占有率を計算する。
fn compute_bbox_ratio(mask: &[bool]) -> f32 {
    let mut min_x = GRID_WIDTH;
    let mut max_x = 0usize;
    let mut min_y = GRID_HEIGHT;
    let mut max_y = 0usize;
    let mut any = false;
    for y in 0..GRID_HEIGHT {
        for x in 0..GRID_WIDTH {
            if mask[y * GRID_WIDTH + x] {
                any = true;
                min_x = min_x.min(x);
                max_x = max_x.max(x);
                min_y = min_y.min(y);
                max_y = max_y.max(y);
            }
        }
    }
    if !any {
        return 0.0;
    }
    let w = (max_x - min_x + 1) as f32;
    let h = (max_y - min_y + 1) as f32;
    (w * h) / GRID_SIZE as f32
}

/// 活性セルマスクから構造美指標を計算する
pub fn measure(cells: &[BiomeCell]) -> PatternMetrics {
    let mut active_mask = vec![false; GRID_SIZE];
    let mut active_count = 0usize;
    let mut sum_x = 0f64;
    let mut sum_y = 0f64;

    for (idx, cell) in cells.iter().enumerate().take(GRID_SIZE) {
        if cell.active {
            active_mask[idx] = true;
            active_count += 1;
            let x = (idx % GRID_WIDTH) as f64;
            let y = (idx / GRID_WIDTH) as f64;
            sum_x += x;
            sum_y += y;
        }
    }

    if active_count == 0 {
        return PatternMetrics {
            symmetry_score: 0.0,
            complexity_score: 0.0,
            cluster_count: 0,
            bbox_ratio: 0.0,
        };
    }

    let cx = sum_x / active_count as f64;
    let cy = sum_y / active_count as f64;

    let symmetry_score = compute_symmetry(&active_mask, cx, cy);
    let (area, perimeter) = compute_area_perimeter(&active_mask);
    let complexity_score = if perimeter > 0 {
        let isoperimetric =
            (4.0 * std::f32::consts::PI * area as f32) / (perimeter as f32 * perimeter as f32);
        (1.0 - isoperimetric).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let cluster_count = count_clusters(&active_mask);
    let bbox_ratio = compute_bbox_ratio(&active_mask);

    PatternMetrics {
        symmetry_score,
        complexity_score,
        cluster_count,
        bbox_ratio,
    }
}

/// Lenia 連続場から構造美指標を計算する（threshold 以上を活性とみなす）
pub fn measure_field(field: &[f32], threshold: f32) -> PatternMetrics {
    let mut active_mask = vec![false; GRID_SIZE];
    let mut active_count = 0usize;
    let mut sum_x = 0f64;
    let mut sum_y = 0f64;

    for (idx, &v) in field.iter().enumerate().take(GRID_SIZE) {
        if v > threshold {
            active_mask[idx] = true;
            active_count += 1;
            let x = (idx % GRID_WIDTH) as f64;
            let y = (idx / GRID_WIDTH) as f64;
            sum_x += x;
            sum_y += y;
        }
    }

    if active_count == 0 {
        return PatternMetrics {
            symmetry_score: 0.0,
            complexity_score: 0.0,
            cluster_count: 0,
            bbox_ratio: 0.0,
        };
    }

    let cx = sum_x / active_count as f64;
    let cy = sum_y / active_count as f64;

    let symmetry_score = compute_symmetry(&active_mask, cx, cy);
    let (area, perimeter) = compute_area_perimeter(&active_mask);
    let complexity_score = if perimeter > 0 {
        let isoperimetric =
            (4.0 * std::f32::consts::PI * area as f32) / (perimeter as f32 * perimeter as f32);
        (1.0 - isoperimetric).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let cluster_count = count_clusters(&active_mask);
    let bbox_ratio = compute_bbox_ratio(&active_mask);

    PatternMetrics {
        symmetry_score,
        complexity_score,
        cluster_count,
        bbox_ratio,
    }
}

fn compute_symmetry(mask: &[bool], cx: f64, cy: f64) -> f32 {
    let mut h_matches = 0usize;
    let mut h_total = 0usize;
    let mut v_matches = 0usize;
    let mut v_total = 0usize;

    for y in 0..GRID_HEIGHT {
        for x in 0..GRID_WIDTH {
            let idx = y * GRID_WIDTH + x;
            if !mask[idx] {
                continue;
            }

            // 水平鏡映 (y = cy)
            let mirror_y = (2.0 * cy - y as f64).round() as isize;
            if mirror_y >= 0 && (mirror_y as usize) < GRID_HEIGHT {
                h_total += 1;
                let mirror_idx = mirror_y as usize * GRID_WIDTH + x;
                if mask[mirror_idx] {
                    h_matches += 1;
                }
            }

            // 垂直鏡映 (x = cx)
            let mirror_x = (2.0 * cx - x as f64).round() as isize;
            if mirror_x >= 0 && (mirror_x as usize) < GRID_WIDTH {
                v_total += 1;
                let mirror_idx = y * GRID_WIDTH + mirror_x as usize;
                if mask[mirror_idx] {
                    v_matches += 1;
                }
            }
        }
    }

    let h_score = if h_total > 0 {
        h_matches as f32 / h_total as f32
    } else {
        0.0
    };
    let v_score = if v_total > 0 {
        v_matches as f32 / v_total as f32
    } else {
        0.0
    };

    h_score.max(v_score)
}

fn compute_area_perimeter(mask: &[bool]) -> (usize, usize) {
    let mut area = 0usize;
    let mut perimeter = 0usize;

    for y in 0..GRID_HEIGHT {
        for x in 0..GRID_WIDTH {
            let idx = y * GRID_WIDTH + x;
            if !mask[idx] {
                continue;
            }
            area += 1;
            let neighbors = [x.wrapping_sub(1), x + 1, y.wrapping_sub(1), y + 1];
            let dirs = [
                (neighbors[0], y),
                (neighbors[1], y),
                (x, neighbors[2]),
                (x, neighbors[3]),
            ];
            for &(nx, ny) in &dirs {
                if nx >= GRID_WIDTH || ny >= GRID_HEIGHT || !mask[ny * GRID_WIDTH + nx] {
                    perimeter += 1;
                }
            }
        }
    }

    (area, perimeter)
}

fn count_clusters(mask: &[bool]) -> u16 {
    let mut visited = vec![false; GRID_SIZE];
    let mut clusters = 0u16;

    for idx in 0..GRID_SIZE {
        if !mask[idx] || visited[idx] {
            continue;
        }
        clusters += 1;
        flood_fill(mask, &mut visited, idx);
    }

    clusters
}

fn flood_fill(mask: &[bool], visited: &mut [bool], start: usize) {
    let mut stack = vec![start];
    while let Some(idx) = stack.pop() {
        if visited[idx] || !mask[idx] {
            continue;
        }
        visited[idx] = true;
        let x = idx % GRID_WIDTH;
        let y = idx / GRID_WIDTH;
        let neighbors = [
            (x.wrapping_sub(1), y),
            (x + 1, y),
            (x, y.wrapping_sub(1)),
            (x, y + 1),
        ];
        for &(nx, ny) in &neighbors {
            if nx < GRID_WIDTH && ny < GRID_HEIGHT {
                let n_idx = ny * GRID_WIDTH + nx;
                if mask[n_idx] && !visited[n_idx] {
                    stack.push(n_idx);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::genome::CellGenome;

    fn make_grid_with_active(coords: &[(usize, usize)]) -> Vec<BiomeCell> {
        let mut cells = vec![BiomeCell::new(); GRID_SIZE];
        for &(x, y) in coords {
            cells[y * GRID_WIDTH + x].active = true;
        }
        cells
    }

    #[test]
    fn test_empty_grid_zero_metrics() {
        let cells = vec![BiomeCell::new(); GRID_SIZE];
        let m = measure(&cells);
        assert_eq!(m.symmetry_score, 0.0);
        assert_eq!(m.complexity_score, 0.0);
        assert_eq!(m.cluster_count, 0);
    }

    #[test]
    fn test_cross_has_high_symmetry() {
        // 十字型 (64,64) 中心
        let coords: Vec<(usize, usize)> = (60..=68).flat_map(|i| [(i, 64), (64, i)]).collect();
        let cells = make_grid_with_active(&coords);
        let m = measure(&cells);
        assert!(
            m.symmetry_score > 0.85,
            "Cross should have high symmetry, got {}",
            m.symmetry_score
        );
    }

    #[test]
    fn test_filled_disk_low_complexity() {
        // 半径5の円盤
        let coords: Vec<(usize, usize)> = (59..=69)
            .flat_map(|y| (59..=69).map(move |x| (x, y)))
            .filter(|&(x, y)| {
                let dx = x as i32 - 64;
                let dy = y as i32 - 64;
                dx * dx + dy * dy <= 25
            })
            .collect();
        let cells = make_grid_with_active(&coords);
        let m = measure(&cells);
        assert!(
            m.complexity_score < 0.55,
            "Disk should have low complexity, got {}",
            m.complexity_score
        );
    }

    #[test]
    fn test_checkerboard_high_cluster_count() {
        let coords: Vec<(usize, usize)> = (0..32)
            .flat_map(|y| (0..32).map(move |x| (x * 2 + (y % 2), y * 2)))
            .filter(|&(x, y)| x < GRID_WIDTH && y < GRID_HEIGHT)
            .collect();
        let cells = make_grid_with_active(&coords);
        let m = measure(&cells);
        assert!(
            m.cluster_count > 50,
            "Checkerboard should have many clusters, got {}",
            m.cluster_count
        );
    }

    #[test]
    fn test_prismatic_genome_marker() {
        let mut genome = CellGenome::default_nurture();
        assert!(!genome.is_prismatic());
        genome.set_prismatic();
        assert!(genome.is_prismatic());
    }
}

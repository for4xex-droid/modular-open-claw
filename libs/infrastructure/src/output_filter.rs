/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

/// フィルタレベル
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterLevel {
    /// キーポイントのみ抽出（最大圧縮）
    Aggressive,
    /// エラー・警告を保持、ボイラープレート除去
    Balanced,
    /// 冗長な空白とコメントのみ除去
    Minimal,
}

/// フィルタ戦略
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterStrategy {
    /// git status / diff 出力の圧縮
    GitOutput,
    /// cargo test / build 出力の圧縮
    CargoOutput,
    /// npm / node 出力の圧縮
    NodeOutput,
    /// 汎用コマンド出力の圧縮
    Generic,
}

/// フィルタ結果
#[derive(Debug, Clone)]
pub struct FilterResult {
    pub filtered_output: String,
    pub original_chars: usize,
    pub filtered_chars: usize,
    pub compression_ratio: f64,
}

/// コマンド出力をLLM向けに圧縮するフィルタ
pub struct OutputFilter;

impl OutputFilter {
    /// コマンド出力をフィルタリングする
    pub fn filter(raw_output: &str, strategy: FilterStrategy, level: FilterLevel) -> FilterResult {
        let lines: Vec<&str> = raw_output.lines().collect();

        // Step 1: ボイラープレート削除
        let filtered_lines = Self::remove_boilerplate(&lines, strategy, level);

        // Step 2: 重複した行の圧縮（Balanced or Aggressive のみ適用）
        let final_lines: Vec<String> =
            if level == FilterLevel::Balanced || level == FilterLevel::Aggressive {
                Self::deduplicate_lines(&filtered_lines)
            } else {
                // Minimal: StringのVecに変換するだけ
                filtered_lines.into_iter().map(|s| s.to_string()).collect()
            };

        let filtered_output = final_lines.join("\n");
        let filtered_chars = filtered_output.chars().count();
        let original_chars = raw_output.chars().count();

        FilterResult {
            filtered_output,
            original_chars,
            filtered_chars,
            compression_ratio: if original_chars == 0 {
                0.0
            } else {
                1.0 - (filtered_chars as f64 / original_chars as f64)
            },
        }
    }

    /// 重複行を折りたたんでカウントを付与する
    fn deduplicate_lines(lines: &[&str]) -> Vec<String> {
        let mut result = Vec::new();
        if lines.is_empty() {
            return result;
        }

        let mut current_line = lines[0];
        let mut count = 1;

        for i in 1..lines.len() {
            if lines[i] == current_line {
                count += 1;
            } else {
                // 空行の連続はそのまま空行1つにする
                if current_line.trim().is_empty() {
                    result.push("".to_string());
                } else if count > 1 {
                    result.push(format!("{} (Repeated {} times)", current_line, count));
                } else {
                    result.push(current_line.to_string());
                }
                current_line = lines[i];
                count = 1;
            }
        }

        if current_line.trim().is_empty() {
            result.push("".to_string());
        } else if count > 1 {
            result.push(format!("{} (Repeated {} times)", current_line, count));
        } else {
            result.push(current_line.to_string());
        }

        result
    }

    /// 余分なログ、情報メッセージを消す
    fn remove_boilerplate<'a>(
        lines: &'a [&'a str],
        strategy: FilterStrategy,
        _level: FilterLevel,
    ) -> Vec<&'a str> {
        lines
            .iter()
            .filter(|line| {
                let trimmed = line.trim();
                match strategy {
                    FilterStrategy::CargoOutput => {
                        !(trimmed.starts_with("test test_") && trimmed.ends_with(" ... ok"))
                    }
                    FilterStrategy::GitOutput => {
                        !(trimmed.starts_with("(use \"git")
                            || trimmed.starts_with("no changes added to commit"))
                    }
                    FilterStrategy::NodeOutput => {
                        !(trimmed.starts_with("npm notice") || trimmed.starts_with("npm WARN"))
                    }
                    _ => true,
                }
            })
            .copied()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_input_safety() {
        let result = OutputFilter::filter("", FilterStrategy::Generic, FilterLevel::Balanced);
        assert_eq!(result.filtered_output, "");
        assert_eq!(result.original_chars, 0);
        assert_eq!(result.filtered_chars, 0);
        assert_eq!(result.compression_ratio, 0.0);
    }

    #[test]
    fn test_binary_input_safety() {
        // null バイトを含む文字列でもパニックしないかテスト
        let raw = "binary\x00data\x01\x02";
        let result = OutputFilter::filter(raw, FilterStrategy::Generic, FilterLevel::Balanced);
        // 現在地のテストは何も通らない（"未実装"を返すだけ）なのでここでは最低限パニックしないかを確認
        // だたし、未実装ダミー実装では String::new() を返すため以下のテストは失敗するはず
        assert!(result.filtered_output.contains("binary"));
    }

    #[test]
    fn test_deduplication() {
        let raw = "line1\nline1\nline1\nline2\nline3\nline3\n";
        let result = OutputFilter::filter(raw, FilterStrategy::Generic, FilterLevel::Balanced);
        let out = &result.filtered_output;

        // "line1 x 3" のように折りたたまれるか、単に1行にまとめられることを期待
        // ここでは簡単に "line1" が1つだけ、"line2" が1つ、"line3" が1つ含まれているかを確認
        let lines: Vec<&str> = out.lines().collect();
        assert_eq!(lines.len(), 3);
        assert!(lines[0].starts_with("line1"));
        assert!(lines[1].starts_with("line2"));
        assert!(lines[2].starts_with("line3"));
    }

    #[test]
    fn test_node_output_compression() {
        let raw = "npm notice New major version\nnpm WARN deprecated\nadded 1 package";
        let result = OutputFilter::filter(raw, FilterStrategy::NodeOutput, FilterLevel::Balanced);
        let out = &result.filtered_output;

        assert!(!out.contains("npm notice"));
        assert!(!out.contains("npm WARN"));
        assert!(out.contains("added 1 package"));
    }

    #[test]
    fn test_cargo_test_compression() {
        let raw = r#"
running 150 tests
test test_a ... ok
test test_b ... ok
test test_c ... FAILED
test test_d ... ok

failures:

---- test_c stdout ----
thread 'test_c' panicked at 'assertion failed'

failures:
    test_c

test result: FAILED. 149 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.23s
"#;
        let result = OutputFilter::filter(raw, FilterStrategy::CargoOutput, FilterLevel::Balanced);
        let out = &result.filtered_output;

        // エラーの本質（test_c が fail したこと、panic 内容）が残っていること
        assert!(out.contains("FAILED"));
        assert!(out.contains("panicked at"));
        assert!(out.contains("test_c"));

        // 成功したテスト行などのボイラープレートが消えていること
        assert!(!out.contains("test test_a ... ok"));
        assert!(!out.contains("test test_b ... ok"));
        assert!(!out.contains("test test_d ... ok"));
    }

    #[test]
    fn test_git_status_compression() {
        let raw = r#"
On branch feature/token-optimization
Your branch is up to date with 'origin/feature/token-optimization'.

Changes not staged for commit:
  (use "git add <file>..." to update what will be committed)
  (use "git restore <file>..." to discard changes in working directory)
        modified:   libs/infrastructure/src/slm_bridge.rs
        modified:   libs/infrastructure/src/context_engine.rs

Untracked files:
  (use "git add <file>..." to include in what will be committed)
        libs/infrastructure/src/output_filter.rs

no changes added to commit (use "git add" and/or "git commit -a")
"#;
        let result = OutputFilter::filter(raw, FilterStrategy::GitOutput, FilterLevel::Balanced);
        let out = &result.filtered_output;

        // 変更されたファイル名のみが残り、ヒントメッセージが消えることを期待
        assert!(out.contains("modified:   libs/infrastructure/src/slm_bridge.rs"));
        assert!(out.contains("libs/infrastructure/src/output_filter.rs"));
        assert!(!out.contains("(use \"git add"));
        assert!(!out.contains("(use \"git restore"));
    }
}

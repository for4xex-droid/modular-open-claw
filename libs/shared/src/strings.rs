/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
use std::borrow::Cow;

/// 安全な文字列操作ユーティリティ
/// パニックを引き起こさず、アロケーションを最小化した高効率な切り詰めを提供します。

const ELLIPSIS: &str = "... (truncated)";

/// バイト数に基づいて文字列を安全に切り詰めます。
///
/// # 安全性
/// マルチバイト文字の境界を考慮し、不正なUTF-8シーケンスを作成しません（パニック回避）。
///
/// # パフォーマンス
/// 切り詰めが不要な場合はアロケーションを行わず、元の文字列の参照を返します。
pub fn truncate_bytes_safely<S: AsRef<str> + ?Sized>(s: &S, max_bytes: usize) -> Cow<'_, str> {
    let s = s.as_ref();
    if s.len() <= max_bytes {
        return Cow::Borrowed(s);
    }
    let mut end = max_bytes;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    Cow::Borrowed(&s[..end])
}

/// 文字数（Unicodeコードポイント）に基づいて文字列を安全に切り詰めます。
///
/// # 特徴
/// - `append_ellipsis` が true の場合、切り詰めが発生した時のみ末尾に追記します。
/// - 切り詰めが不要な場合は、コピーせずに参照（Cow::Borrowed）を返し、ゼロアロケーションを実現します。
/// - 切り詰めが発生する場合でも、`String::with_capacity` によりメモリ再確保を最小化します。
///
/// # 注意
/// 結合文字（Grapheme Clusters / 例: 👨‍👩‍👧‍👦）の中間で切断される可能性があります。
/// メモリ安全性は保たれますが、表示上の整合性（文字化け）が必要な場合は unicode-segmentation の使用を検討してください。
pub fn truncate_chars_safely<S: AsRef<str> + ?Sized>(
    s: &S,
    max_chars: usize,
    append_ellipsis: bool,
) -> Cow<'_, str> {
    let s = s.as_ref();

    // 0文字指定の特殊最適化
    if max_chars == 0 {
        if append_ellipsis {
            return Cow::Owned(ELLIPSIS.to_string());
        } else {
            return Cow::Borrowed("");
        }
    }

    // 文字の境界（バイトインデックス）を正確に特定（O(max_chars)）
    let mut indices = s.char_indices();
    if let Some((idx, _)) = indices.nth(max_chars) {
        // 切り詰めが必要
        if append_ellipsis {
            // 容量を事前に確保（切り詰め部分のバイト数 + 省略記号のバイト数）
            let mut truncated = String::with_capacity(idx + ELLIPSIS.len());
            truncated.push_str(&s[..idx]);
            truncated.push_str(ELLIPSIS);
            Cow::Owned(truncated)
        } else {
            // 省略記号がない場合は、アロケーションして返す（参照では返せないため）
            Cow::Owned(s[..idx].to_string())
        }
    } else {
        // そもそも max_chars 以内の長さ
        Cow::Borrowed(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_bytes_safely() {
        let s = "あいう"; // 9 bytes
        assert_eq!(truncate_bytes_safely(s, 9), "あいう");
        assert_eq!(truncate_bytes_safely(s, 5), "あ");
        assert_eq!(truncate_bytes_safely(s, 0), "");
        assert!(matches!(truncate_bytes_safely(s, 5), Cow::Borrowed(_)));
    }

    #[test]
    fn test_truncate_chars_safely_no_alloc() {
        let s = "あいうえお";
        let result = truncate_chars_safely(s, 5, true);
        assert!(matches!(result, Cow::Borrowed(_)));
        assert_eq!(result, "あいうえお");
    }

    #[test]
    fn test_truncate_chars_safely_alloc_and_capacity() {
        let s = "あいうえお";
        let result = truncate_chars_safely(s, 2, true);
        assert_eq!(result, "あい... (truncated)");
        // アロケートされた String の容量が効率的か確認（RustのString実装に依存するが、少なくとも切り詰めサイズ以上はある）
        if let Cow::Owned(ref inner) = result {
            assert!(inner.capacity() >= 6 + ELLIPSIS.len());
        }
    }

    #[test]
    fn test_truncate_chars_safely_zero_edge() {
        let s = "A";
        assert_eq!(truncate_chars_safely(s, 0, false), "");
        assert!(matches!(
            truncate_chars_safely(s, 0, false),
            Cow::Borrowed(_)
        ));

        assert_eq!(truncate_chars_safely(s, 0, true), "... (truncated)");
        assert!(matches!(truncate_chars_safely(s, 0, true), Cow::Owned(_)));
    }

    #[test]
    fn test_empty_input() {
        let s = "";
        assert_eq!(truncate_chars_safely(s, 10, true), "");
        assert!(matches!(
            truncate_chars_safely(s, 10, true),
            Cow::Borrowed(_)
        ));
    }
}

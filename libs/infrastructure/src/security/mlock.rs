/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */
#![allow(unsafe_code)]

use libc::{mlock, munlock, size_t};
use std::ops::{Deref, DerefMut};
use zeroize::Zeroize;

/// メモリ上に固定(mlock)され、Drop時にゼロ消去(Zeroize)される鍵・シークレット用ベクタ
/// Zeroizing<Vec<u8>> の代替として、より強力なメモリ保護を提供する。
pub struct MlockedVec {
    inner: Vec<u8>,
    locked: bool,
}

impl MlockedVec {
    /// 新しいメモリ固定領域を作成する
    pub fn new(mut data: Vec<u8>) -> Self {
        let ptr = data.as_ptr() as *const libc::c_void;
        let len = data.len() as size_t;
        let mut locked = false;

        if len > 0 {
            unsafe {
                if mlock(ptr, len) != 0 {
                    // 権限不足やリミット超過の場合は警告。
                    // 開発環境(非root)では失敗することが多いためパニックはさせない。
                    tracing::warn!(
                        "🔐 [Security] mlock() failed. Memory may be swappable. (Check ulimit -l)"
                    );
                } else {
                    locked = true;
                    tracing::debug!("🔐 [Security] Memory locked (mlock) successfully.");
                }
            }
        }
        Self {
            inner: data,
            locked,
        }
    }

    /// メモリ固定に成功したかどうかを返す
    pub fn is_locked(&self) -> bool {
        self.locked
    }

    /// Zeroizing<Vec<u8>> に変換する (クローンを伴う)
    pub fn to_zeroizing(&self) -> zeroize::Zeroizing<Vec<u8>> {
        zeroize::Zeroizing::new(self.inner.clone())
    }
}

impl std::fmt::Debug for MlockedVec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MlockedVec")
            .field("inner", &"***REDACTED***")
            .finish()
    }
}

impl Drop for MlockedVec {
    fn drop(&mut self) {
        let ptr = self.inner.as_ptr() as *const libc::c_void;
        let len = self.inner.len() as size_t;

        unsafe {
            // 1. ゼロ消去 (Zeroize) - 常に実行
            self.inner.zeroize();

            // 2. メモリ固定解除 - 成功していた場合のみ
            if self.locked && len > 0 && munlock(ptr, len) != 0 {
                // 失敗してもログのみ
                tracing::error!("🚨 [Security] munlock() failed.");
            }
        }
    }
}

impl Deref for MlockedVec {
    type Target = Vec<u8>;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for MlockedVec {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl Clone for MlockedVec {
    fn clone(&self) -> Self {
        // クローン時も新しい領域を mlock する
        Self::new(self.inner.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mlocked_vec_lifecycle() {
        let data = vec![1, 2, 3, 4, 5];
        let m = MlockedVec::new(data.clone());

        assert_eq!(*m, data);
        // mlock 成功/失敗によらず機能は維持される
        drop(m);
    }
}

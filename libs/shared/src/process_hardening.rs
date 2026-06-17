/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

//! プロセスの堅牢化モジュール
//!
//! ptraceの無効化、コアダンプの防止、環境変数のスクラブなどを行います。

use std::env;

/// 環境変数から特定のプレフィックスを持つものを一括削除する
///
/// # Safety
/// 内部で `std::env::remove_var` を呼び出します。
/// マルチスレッド環境で実行すると未定義動作を引き起こす可能性があります。
/// そのため、この関数はプログラムの開始直後、スレッドが生成される前に呼び出す必要があります。
#[allow(unsafe_code)]
pub fn scrub_env_prefix(prefix: &str) {
    let keys_to_remove: Vec<String> = env::vars()
        .filter(|(k, _)| k.starts_with(prefix))
        .map(|(k, _)| k)
        .collect();

    for key in keys_to_remove {
        // SAFETY: pre_main_hardening からシングルスレッド環境で呼ばれる前提
        unsafe {
            env::remove_var(&key);
        }
    }
}

/// プロセスの堅牢化を事前（main開始直後）に行う
/// ptrace の無効化、コアダンプの防止などをOSに応じて実行します。
pub fn pre_main_hardening() {
    // スレッドセーフティのため、環境変数の削除は一番最初に行う
    #[cfg(target_os = "macos")]
    scrub_env_prefix("DYLD_");

    #[cfg(target_os = "linux")]
    scrub_env_prefix("LD_");

    // デバッグ時はデバッガへのアタッチやコアダンプが必要なため無効化しない
    if cfg!(debug_assertions) {
        return;
    }

    disable_core_dumps();
    disable_ptrace();
}

#[cfg(unix)]
fn disable_core_dumps() {
    use nix::sys::resource::{setrlimit, Resource};
    // コアダンプのファイルサイズ制限を0に設定
    if let Err(e) = setrlimit(Resource::RLIMIT_CORE, 0, 0) {
        eprintln!("Failed to disable core dumps: {}", e);
    }
}

#[cfg(not(unix))]
fn disable_core_dumps() {
    // Non-Unix OS: No-op for now
}

#[cfg(target_os = "macos")]
#[allow(unsafe_code)]
fn disable_ptrace() {
    // macOS では nix::libc の PT_DENY_ATTACH (31) を使用する
    const PT_DENY_ATTACH: i32 = 31;
    unsafe {
        nix::libc::ptrace(PT_DENY_ATTACH, 0, std::ptr::null_mut(), 0);
    }
}

#[cfg(target_os = "linux")]
#[allow(unsafe_code)]
fn disable_ptrace() {
    // Linux では prctl を使って dumpable を無効にする
    unsafe {
        nix::libc::prctl(nix::libc::PR_SET_DUMPABLE, 0, 0, 0, 0);
    }
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
fn disable_ptrace() {
    // Other OS: No-op for now
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    #[allow(unsafe_code)]
    fn test_scrub_env_prefix() {
        // Arrange
        unsafe {
            env::set_var("TEST_PREFIX_1", "value1");
            env::set_var("TEST_PREFIX_2", "value2");
            env::set_var("TEST_OTHER_VAR", "value3");
        }

        // Act
        scrub_env_prefix("TEST_PREFIX_");

        // Assert
        assert!(
            env::var("TEST_PREFIX_1").is_err(),
            "TEST_PREFIX_1 should be removed"
        );
        assert!(
            env::var("TEST_PREFIX_2").is_err(),
            "TEST_PREFIX_2 should be removed"
        );
        assert_eq!(
            env::var("TEST_OTHER_VAR").unwrap(),
            "value3",
            "TEST_OTHER_VAR should remain"
        );

        // Cleanup
        unsafe {
            env::remove_var("TEST_OTHER_VAR");
        }
    }

    #[test]
    fn test_pre_main_hardening_does_not_panic() {
        // Just checking it runs without panicking
        pre_main_hardening();
    }
}

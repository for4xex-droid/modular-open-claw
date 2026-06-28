/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
fn main() {
    #[cfg(target_os = "macos")]
    let _ = nix::sys::ptrace::deny_attach();
}

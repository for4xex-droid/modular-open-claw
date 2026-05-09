fn main() {
    #[cfg(target_os = "macos")]
    let _ = nix::sys::ptrace::deny_attach();
}

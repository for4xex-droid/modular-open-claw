use sysinfo::{System, Pid};
use serde::{Deserialize, Serialize};
use std::fmt;

/// 秘密情報をログ出力から保護するためのラッパー
#[derive(Clone, Deserialize, Serialize)]
pub struct Secret<T>(T);

impl<T> Secret<T> {
    pub fn new(val: T) -> Self {
        Self(val)
    }

    pub fn expose(&self) -> &T {
        &self.0
    }
}

// 誤ってログに出力されないようにマスクする
impl<T> fmt::Debug for Secret<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "********")
    }
}

impl<T> fmt::Display for Secret<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "********")
    }
}

/// リソースの使用状況
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceStatus {
    pub memory_usage_mb: u64,
    pub cpu_usage_percent: f32,
    pub open_files: Option<u64>,
}

/// システムの状態を監視する
pub struct HealthMonitor {
    sys: System,
    pid: Pid,
}

impl HealthMonitor {
    pub fn new() -> Self {
        let mut sys = System::new_all();
        sys.refresh_all();
        // std::process::id() returns u32, sysinfo::Pid is platform dependent but often u32 or i32
        let pid = Pid::from(std::process::id() as usize);
        Self { sys, pid }
    }

    pub fn check(&mut self) -> ResourceStatus {
        // 特定のプロセスのみリフレッシュ
        self.sys.refresh_process(self.pid);
        
        let mut memory_usage_mb = 0;
        let mut cpu_usage_percent = 0.0;
        
        if let Some(process) = self.sys.process(self.pid) {
            // sysinfo 0.30 では bytes 単位
            memory_usage_mb = process.memory() / 1024 / 1024;
            cpu_usage_percent = process.cpu_usage();
        }

        ResourceStatus {
            memory_usage_mb,
            cpu_usage_percent,
            open_files: None,
        }
    }
}

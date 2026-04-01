#[cfg(windows)]
use std::process::Command;

// 锁屏监测器。
// 当前版本是轻量轮询实现（通过 LogonUI 进程判断）。
pub struct LockMonitor;

impl LockMonitor {
    pub fn new() -> Self {
        Self
    }

    pub fn is_session_locked(&self) -> bool {
        #[cfg(windows)]
        {
            // 用 LogonUI 进程作为锁屏状态的轻量判断依据：
            // 常见 Windows 10/11 桌面环境中，锁屏界面出现时 LogonUI 会存在。
            let output = Command::new("powershell")
                .args([
                    "-NoProfile",
                    "-NonInteractive",
                    "-Command",
                    "(Get-Process -Name LogonUI -ErrorAction SilentlyContinue) -ne $null",
                ])
                .output();

            if let Ok(out) = output {
                let text = String::from_utf8_lossy(&out.stdout);
                return text.trim().eq_ignore_ascii_case("True");
            }
            false
        }

        #[cfg(not(windows))]
        {
            false
        }
    }
}

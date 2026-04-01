#[cfg(windows)]
use std::os::windows::process::CommandExt;
#[cfg(windows)]
use std::process::Command;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

pub struct LockMonitor;

impl LockMonitor {
    pub fn new() -> Self {
        Self
    }

    pub fn is_session_locked(&self) -> bool {
        #[cfg(windows)]
        {
            let output = Command::new("powershell")
                .args([
                    "-NoProfile",
                    "-NonInteractive",
                    "-Command",
                    "(Get-Process -Name LogonUI -ErrorAction SilentlyContinue) -ne $null",
                ])
                .creation_flags(CREATE_NO_WINDOW)
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

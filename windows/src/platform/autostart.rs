use anyhow::Result;
#[cfg(windows)]
use std::env;
#[cfg(windows)]
use std::process::{Command, Stdio};

// 管理开机自启动。
// 当前实现基于 Windows 注册表：
// HKCU\Software\Microsoft\Windows\CurrentVersion\Run
pub struct AutoStartManager;

impl AutoStartManager {
    pub fn new() -> Self {
        Self
    }

    pub fn sync_from_config(&self, enabled: bool) -> Result<()> {
        #[cfg(windows)]
        {
            // 启动项命令："<exe-path>" run
            // 这样系统启动后直接进入 UI 模式。
            let exe = env::current_exe()?;
            let exe_str = exe.to_string_lossy().to_string();
            let run_value = format!("\"{exe_str}\" run");

            if enabled {
                // 写入/覆盖启动项。
                let status = Command::new("reg")
                    .args([
                        "add",
                        r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run",
                        "/v",
                        "RhythmWin",
                        "/t",
                        "REG_SZ",
                        "/d",
                        &run_value,
                        "/f",
                    ])
                    .stdin(Stdio::null())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status()?;
                if !status.success() {
                    anyhow::bail!("写入注册表启动项失败");
                }
                println!("[autostart] 已启用");
            } else {
                // 先查后删，避免删除不存在值时产生无意义报错。
                let has_value = Command::new("reg")
                    .args([
                        "query",
                        r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run",
                        "/v",
                        "RhythmWin",
                    ])
                    .stdin(Stdio::null())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status()?
                    .success();
                if !has_value {
                    return Ok(());
                }

                let status = Command::new("reg")
                    .args([
                        "delete",
                        r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run",
                        "/v",
                        "RhythmWin",
                        "/f",
                    ])
                    .stdin(Stdio::null())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status()?;
                if !status.success() {
                    eprintln!("[autostart] 启动项可能不存在，跳过删除");
                } else {
                    println!("[autostart] 已禁用");
                }
            }
        }

        #[cfg(not(windows))]
        {
            let _ = enabled;
            println!("[autostart] 非 Windows 平台，跳过");
        }
        Ok(())
    }

    pub fn is_enabled(&self) -> Result<bool> {
        #[cfg(windows)]
        {
            // 通过查询注册表判断当前是否启用开机启动。
            let exists = Command::new("reg")
                .args([
                    "query",
                    r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run",
                    "/v",
                    "RhythmWin",
                ])
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()?
                .success();
            Ok(exists)
        }

        #[cfg(not(windows))]
        {
            Ok(false)
        }
    }
}

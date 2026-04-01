use anyhow::Result;
#[cfg(windows)]
use std::os::windows::process::CommandExt;
#[cfg(windows)]
use std::process::{Child, Command, Stdio};
#[cfg(windows)]
use std::time::Instant;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

// 休息遮罩管理器：
// - show_rest_overlay: 弹出全屏遮罩
// - hide_rest_overlay: 主动关闭遮罩
// - poll_event: 轮询遮罩是否结束，以及结束原因（完成/跳过）
pub struct OverlayManager {
    #[cfg(windows)]
    state: Option<OverlayState>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverlayEvent {
    // 到时自动关闭（视为正常完成休息）
    Completed,
    // 用户提前关闭（例如 ESC）
    Skipped,
}

#[cfg(windows)]
struct OverlayState {
    // 子进程（PowerShell + WinForms）句柄
    child: Child,
    // 启动时间，用于区分“自然结束”还是“提前关闭”
    started_at: Instant,
    // 原计划休息时长（秒）
    scheduled_seconds: u64,
}

impl OverlayManager {
    pub fn new() -> Self {
        Self {
            #[cfg(windows)]
            state: None,
        }
    }

    pub fn show_rest_overlay(&mut self, seconds: u64) -> Result<()> {
        #[cfg(windows)]
        {
            self.hide_rest_overlay()?;

            // 用 PowerShell + WinForms 提供可用的全屏倒计时遮罩。
            // 这里把秒数直接写入脚本字符串，避免命令行参数传递导致的兼容问题。
            let safe_seconds = seconds.max(1);
            let script = r#"
& {
  Add-Type -AssemblyName System.Windows.Forms
  Add-Type -AssemblyName System.Drawing
  $form = New-Object System.Windows.Forms.Form
  $form.FormBorderStyle = [System.Windows.Forms.FormBorderStyle]::None
  $form.WindowState = [System.Windows.Forms.FormWindowState]::Maximized
  $form.TopMost = $true
  $form.BackColor = [System.Drawing.Color]::Black
  $form.Opacity = 0.85
  $form.KeyPreview = $true

  $label = New-Object System.Windows.Forms.Label
  $label.Dock = [System.Windows.Forms.DockStyle]::Fill
  $label.ForeColor = [System.Drawing.Color]::White
  $label.Font = New-Object System.Drawing.Font('Segoe UI', 48, [System.Drawing.FontStyle]::Bold)
  $label.TextAlign = [System.Drawing.ContentAlignment]::MiddleCenter
  $form.Controls.Add($label)

  $script:remaining = [Math]::Max(1, __SECONDS__)
  $label.Text = "休息中: $script:remaining 秒`n按 ESC 跳过"

  $timer = New-Object System.Windows.Forms.Timer
  $timer.Interval = 1000
  $timer.Add_Tick({
    $script:remaining--
    if ($script:remaining -le 0) {
      $timer.Stop()
      $form.Close()
    } else {
      $label.Text = "休息中: $script:remaining 秒`n按 ESC 跳过"
    }
  })

  $form.Add_KeyDown({
    param($sender, $e)
    if ($e.KeyCode -eq [System.Windows.Forms.Keys]::Escape) {
      $timer.Stop()
      $form.Close()
    }
  })
  $form.Add_Shown({ $timer.Start() })
  [void]$form.ShowDialog()
}
"#
            .replace("__SECONDS__", &safe_seconds.to_string());

            let child = Command::new("powershell")
                .args([
                    "-NoProfile",
                    "-ExecutionPolicy",
                    "Bypass",
                    "-WindowStyle",
                    "Hidden",
                    "-Command",
                    &script,
                ])
                .creation_flags(CREATE_NO_WINDOW)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()?;
            self.state = Some(OverlayState {
                child,
                started_at: Instant::now(),
                scheduled_seconds: seconds.max(1),
            });
        }

        #[cfg(not(windows))]
        {
            println!("[overlay] non-windows overlay ({seconds}s)");
        }
        Ok(())
    }

    pub fn hide_rest_overlay(&mut self) -> Result<()> {
        #[cfg(windows)]
        {
            if let Some(mut state) = self.state.take() {
                let _ = state.child.kill();
                let _ = state.child.wait();
            }
        }

        #[cfg(not(windows))]
        {
            println!("[overlay] hide rest overlay");
        }
        Ok(())
    }

    pub fn poll_event(&mut self) -> Result<Option<OverlayEvent>> {
        #[cfg(windows)]
        {
            // 子进程退出后，根据实际耗时判断：
            // - 接近计划时长 => Completed
            // - 明显提前退出 => Skipped
            if let Some(state) = self.state.as_mut() {
                if state.child.try_wait()?.is_some() {
                    let elapsed = state.started_at.elapsed().as_secs();
                    let scheduled = state.scheduled_seconds;
                    self.state = None;
                    if elapsed + 1 >= scheduled {
                        return Ok(Some(OverlayEvent::Completed));
                    }
                    return Ok(Some(OverlayEvent::Skipped));
                }
            }
            Ok(None)
        }

        #[cfg(not(windows))]
        {
            Ok(None)
        }
    }
}

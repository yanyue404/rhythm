use anyhow::Result;
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

// 单条休息记录。
// skipped = true 表示这次休息是被跳过（比如 ESC 或不休息模式）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestSession {
    pub id: String,
    pub scheduled_rest_seconds: u64,
    pub actual_rest_seconds: u64,
    pub started_at_epoch: i64,
    pub ended_at_epoch: i64,
    pub skipped: bool,
    pub skip_reason: Option<String>,
    pub created_at_epoch: i64,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct SessionStore {
    // 这里按时间顺序存储（旧 -> 新），读取 recent 时再倒序。
    sessions: Vec<RestSession>,
}

impl SessionStore {
    // 从本地 JSON 加载会话记录；没有文件就返回空仓库。
    pub fn load_or_default() -> Result<Self> {
        let path = sessions_path()?;
        if !path.exists() {
            return Ok(Self::default());
        }
        let raw = fs::read_to_string(path)?;
        let store = serde_json::from_str(&raw)?;
        Ok(store)
    }

    // 将当前会话列表保存到磁盘。
    pub fn save(&self) -> Result<()> {
        let path = sessions_path()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, serde_json::to_string_pretty(self)?)?;
        Ok(())
    }

    // 追加一条“正常完成休息”的记录。
    pub fn push_completed(
        &mut self,
        scheduled_rest_seconds: u64,
        actual_rest_seconds: u64,
        started_at_epoch: i64,
        ended_at_epoch: i64,
    ) {
        self.sessions.push(RestSession {
            id: format!("session-{started_at_epoch}-{ended_at_epoch}"),
            scheduled_rest_seconds,
            actual_rest_seconds,
            started_at_epoch,
            ended_at_epoch,
            skipped: false,
            skip_reason: None,
            created_at_epoch: ended_at_epoch,
        });
        self.trim_history();
    }

    // 追加一条“跳过休息”的记录，reason 用于区分 skip-mode / esc。
    pub fn push_skipped(
        &mut self,
        scheduled_rest_seconds: u64,
        started_at_epoch: i64,
        ended_at_epoch: i64,
        reason: &str,
    ) {
        self.sessions.push(RestSession {
            id: format!("session-{started_at_epoch}-{ended_at_epoch}"),
            scheduled_rest_seconds,
            actual_rest_seconds: 0,
            started_at_epoch,
            ended_at_epoch,
            skipped: true,
            skip_reason: Some(reason.to_string()),
            created_at_epoch: ended_at_epoch,
        });
        self.trim_history();
    }

    // 按“最近优先”返回记录，供 UI 展示最近 n 条。
    pub fn recent(&self, limit: usize) -> Vec<&RestSession> {
        self.sessions.iter().rev().take(limit).collect()
    }

    // 简单历史裁剪，防止文件无限增长。
    fn trim_history(&mut self) {
        const MAX_SESSIONS: usize = 1000;
        if self.sessions.len() <= MAX_SESSIONS {
            return;
        }
        let drop_count = self.sessions.len() - MAX_SESSIONS;
        self.sessions.drain(0..drop_count);
    }
}

// 会话文件路径：
// Windows 下通常在 %APPDATA%\com\yanyue404\rhythm-win\data\sessions.json
fn sessions_path() -> Result<PathBuf> {
    let dirs = ProjectDirs::from("com", "yanyue404", "rhythm-win")
        .ok_or_else(|| anyhow::anyhow!("无法解析应用数据目录"))?;
    Ok(dirs.data_dir().join("sessions.json"))
}

#[cfg(test)]
mod tests {
    use super::SessionStore;

    #[test]
    fn recent_should_return_latest_first() {
        let mut store = SessionStore::default();
        store.push_completed(60, 60, 1, 2);
        store.push_completed(90, 80, 3, 4);
        let recent = store.recent(1);
        assert_eq!(recent.len(), 1);
        assert_eq!(recent[0].scheduled_rest_seconds, 90);
    }
}

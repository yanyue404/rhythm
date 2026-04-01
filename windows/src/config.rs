use anyhow::Result;
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

// 用户可配置项。
// 这些字段直接影响状态机行为与 UI 开关状态。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub focus_minutes: u64,
    pub rest_seconds: u64,
    pub skip_rest_enabled: bool,
    pub launch_at_login: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            focus_minutes: 25,
            rest_seconds: 300,
            skip_rest_enabled: false,
            launch_at_login: false,
        }
    }
}

impl AppConfig {
    // 统一做范围约束，防止配置文件被手改成非法值。
    pub fn normalize(&mut self) {
        self.focus_minutes = self.focus_minutes.clamp(10, 120);
        self.rest_seconds = self.rest_seconds.clamp(30, 600);
    }

    // 从磁盘读取配置，若不存在则使用默认值。
    // 同时兼容历史配置结构（通过 from_legacy 迁移）。
    pub fn load_or_default() -> Result<Self> {
        let path = config_path()?;
        if !path.exists() {
            return Ok(Self::default());
        }

        let raw = fs::read_to_string(path)?;
        let mut cfg = if let Ok(v) = serde_json::from_str::<Self>(&raw) {
            v
        } else {
            let legacy: LegacyConfig = serde_json::from_str(&raw)?;
            Self::from_legacy(legacy)
        };
        cfg.normalize();
        Ok(cfg)
    }

    // 持久化配置到用户目录。
    pub fn save(&self) -> Result<()> {
        let path = config_path()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, serde_json::to_string_pretty(self)?)?;
        Ok(())
    }
}

// 历史版本配置结构（用于迁移）。
// 例如旧版本使用 rest_minutes，新版本使用 rest_seconds。
#[derive(Debug, Clone, Deserialize)]
struct LegacyConfig {
    focus_minutes: Option<u64>,
    rest_seconds: Option<u64>,
    rest_minutes: Option<u64>,
    skip_rest_enabled: Option<bool>,
    launch_at_login: Option<bool>,
}

impl AppConfig {
    // 将历史配置结构映射为当前配置结构。
    fn from_legacy(old: LegacyConfig) -> Self {
        let mut cfg = Self::default();
        if let Some(v) = old.focus_minutes {
            cfg.focus_minutes = v;
        }
        if let Some(v) = old.rest_seconds {
            cfg.rest_seconds = v;
        } else if let Some(v) = old.rest_minutes {
            cfg.rest_seconds = v.saturating_mul(60);
        }
        if let Some(v) = old.skip_rest_enabled {
            cfg.skip_rest_enabled = v;
        }
        if let Some(v) = old.launch_at_login {
            cfg.launch_at_login = v;
        }
        cfg
    }
}

// 计算配置文件路径：
// Windows 下通常在 %APPDATA%\com\yanyue404\rhythm-win\config.json
fn config_path() -> Result<PathBuf> {
    let dirs = ProjectDirs::from("com", "yanyue404", "rhythm-win")
        .ok_or_else(|| anyhow::anyhow!("无法解析应用数据目录"))?;
    Ok(dirs.config_dir().join("config.json"))
}

#[cfg(test)]
mod tests {
    use super::AppConfig;

    #[test]
    fn normalize_should_clamp_range() {
        let mut c = AppConfig {
            focus_minutes: 1,
            rest_seconds: 9999,
            skip_rest_enabled: false,
            launch_at_login: false,
        };
        c.normalize();
        assert_eq!(c.focus_minutes, 10);
        assert_eq!(c.rest_seconds, 600);
    }
}

mod config;
mod platform;
mod session;
mod timer_engine;
mod ui;

use anyhow::Result;
use clap::{Parser, Subcommand};
use config::AppConfig;
use platform::{autostart::AutoStartManager, lock_monitor::LockMonitor, overlay::OverlayManager};
use session::SessionStore;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use timer_engine::{EngineTransition, TimerEngine};

// 程序主入口：
// 1) 解析命令行
// 2) 分发到 UI / 后台模式 / 配置管理 / 会话查看
fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command.unwrap_or(Commands::Run) {
        Commands::Run => ui::run_ui(),
        Commands::Daemon => run_daemon(),
        Commands::Config { action } => run_config(action),
        Commands::Sessions { action } => run_sessions(action),
    }
}

#[derive(Debug, Parser)]
#[command(name = "rhythm-win")]
#[command(about = "Windows Rust 节奏提醒工具", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// 启动可视化面板
    Run,
    /// 启动无界面计时循环
    Daemon,
    /// 查看或修改配置
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
    /// 查看休息记录
    Sessions {
        #[command(subcommand)]
        action: SessionAction,
    },
}

#[derive(Debug, Subcommand)]
enum ConfigAction {
    /// 打印当前配置
    Show,
    /// 更新配置项（仅设置你传入的字段）
    Set {
        #[arg(long)]
        focus_minutes: Option<u64>,
        #[arg(long)]
        rest_seconds: Option<u64>,
        #[arg(long)]
        skip_rest_enabled: Option<bool>,
        #[arg(long)]
        launch_at_login: Option<bool>,
    },
}

#[derive(Debug, Subcommand)]
enum SessionAction {
    /// 列出最近休息记录
    List {
        #[arg(long, default_value_t = 10)]
        limit: usize,
    },
}

// 处理 `config` 子命令。
// 这里会优先读取系统真实的开机启动状态，避免配置文件与注册表状态不一致。
fn run_config(action: ConfigAction) -> Result<()> {
    let mut config = AppConfig::load_or_default()?;
    let autostart = AutoStartManager::new();
    if let Ok(enabled) = autostart.is_enabled() {
        config.launch_at_login = enabled;
    }
    match action {
        ConfigAction::Show => {
            println!("{}", serde_json::to_string_pretty(&config)?);
        }
        ConfigAction::Set {
            focus_minutes,
            rest_seconds,
            skip_rest_enabled,
            launch_at_login,
        } => {
            if let Some(v) = focus_minutes {
                config.focus_minutes = v;
            }
            if let Some(v) = rest_seconds {
                config.rest_seconds = v;
            }
            if let Some(v) = skip_rest_enabled {
                config.skip_rest_enabled = v;
            }
            if let Some(v) = launch_at_login {
                config.launch_at_login = v;
            }
            config.normalize();
            config.save()?;

            if let Err(err) = autostart.sync_from_config(config.launch_at_login) {
                eprintln!("同步开机启动状态失败: {err}");
            }

            println!("配置已更新:");
            println!("{}", serde_json::to_string_pretty(&config)?);
        }
    }
    Ok(())
}

// 处理 `sessions` 子命令，用于在终端查看最近休息记录。
fn run_sessions(action: SessionAction) -> Result<()> {
    let store = SessionStore::load_or_default()?;
    match action {
        SessionAction::List { limit } => {
            for item in store.recent(limit) {
                println!(
                    "[{}] scheduled={}s actual={}s skipped={} reason={}",
                    item.id,
                    item.scheduled_rest_seconds,
                    item.actual_rest_seconds,
                    item.skipped,
                    item.skip_reason.as_deref().unwrap_or("-")
                );
            }
        }
    }
    Ok(())
}

// 无界面后台循环（daemon 模式）：
// - 负责推进计时状态机
// - 负责锁屏重置
// - 负责休息遮罩与会话落盘
fn run_daemon() -> Result<()> {
    let mut config = AppConfig::load_or_default()?;
    config.normalize();
    config.save()?;

    let mut session_store = SessionStore::load_or_default()?;
    let lock_monitor = LockMonitor::new();
    let mut overlay = OverlayManager::new();
    let autostart = AutoStartManager::new();

    println!("Rhythm Windows (Rust) 已启动");
    println!(
        "当前配置: 专注 {} 分钟, 休息 {} 秒, 不休息模式 {}",
        config.focus_minutes, config.rest_seconds, config.skip_rest_enabled
    );

    if let Err(err) = autostart.sync_from_config(config.launch_at_login) {
        eprintln!("同步开机启动状态失败: {err}");
    }

    let mut engine = TimerEngine::new(config.focus_minutes * 60, config.rest_seconds);

    let mut last_locked_state = false;

    loop {
        // 锁屏后重置当前周期，保证行为和设计文档一致。
        let locked = lock_monitor.is_session_locked();
        if locked && !last_locked_state {
            engine.reset_focus();
            println!("检测到系统锁屏，已重置到新一轮专注周期。");
        }
        last_locked_state = locked;

        // 每秒推进状态机一次。
        // tick 可能只是继续计时，也可能触发阶段切换。
        match engine.tick(config.skip_rest_enabled) {
            EngineTransition::Continue => {}
            EngineTransition::FocusToRest { scheduled_rest_seconds, started_at_epoch } => {
                overlay.show_rest_overlay(scheduled_rest_seconds)?;
                println!("进入休息阶段: {} 秒", scheduled_rest_seconds);

                if config.skip_rest_enabled {
                    let now_epoch = now_epoch_seconds();
                    session_store.push_skipped(
                        scheduled_rest_seconds,
                        started_at_epoch,
                        now_epoch,
                        "skip-mode",
                    );
                    session_store.save()?;
                    println!("不休息模式已开启，本次休息自动跳过。");
                    engine.force_back_to_focus();
                }
            }
            EngineTransition::RestCompleted {
                scheduled_rest_seconds,
                actual_rest_seconds,
                started_at_epoch,
                ended_at_epoch,
            } => {
                session_store.push_completed(
                    scheduled_rest_seconds,
                    actual_rest_seconds,
                    started_at_epoch,
                    ended_at_epoch,
                );
                session_store.save()?;
                overlay.hide_rest_overlay()?;
                println!("休息结束，回到专注阶段。");
            }
        }

        // 轮询遮罩进程事件：
        // 如果用户按 ESC 关闭遮罩，则记录一次 skipped 会话。
        if let Some(event) = overlay.poll_event()? {
            if matches!(event, platform::overlay::OverlayEvent::Skipped)
                && matches!(engine.phase(), timer_engine::Phase::Resting)
            {
                let ended_at = now_epoch_seconds();
                let started_at = ended_at - config.rest_seconds as i64;
                session_store.push_skipped(config.rest_seconds, started_at, ended_at, "esc");
                session_store.save()?;
                engine.force_back_to_focus();
                println!("用户按 ESC 跳过休息。");
            }
        }

        thread::sleep(Duration::from_secs(1));
    }
}

// 统一获取 Unix 时间戳秒，用于会话记录落盘。
fn now_epoch_seconds() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

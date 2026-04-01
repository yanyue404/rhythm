# Rhythm Windows (Rust)

这是基于 `xiao2dou/rhythm` 思路改造的 Windows 版本，技术栈按 `doc.md` 选择 Rust 原生方向。

## 当前状态

- 已实现：配置管理（含历史字段迁移）、专注/休息状态机、会话记录持久化
- 已实现：Windows 开机启动注册表同步（HKCU Run）
- 已实现：Windows 锁屏轮询检测（LogonUI 进程判断）
- 已实现：休息阶段全屏遮罩（PowerShell + WinForms，支持 ESC 关闭）
- 提供 CLI 子命令：运行、配置、会话查询

## 技术栈

- Rust 2021
- `windows-rs`（预留 Win32 深度接入）
- `serde + json`（配置与会话）
- `clap`（CLI）

## 本地运行（Windows）

```bash
cargo run
```

默认会打开可视化面板（等价于 `cargo run -- run`）。

首次运行后会在本地生成：

- 配置：`%APPDATA%\com\yanyue404\rhythm-win\config\config.json`
- 会话：`%APPDATA%\com\yanyue404\rhythm-win\data\sessions.json`

## CLI 使用

查看配置：

```bash
cargo run -- config show
```

更新配置：

```bash
cargo run -- config set --focus-minutes 45 --rest-seconds 180 --skip-rest-enabled false --launch-at-login true
```

查看最近 20 条会话：

```bash
cargo run -- sessions list --limit 20
```

启动无界面循环（用于后台/调试）：

```bash
cargo run -- daemon
```

## 后续增强建议

1. 用纯 Win32/Rust 重写遮罩（替换 PowerShell WinForms）
2. 用 `WTSRegisterSessionNotification` 改造锁屏检测为事件驱动
3. 接入托盘菜单（开始/暂停、配置、不休息开关、退出）
4. 增加单元测试和集成测试（配置迁移、计时流转、会话持久化）

## 打包发布（Windows）

### 生成 Release

```bash
cargo build --release
```

可执行文件在：

- `target/release/rhythm-win.exe`

### 一键打包 ZIP

在 `windows/` 目录执行：

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\package_win.ps1
```

可选传版本号：

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\package_win.ps1 -Version 0.1.0
```

输出：

- `dist/RhythmWin-vX.Y.Z-win64/`
- `dist/RhythmWin-vX.Y.Z-win64.zip`

### 生成安装包（Inno Setup）

1. 先执行 `cargo build --release`
2. 用 Inno Setup 打开 `installer/rhythm-win.iss`
3. 点击 Compile

输出安装包：

- `dist/RhythmWin-Setup-vX.Y.Z.exe`

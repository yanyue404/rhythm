# Windows 开发指南（Rust）

这份文档给“第一次接手本项目”的开发者用，目标是：

- 快速跑起来
- 知道每个模块改哪里
- 改动后能自检，不容易踩坑

---

## 1. 项目结构速览

```txt
windows/
├─ src/
│  ├─ main.rs              # 程序入口、CLI 分发、daemon 主循环
│  ├─ ui.rs                # 固定弹窗 UI（按设计稿分区绘制）
│  ├─ timer_engine.rs      # 核心状态机（focusing/resting）
│  ├─ config.rs            # 配置读写与迁移
│  ├─ session.rs           # 会话记录读写与裁剪
│  └─ platform/
│     ├─ mod.rs
│     ├─ autostart.rs      # 开机启动（注册表 Run）
│     ├─ lock_monitor.rs   # 锁屏检测
│     └─ overlay.rs        # 全屏遮罩与 ESC 跳过事件
├─ installer/
│  └─ rhythm-win.iss       # Inno Setup 安装脚本
├─ scripts/
│  └─ package_win.ps1      # ZIP 一键打包脚本
├─ Cargo.toml
└─ README.md
```

---

## 2. 本地开发最小流程

在 `windows/` 目录执行：

```bash
cargo run
```

常用命令：

- `cargo run`：打开可视化弹窗
- `cargo run -- daemon`：无界面后台循环
- `cargo run -- config show`：查看配置
- `cargo run -- sessions list --limit 20`：查看最近记录
- `cargo check`：快速编译检查

---

## 3. 程序运行主链路（建议按这个顺序读代码）

1. `main.rs`
   - 看 `main()`：CLI 怎么分发到 UI/daemon
   - 看 `run_daemon()`：后台模式怎么推进状态机
2. `timer_engine.rs`
   - 看 `Phase` 和 `EngineTransition`
   - 看 `tick()`：每秒推进后的状态变化
3. `ui.rs`
   - 看 `update()`：每帧做什么（轮询遮罩、锁屏、tick、绘制）
   - 看 `draw_popup()`：固定分区布局坐标
4. `platform/overlay.rs`
   - 看 `show_rest_overlay()` 和 `poll_event()`：休息遮罩与 ESC 跳过
5. `config.rs` / `session.rs`
   - 看配置和会话怎么落盘

---

## 4. 常见需求该改哪里

### 4.1 改 UI 位置/尺寸/间距

改 `src/ui.rs`：

- `draw_popup()`：卡片坐标与尺寸（最关键）
- `draw_header / draw_timer / draw_settings / draw_records / draw_footer`：各区内部排版
- `BASE_W / BASE_H / BASE_PANEL_W`：设计基准尺寸

### 4.2 改节奏规则（专注/休息切换）

改 `src/timer_engine.rs`：

- `tick()`：自动切换逻辑
- `start_rest_now()`：立即休息按钮逻辑
- `update_schedule()`：改配置后是否立即生效

### 4.3 改配置项

改 `src/config.rs`：

- `AppConfig` 新增字段
- `normalize()` 增加范围约束
- `from_legacy()` 增加兼容迁移

然后同步：

- `src/ui.rs` 的设置区展示与交互
- `src/main.rs` 的 CLI `config set`

### 4.4 改会话记录展示或字段

改 `src/session.rs`（数据结构/落盘）+ `src/ui.rs`（展示样式）。

### 4.5 改平台能力

- 开机启动：`src/platform/autostart.rs`
- 锁屏检测：`src/platform/lock_monitor.rs`
- 遮罩行为：`src/platform/overlay.rs`

---

## 5. 关键设计约束（重要）

1. **固定弹窗 + 比例缩放**
   - 当前 UI 不走“自由流式布局”，而是固定分区坐标，避免跨电脑漂移
2. **状态机与 UI 解耦**
   - 业务规则在 `timer_engine.rs`
   - UI 只消费 `EngineTransition`
3. **ESC 跳过必须落盘**
   - `overlay.poll_event()` 返回 `Skipped` 时，必须写 session
4. **配置修改立即生效**
   - 改完配置后必须调用 `apply_config()`，同步到状态机

---

## 6. 自检清单（每次改完都跑）

1. `cargo check` 通过
2. 点击 `立即休息`，遮罩正常倒计时
3. 按 `ESC`，最近记录出现“跳过”
4. 修改专注/休息时长后立即生效
5. 切换“不休息”后，到点自动跳过并记录
6. 切换“开机启动”后，注册表状态一致

---

## 7. 打包发布

在 `windows/`：

```bash
cargo build --release
```

仓库根目录：

```powershell
powershell -ExecutionPolicy Bypass -File .\windows\scripts\package_win.ps1
```

输出在 `dist/`。

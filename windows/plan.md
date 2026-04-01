## 推荐技术栈（按“体积小”优先级）

### 方案 A（最推荐）：Rust + Windows 原生 API（无 WebView）

**技术栈建议：**

- 语言：`Rust`
- UI/窗口：`windows-rs`（直接调 Win32）或 `tao/winit + tray-icon`
- 托盘：`tray-icon`
- 配置/数据：`serde + toml/json`（轻量）或 `rusqlite`（需要历史查询时）
- 构建：`cargo build --release` + `lto = true` + `strip`（显著减小体积）
- 安装：`WiX` 或 `NSIS`（可选）

**优点：**

- 体积小（通常明显小于 Electron / Flutter）
- 启动快、内存占用低
- 原生能力全（托盘、锁屏事件、顶层遮罩、自启动都好做）

**缺点：**

- 开发复杂度高于 .NET / Tauri Web 前端

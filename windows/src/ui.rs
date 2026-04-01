use crate::config::AppConfig;
use crate::platform::{
    autostart::AutoStartManager,
    lock_monitor::LockMonitor,
    overlay::{OverlayEvent, OverlayManager},
};
use crate::session::{RestSession, SessionStore};
use crate::timer_engine::{EngineTransition, Phase, TimerEngine};
use anyhow::Result;
use chrono::{Local, TimeZone};
use eframe::egui;
use egui::{Color32, FontData, FontFamily, FontId, Rect, RichText, Stroke, UiBuilder, Vec2, pos2, vec2};
use std::fs;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

// 下面三组常量是“设计稿基准尺寸”：
// 实际窗口会按屏幕缩放（scale），但布局坐标都基于这套基准值计算。
const BASE_W: f32 = 726.0;
const BASE_H: f32 = 1040.0;
const BASE_PANEL_W: f32 = 680.0;

#[cfg(windows)]
fn popup_size_from_screen() -> Vec2 {
    use windows::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN};
    let sw = unsafe { GetSystemMetrics(SM_CXSCREEN) as f32 };
    let sh = unsafe { GetSystemMetrics(SM_CYSCREEN) as f32 };
    let h = (sh * 0.60).clamp(560.0, 900.0);
    let mut w = h * 0.68;
    w = w.min(sw * 0.55).clamp(420.0, 620.0);
    vec2(w.round(), h.round())
}

#[cfg(not(windows))]
fn popup_size_from_screen() -> Vec2 {
    vec2(520.0, 760.0)
}

// UI 模式入口。
// 这里会先算出“屏幕自适应弹窗尺寸”，再启动 eframe 窗口。
pub fn run_ui() -> Result<()> {
    let popup = popup_size_from_screen();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([popup.x, popup.y])
            .with_min_inner_size([popup.x, popup.y])
            .with_max_inner_size([popup.x, popup.y])
            .with_resizable(false)
            .with_title("Rhythm"),
        ..Default::default()
    };

    eframe::run_native(
        "Rhythm",
        options,
        Box::new(|cc| {
            configure_egui(&cc.egui_ctx);
            let app = RhythmGuiApp::new(popup)
                .unwrap_or_else(|e| RhythmGuiApp::fallback(format!("初始化失败: {e}")));
            Ok(Box::new(app))
        }),
    )
    .map_err(|e| anyhow::anyhow!("启动 UI 失败: {e}"))
}

struct PendingRest {
    // 进入休息时的起点时间
    started_at_epoch: i64,
    // 计划休息时长（秒）
    scheduled_rest_seconds: u64,
}

struct RhythmGuiApp {
    config: AppConfig,
    sessions: SessionStore,
    engine: TimerEngine,
    lock_monitor: LockMonitor,
    overlay: OverlayManager,
    autostart: AutoStartManager,
    pending_rest: Option<PendingRest>,
    status: String,
    scale: f32,
    last_tick: Instant,
    last_lock_poll: Instant,
    last_lock_state: bool,
}

impl RhythmGuiApp {
    fn fallback(status: String) -> Self {
        let config = AppConfig::default();
        let popup = popup_size_from_screen();
        let scale = (popup.x / BASE_W).min(popup.y / BASE_H);
        Self {
            engine: TimerEngine::new(config.focus_minutes * 60, config.rest_seconds),
            sessions: SessionStore::default(),
            lock_monitor: LockMonitor::new(),
            overlay: OverlayManager::new(),
            autostart: AutoStartManager::new(),
            config,
            pending_rest: None,
            status,
            scale,
            last_tick: Instant::now(),
            last_lock_poll: Instant::now(),
            last_lock_state: false,
        }
    }

    fn new(popup_size: Vec2) -> Result<Self> {
        let mut config = AppConfig::load_or_default()?;
        config.normalize();
        let autostart = AutoStartManager::new();
        if let Ok(enabled) = autostart.is_enabled() {
            config.launch_at_login = enabled;
        }
        config.save()?;

        let scale = (popup_size.x / BASE_W).min(popup_size.y / BASE_H);
        Ok(Self {
            engine: TimerEngine::new(config.focus_minutes * 60, config.rest_seconds),
            sessions: SessionStore::load_or_default()?,
            lock_monitor: LockMonitor::new(),
            overlay: OverlayManager::new(),
            autostart,
            config,
            pending_rest: None,
            status: "专注中".to_string(),
            scale,
            last_tick: Instant::now(),
            last_lock_poll: Instant::now(),
            last_lock_state: false,
        })
    }

    fn apply_config(&mut self) {
        // 修改配置后立即作用到状态机，并持久化到磁盘。
        self.config.normalize();
        self.engine
            .update_schedule(self.config.focus_minutes * 60, self.config.rest_seconds);
        if let Err(e) = self.config.save() {
            self.status = format!("配置保存失败: {e}");
            return;
        }
        if let Err(e) = self.autostart.sync_from_config(self.config.launch_at_login) {
            self.status = format!("开机启动同步失败: {e}");
            return;
        }
        self.status = "配置已生效".to_string();
    }

    fn start_rest(&mut self) {
        // “立即休息”按钮直接触发状态机切换。
        let trans = self.engine.start_rest_now();
        self.handle_transition(trans);
    }

    fn reset_focus(&mut self, reason: &str) {
        // 手动重置/锁屏重置都走这里，保证行为一致。
        self.engine.reset_focus();
        self.pending_rest = None;
        let _ = self.overlay.hide_rest_overlay();
        self.status = reason.to_string();
    }

    fn handle_transition(&mut self, transition: EngineTransition) {
        // 消费状态机事件，并同步驱动遮罩、会话记录、状态文案。
        match transition {
            EngineTransition::Continue => {}
            EngineTransition::FocusToRest {
                scheduled_rest_seconds,
                started_at_epoch,
            } => {
                self.pending_rest = Some(PendingRest {
                    started_at_epoch,
                    scheduled_rest_seconds,
                });
                self.status = "休息中".to_string();

                if let Err(e) = self.overlay.show_rest_overlay(scheduled_rest_seconds) {
                    self.status = format!("遮罩失败: {e}");
                    return;
                }

                if self.config.skip_rest_enabled {
                    let now = now_epoch_seconds();
                    self.sessions.push_skipped(
                        scheduled_rest_seconds,
                        started_at_epoch,
                        now,
                        "skip-mode",
                    );
                    if let Err(e) = self.sessions.save() {
                        self.status = format!("保存会话失败: {e}");
                    }
                    self.pending_rest = None;
                    self.engine.force_back_to_focus();
                    let _ = self.overlay.hide_rest_overlay();
                    self.status = "不休息：已自动跳过并记录".to_string();
                }
            }
            EngineTransition::RestCompleted {
                scheduled_rest_seconds,
                actual_rest_seconds,
                started_at_epoch,
                ended_at_epoch,
            } => {
                self.sessions.push_completed(
                    scheduled_rest_seconds,
                    actual_rest_seconds,
                    started_at_epoch,
                    ended_at_epoch,
                );
                if let Err(e) = self.sessions.save() {
                    self.status = format!("保存会话失败: {e}");
                } else {
                    self.status = "休息完成，已记录".to_string();
                }
                self.pending_rest = None;
                let _ = self.overlay.hide_rest_overlay();
            }
        }
    }

    fn poll_overlay_event(&mut self) {
        // 轮询遮罩进程退出事件，用于识别 ESC 跳过并落盘记录。
        match self.overlay.poll_event() {
            Ok(Some(OverlayEvent::Skipped)) => {
                if self.engine.phase() == Phase::Resting {
                    let ended = now_epoch_seconds();
                    if let Some(p) = self.pending_rest.take() {
                        self.sessions.push_skipped(
                            p.scheduled_rest_seconds,
                            p.started_at_epoch,
                            ended,
                            "esc",
                        );
                    } else {
                        let started = ended - self.config.rest_seconds as i64;
                        self.sessions
                            .push_skipped(self.config.rest_seconds, started, ended, "esc");
                    }
                    if let Err(e) = self.sessions.save() {
                        self.status = format!("保存跳过记录失败: {e}");
                    } else {
                        self.status = "按 ESC 跳过，已记录".to_string();
                    }
                    self.engine.force_back_to_focus();
                }
            }
            Ok(Some(OverlayEvent::Completed)) => {}
            Ok(None) => {}
            Err(e) => self.status = format!("遮罩事件异常: {e}"),
        }
    }

    fn poll_lock_state(&mut self) {
        // 降频轮询锁屏状态，避免每帧都起系统命令。
        if self.last_lock_poll.elapsed() < Duration::from_secs(2) {
            return;
        }
        self.last_lock_poll = Instant::now();
        let current = self.lock_monitor.is_session_locked();
        if current && !self.last_lock_state {
            self.reset_focus("检测到锁屏，已重置");
        }
        self.last_lock_state = current;
    }

    fn tick(&mut self) {
        // 每秒推进一次状态机；如果系统中断过久，重置到 focusing。
        let elapsed = self.last_tick.elapsed().as_secs();
        if elapsed >= 10 {
            self.reset_focus("检测到系统中断，已重置");
            self.last_tick = Instant::now();
            return;
        }
        if elapsed < 1 {
            return;
        }
        for _ in 0..elapsed {
            let trans = self.engine.tick(self.config.skip_rest_enabled);
            self.handle_transition(trans);
        }
        self.last_tick = Instant::now();
    }

    fn draw_popup(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        // 固定分区布局（接近设计稿）：
        // 先算每个卡片矩形，再在对应区域绘制控件。
        let s = self.scale;
        let base = ui.max_rect();
        let panel_w = BASE_PANEL_W * s;
        let panel_x = base.center().x - panel_w / 2.0;

        let header = Rect::from_min_size(pos2(panel_x, 16.0 * s), vec2(panel_w, 88.0 * s));
        let timer = Rect::from_min_size(pos2(panel_x, 118.0 * s), vec2(panel_w, 176.0 * s));
        let settings = Rect::from_min_size(pos2(panel_x, 308.0 * s), vec2(panel_w, 300.0 * s));
        let records = Rect::from_min_size(pos2(panel_x, 622.0 * s), vec2(panel_w, 300.0 * s));
        let footer = Rect::from_min_size(pos2(panel_x, 934.0 * s), vec2(panel_w, 56.0 * s));

        draw_card(ui, header);
        draw_card(ui, timer);
        draw_card(ui, settings);
        draw_card(ui, records);

        self.draw_header(ui, header.shrink2(vec2(20.0 * s, 14.0 * s)), s);
        self.draw_timer(ui, timer.shrink2(vec2(20.0 * s, 14.0 * s)), s);
        self.draw_settings(ui, settings.shrink2(vec2(20.0 * s, 14.0 * s)), s);
        self.draw_records(ui, records.shrink2(vec2(20.0 * s, 14.0 * s)), s);
        self.draw_footer(ui, footer, ctx, s);
    }

    fn draw_header(&self, ui: &mut egui::Ui, rect: Rect, s: f32) {
        let _ = ui.allocate_new_ui(UiBuilder::new().max_rect(rect), |ui| {
            ui.horizontal(|ui| {
                let (logo_rect, _) =
                    ui.allocate_exact_size(Vec2::new(42.0 * s, 42.0 * s), egui::Sense::hover());
                let p = ui.painter();
                p.circle_filled(logo_rect.center(), 19.0 * s, Color32::from_rgb(204, 224, 234));
                p.circle_stroke(
                    logo_rect.center(),
                    13.0 * s,
                    Stroke::new(2.2 * s, Color32::from_rgb(42, 52, 62)),
                );
                p.line_segment(
                    [
                        logo_rect.center() + Vec2::new(-6.0, 0.0),
                        logo_rect.center() + Vec2::new(6.0, 0.0),
                    ],
                    Stroke::new(2.0, Color32::from_rgb(42, 52, 62)),
                );

                ui.vertical(|ui| {
                    ui.label(RichText::new("Rhythm").size(22.0 * s).strong());
                    ui.label(
                        RichText::new("专注与休息节奏")
                            .size(13.0 * s)
                            .color(Color32::from_rgb(90, 102, 112)),
                    );
                });

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let text = if self.engine.phase() == Phase::Focusing {
                        "专注中"
                    } else {
                        "休息中"
                    };
                    let _ = ui.add(
                        egui::Button::new(RichText::new(text).size(16.0 * s).strong())
                            .fill(Color32::from_rgb(205, 219, 228))
                            .stroke(Stroke::NONE)
                            .rounding(egui::Rounding::same(16.0 * s)),
                    );
                });
            });
        });
    }

    fn draw_timer(&self, ui: &mut egui::Ui, rect: Rect, s: f32) {
        let _ = ui.allocate_new_ui(UiBuilder::new().max_rect(rect), |ui| {
            ui.label(RichText::new("距离休息").size(18.0 * s).strong());
            ui.add_space(24.0 * s);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    RichText::new(format_seconds(self.engine.remaining_seconds()))
                        .size(72.0 * s)
                        .strong(),
                );
            });
        });
    }

    fn draw_settings(&mut self, ui: &mut egui::Ui, rect: Rect, s: f32) {
        // settings 卡只负责“采集用户改动”，真正应用在函数末尾统一处理。
        let mut changed = false;
        let focus_text = format!("{} 分钟", self.config.focus_minutes);
        let rest_text = format_rest_text(self.config.rest_seconds);
        let _ = ui.allocate_new_ui(UiBuilder::new().max_rect(rect), |ui| {
            ui.label(RichText::new("节奏设置").size(20.0 * s).strong());
            ui.add_space(12.0 * s);
            changed |= step_row(
                ui,
                "专注间隔",
                &mut self.config.focus_minutes,
                10,
                120,
                5,
                focus_text.clone(),
                s,
            );
            changed |= step_row(
                ui,
                "休息时长",
                &mut self.config.rest_seconds,
                30,
                600,
                30,
                rest_text.clone(),
                s,
            );
            changed |= switch_row(ui, "不休息", &mut self.config.skip_rest_enabled, s);
            changed |= switch_row(ui, "开机启动", &mut self.config.launch_at_login, s);
        });
        if changed {
            self.apply_config();
        }
    }

    fn draw_records(&self, ui: &mut egui::Ui, rect: Rect, s: f32) {
        let _ = ui.allocate_new_ui(UiBuilder::new().max_rect(rect), |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("最近记录").size(20.0 * s).strong());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        RichText::new(format!("{} 次", self.sessions.recent(1000).len()))
                            .size(16.0 * s)
                            .color(Color32::from_rgb(95, 104, 112)),
                    );
                });
            });
            ui.add_space(10.0 * s);
            for item in self.sessions.recent(5) {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(format_session_time(item))
                            .size(16.0 * s)
                            .color(Color32::from_rgb(98, 106, 114)),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let text = if item.skipped {
                            format!("跳过 {}", format_seconds(item.scheduled_rest_seconds))
                        } else {
                            format!("完成 {}", format_seconds(item.actual_rest_seconds))
                        };
                        let color = if item.skipped {
                            Color32::from_rgb(227, 129, 29)
                        } else {
                            Color32::from_rgb(48, 133, 78)
                        };
                        ui.label(RichText::new(text).size(16.0 * s).color(color));
                    });
                });
                ui.add_space(8.0 * s);
            }
        });
    }

    fn draw_footer(&mut self, ui: &mut egui::Ui, rect: Rect, ctx: &egui::Context, s: f32) {
        let _ = ui.allocate_new_ui(UiBuilder::new().max_rect(rect), |ui| {
            ui.horizontal(|ui| {
                if action_btn(ui, "立即休息", s).clicked() {
                    self.start_rest();
                }
                if action_btn(ui, "重置计时", s).clicked() {
                    self.reset_focus("手动重置");
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .add(
                            egui::Button::new(
                                RichText::new("退出")
                                    .size(18.0 * s)
                                    .color(Color32::from_rgb(94, 103, 111)),
                            )
                            .frame(false),
                        )
                        .clicked()
                    {
                        let _ = self.overlay.hide_rest_overlay();
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });
            });
        });
    }
}

impl eframe::App for RhythmGuiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // update 是每帧回调：先处理业务事件，再绘制 UI。
        ctx.request_repaint_after(Duration::from_millis(100));
        self.poll_overlay_event();
        self.poll_lock_state();
        self.tick();

        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(Color32::from_rgb(238, 247, 252)))
            .show(ctx, |ui| {
                self.draw_popup(ui, ctx);
            });
    }
}

fn draw_card(ui: &egui::Ui, rect: Rect) {
    // 统一卡片风格，避免每个区块重复写颜色/圆角/描边参数。
    ui.painter().rect(
        rect,
        egui::Rounding::same(22.0),
        Color32::from_rgb(225, 237, 245),
        Stroke::new(1.0, Color32::from_rgb(204, 217, 226)),
    );
}

fn step_row(
    ui: &mut egui::Ui,
    title: &str,
    value: &mut u64,
    min: u64,
    max: u64,
    step: u64,
    text: String,
    s: f32,
) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.label(RichText::new(title).size(18.0 * s).strong());
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if circle_btn(ui, "+", s).clicked() && *value + step <= max {
                *value += step;
                changed = true;
            }
            ui.add_space(10.0 * s);
            ui.label(
                RichText::new(text)
                    .size(16.0 * s)
                    .color(Color32::from_rgb(88, 98, 107)),
            );
            ui.add_space(10.0 * s);
            if circle_btn(ui, "-", s).clicked() && *value >= min + step {
                *value -= step;
                changed = true;
            }
        });
    });
    ui.add_space(10.0 * s);
    changed
}

fn switch_row(ui: &mut egui::Ui, title: &str, value: &mut bool, s: f32) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.label(RichText::new(title).size(18.0 * s).strong());
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if pill_switch(ui, value, s).clicked() {
                changed = true;
            }
        });
    });
    ui.add_space(10.0 * s);
    changed
}

fn action_btn(ui: &mut egui::Ui, text: &str, s: f32) -> egui::Response {
    ui.add(
        egui::Button::new(RichText::new(text).size(20.0 * s).strong())
            .fill(Color32::from_rgb(214, 220, 226))
            .stroke(Stroke::new(1.0, Color32::from_rgb(182, 190, 197)))
            .rounding(egui::Rounding::same(14.0 * s))
            .min_size(Vec2::new(130.0 * s, 46.0 * s)),
    )
}

fn circle_btn(ui: &mut egui::Ui, text: &str, s: f32) -> egui::Response {
    ui.add(
        egui::Button::new(RichText::new(text).size(18.0 * s).strong())
            .fill(Color32::from_rgb(214, 220, 226))
            .stroke(Stroke::new(1.0, Color32::from_rgb(180, 188, 194)))
            .rounding(egui::Rounding::same(8.0 * s))
            .min_size(Vec2::new(38.0 * s, 38.0 * s)),
    )
}

fn pill_switch(ui: &mut egui::Ui, value: &mut bool, s: f32) -> egui::Response {
    let desired_size = Vec2::new(58.0 * s, 32.0 * s);
    let (rect, mut response) = ui.allocate_exact_size(desired_size, egui::Sense::click());
    if response.clicked() {
        *value = !*value;
        response.mark_changed();
    }
    let bg = if *value {
        Color32::from_rgb(45, 115, 232)
    } else {
        Color32::from_rgb(211, 216, 221)
    };
    ui.painter()
        .rect(rect, egui::Rounding::same(16.0 * s), bg, Stroke::NONE);
    let knob_x = if *value {
        rect.right() - 16.0 * s
    } else {
        rect.left() + 16.0 * s
    };
    ui.painter().circle_filled(
        egui::pos2(knob_x, rect.center().y),
        13.0 * s,
        Color32::from_rgb(244, 246, 248),
    );
    response
}

fn configure_egui(ctx: &egui::Context) {
    configure_fonts(ctx);
    let mut style = (*ctx.style()).clone();
    style.text_styles.insert(
        egui::TextStyle::Heading,
        FontId::new(20.0, FontFamily::Proportional),
    );
    style.text_styles.insert(
        egui::TextStyle::Body,
        FontId::new(16.0, FontFamily::Proportional),
    );
    style.text_styles.insert(
        egui::TextStyle::Button,
        FontId::new(16.0, FontFamily::Proportional),
    );
    style.visuals.window_fill = Color32::from_rgb(226, 236, 242);
    ctx.set_style(style);
}

fn configure_fonts(ctx: &egui::Context) {
    // 优先加载 Windows 中文字体，避免中文出现方块字。
    let mut fonts = egui::FontDefinitions::default();
    let candidates = [
        r"C:\Windows\Fonts\msyh.ttc",
        r"C:\Windows\Fonts\simhei.ttf",
        r"C:\Windows\Fonts\simsun.ttc",
    ];
    for p in candidates {
        if let Ok(bytes) = fs::read(p) {
            fonts
                .font_data
                .insert("zh_cn".to_string(), FontData::from_owned(bytes).into());
            if let Some(f) = fonts.families.get_mut(&FontFamily::Proportional) {
                f.insert(0, "zh_cn".to_string());
            }
            if let Some(f) = fonts.families.get_mut(&FontFamily::Monospace) {
                f.push("zh_cn".to_string());
            }
            break;
        }
    }
    ctx.set_fonts(fonts);
}

fn format_seconds(total: u64) -> String {
    let m = total / 60;
    let s = total % 60;
    format!("{m:02}:{s:02}")
}

fn format_rest_text(rest_seconds: u64) -> String {
    if rest_seconds % 60 == 0 {
        format!("{} 分钟", rest_seconds / 60)
    } else {
        format!("{} 秒", rest_seconds)
    }
}

fn format_session_time(s: &RestSession) -> String {
    if let Some(dt) = Local.timestamp_opt(s.started_at_epoch, 0).single() {
        dt.format("%m-%d %H:%M").to_string()
    } else {
        "-".to_string()
    }
}

fn now_epoch_seconds() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

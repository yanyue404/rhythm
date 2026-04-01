use std::time::{SystemTime, UNIX_EPOCH};

// 应用计时状态：专注中 / 休息中
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Focusing,
    Resting,
}

// 状态机每次 tick 后可能产生的“事件”。
// UI/后台主循环根据这些事件决定：
// - 是否弹出遮罩
// - 是否写入会话
// - 是否切回专注
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EngineTransition {
    Continue,
    FocusToRest {
        scheduled_rest_seconds: u64,
        started_at_epoch: i64,
    },
    RestCompleted {
        scheduled_rest_seconds: u64,
        actual_rest_seconds: u64,
        started_at_epoch: i64,
        ended_at_epoch: i64,
    },
}

// 纯业务状态机，不依赖 UI。
// 你可以把它当作“节奏规则引擎”，外层只负责消费 Transition。
#[derive(Debug)]
pub struct TimerEngine {
    focus_seconds: u64,
    rest_seconds: u64,
    remaining_seconds: u64,
    phase: Phase,
    rest_started_at_epoch: Option<i64>,
}

impl TimerEngine {
    // 创建一轮新计时，从 focusing 开始。
    pub fn new(focus_seconds: u64, rest_seconds: u64) -> Self {
        Self {
            focus_seconds,
            rest_seconds,
            remaining_seconds: focus_seconds,
            phase: Phase::Focusing,
            rest_started_at_epoch: None,
        }
    }

    // 每调用一次 tick，代表时间推进 1 秒。
    // 返回值告诉调用方：本秒是否发生阶段切换。
    pub fn tick(&mut self, skip_rest_enabled: bool) -> EngineTransition {
        if self.remaining_seconds > 0 {
            self.remaining_seconds -= 1;
            return EngineTransition::Continue;
        }

        match self.phase {
            Phase::Focusing => {
                // 专注结束 -> 进入休息
                self.phase = Phase::Resting;
                self.remaining_seconds = self.rest_seconds;
                let started = now_epoch_seconds();
                self.rest_started_at_epoch = Some(started);

                if skip_rest_enabled {
                    // 开启不休息时，不进入真正休息流程，直接回到专注。
                    self.force_back_to_focus();
                }

                EngineTransition::FocusToRest {
                    scheduled_rest_seconds: self.rest_seconds,
                    started_at_epoch: started,
                }
            }
            Phase::Resting => {
                // 休息结束 -> 回到专注，并产出实际休息时长。
                let started = self.rest_started_at_epoch.unwrap_or_else(now_epoch_seconds);
                let ended = now_epoch_seconds();
                let actual = (ended - started).max(0) as u64;
                self.phase = Phase::Focusing;
                self.remaining_seconds = self.focus_seconds;
                self.rest_started_at_epoch = None;

                EngineTransition::RestCompleted {
                    scheduled_rest_seconds: self.rest_seconds,
                    actual_rest_seconds: actual,
                    started_at_epoch: started,
                    ended_at_epoch: ended,
                }
            }
        }
    }

    pub fn phase(&self) -> Phase {
        self.phase
    }

    pub fn remaining_seconds(&self) -> u64 {
        self.remaining_seconds
    }

    // 更新节奏配置后，立即重置当前阶段剩余时间，保证“改配置立即生效”。
    pub fn update_schedule(&mut self, focus_seconds: u64, rest_seconds: u64) {
        self.focus_seconds = focus_seconds;
        self.rest_seconds = rest_seconds;
        if matches!(self.phase, Phase::Focusing) {
            self.remaining_seconds = self.focus_seconds;
        } else {
            self.remaining_seconds = self.rest_seconds;
        }
    }

    // “立即休息”按钮调用这个方法，强制切换到 resting。
    pub fn start_rest_now(&mut self) -> EngineTransition {
        self.phase = Phase::Resting;
        self.remaining_seconds = self.rest_seconds;
        let started = now_epoch_seconds();
        self.rest_started_at_epoch = Some(started);
        EngineTransition::FocusToRest {
            scheduled_rest_seconds: self.rest_seconds,
            started_at_epoch: started,
        }
    }

    // 不休息或 ESC 跳过后，强制切回 focusing。
    pub fn force_back_to_focus(&mut self) {
        self.phase = Phase::Focusing;
        self.remaining_seconds = self.focus_seconds;
        self.rest_started_at_epoch = None;
    }

    // 锁屏/手动重置时调用，重置到一轮新的 focusing。
    pub fn reset_focus(&mut self) {
        self.phase = Phase::Focusing;
        self.remaining_seconds = self.focus_seconds;
        self.rest_started_at_epoch = None;
    }
}

// 用于会话记录时间戳。
fn now_epoch_seconds() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::{EngineTransition, TimerEngine};

    #[test]
    fn should_enter_rest_after_focus_countdown() {
        let mut engine = TimerEngine::new(1, 2);
        assert!(matches!(engine.tick(false), EngineTransition::Continue));
        assert!(matches!(engine.tick(false), EngineTransition::FocusToRest { .. }));
    }

    #[test]
    fn should_complete_rest_and_back_to_focus() {
        let mut engine = TimerEngine::new(0, 0);
        assert!(matches!(engine.tick(false), EngineTransition::FocusToRest { .. }));
        assert!(matches!(engine.tick(false), EngineTransition::RestCompleted { .. }));
    }
}

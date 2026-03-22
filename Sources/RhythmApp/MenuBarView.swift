import AppKit
import RhythmCore
import SwiftUI

struct MenuBarView: View {
    @ObservedObject var timerEngine: TimerEngine
    @ObservedObject var settingsStore: SettingsStore
    @ObservedObject var sessionStore: SessionStore

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            RhythmBrandBadge()
            Divider()
            statusSection
            Divider()
            configSection
            Divider()
            sessionsSection
            Divider()
            actionSection
        }
        .padding(14)
        .frame(width: 360)
    }

    private var statusSection: some View {
        VStack(alignment: .leading, spacing: 6) {
            Text(timerEngine.mode == .focusing ? "当前状态：专注中" : "当前状态：休息中")
                .font(.headline)

            if timerEngine.mode == .focusing {
                Text("距离休息还有 \(formatDuration(timerEngine.secondsUntilBreak))")
                    .font(.subheadline)
                    .foregroundStyle(.secondary)
            } else {
                Text("休息遮罩已显示，按 ESC 可跳过")
                    .font(.subheadline)
                    .foregroundStyle(.secondary)
            }
        }
    }

    private var configSection: some View {
        VStack(alignment: .leading, spacing: 10) {
            Text("节奏设置")
                .font(.subheadline.weight(.semibold))

            compactSettingRow(
                title: "专注间隔",
                value: "\(settingsStore.focusMinutes) 分钟",
                canDecrease: settingsStore.focusMinutes > SettingsStore.minFocusMinutes,
                canIncrease: settingsStore.focusMinutes < SettingsStore.maxFocusMinutes,
                onDecrease: decreaseFocusDuration,
                onIncrease: increaseFocusDuration
            )

            compactSettingRow(
                title: "休息时长",
                value: restSettingLabel(settingsStore.restSeconds),
                canDecrease: settingsStore.restSeconds > (SettingsStore.restPresetSeconds.first ?? SettingsStore.minRestSeconds),
                canIncrease: settingsStore.restSeconds < (SettingsStore.restPresetSeconds.last ?? SettingsStore.maxRestSeconds),
                onDecrease: decreaseRestDuration,
                onIncrease: increaseRestDuration
            )
        }
    }

    private var sessionsSection: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack {
                Text("最近记录")
                    .font(.subheadline.weight(.semibold))
                Spacer()
                Text("\(sessionStore.sessions.count) 次")
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }

            if sessionStore.sessions.isEmpty {
                Text("暂无记录")
                    .font(.caption)
                    .foregroundStyle(.secondary)
            } else {
                ForEach(sessionStore.sessions.prefix(5)) { session in
                    HStack {
                        Text(timeLabel(session.startedAt))
                            .font(.caption)
                            .foregroundStyle(.secondary)
                        Spacer()
                        Text(
                            session.skipped
                                ? "跳过 \(formatDuration(session.actualRestSeconds))"
                                : "完成 \(formatDuration(session.actualRestSeconds))"
                        )
                        .font(.caption)
                        .foregroundStyle(session.skipped ? .orange : .green)
                    }
                }
            }
        }
    }

    private var actionSection: some View {
        HStack {
            if timerEngine.mode == .focusing {
                Button("立即休息") {
                    timerEngine.startBreakNow()
                }
            } else {
                Button("跳过本次休息") {
                    timerEngine.skipBreak()
                }
            }

            Button("重置计时") {
                timerEngine.resetCycle()
            }

            Spacer()

            Button("退出") {
                NSApplication.shared.terminate(nil)
            }
        }
    }

    private func formatDuration(_ seconds: Int) -> String {
        let minute = max(0, seconds) / 60
        let second = max(0, seconds) % 60
        return String(format: "%02d:%02d", minute, second)
    }

    private func restSettingLabel(_ seconds: Int) -> String {
        if seconds < 60 {
            return "\(seconds) 秒"
        }
        let minute = seconds / 60
        let second = seconds % 60
        if second == 0 {
            return "\(minute) 分钟"
        }
        return "\(minute)分\(second)秒"
    }

    @ViewBuilder
    private func compactSettingRow(
        title: String,
        value: String,
        canDecrease: Bool,
        canIncrease: Bool,
        onDecrease: @escaping () -> Void,
        onIncrease: @escaping () -> Void
    ) -> some View {
        HStack(spacing: 10) {
            Text(title)
                .frame(width: 62, alignment: .leading)

            Spacer(minLength: 0)

            HStack(spacing: 8) {
                compactAdjustButton(systemImage: "minus", enabled: canDecrease, action: onDecrease)

                Text(value)
                    .frame(width: 96, alignment: .center)
                    .monospacedDigit()
                    .foregroundStyle(.secondary)

                compactAdjustButton(systemImage: "plus", enabled: canIncrease, action: onIncrease)
            }
            .frame(width: 150, alignment: .trailing)
        }
    }

    @ViewBuilder
    private func compactAdjustButton(systemImage: String, enabled: Bool, action: @escaping () -> Void) -> some View {
        Button(action: action) {
            Image(systemName: systemImage)
                .font(.system(size: 10.5, weight: .semibold))
                .frame(width: 22, height: 20)
                .contentShape(Rectangle())
        }
        .buttonStyle(.borderless)
        .foregroundStyle(enabled ? .secondary : .tertiary)
        .disabled(!enabled)
    }

    private func increaseFocusDuration() {
        settingsStore.focusMinutes += SettingsStore.focusMinutesStep
    }

    private func decreaseFocusDuration() {
        settingsStore.focusMinutes -= SettingsStore.focusMinutesStep
    }

    private func increaseRestDuration() {
        let options = SettingsStore.restPresetSeconds
        guard let next = options.first(where: { $0 > settingsStore.restSeconds }) else {
            settingsStore.restSeconds = options.last ?? settingsStore.restSeconds
            return
        }
        settingsStore.restSeconds = next
    }

    private func decreaseRestDuration() {
        let options = SettingsStore.restPresetSeconds
        guard let previous = options.reversed().first(where: { $0 < settingsStore.restSeconds }) else {
            settingsStore.restSeconds = options.first ?? settingsStore.restSeconds
            return
        }
        settingsStore.restSeconds = previous
    }

    private func timeLabel(_ date: Date) -> String {
        let formatter = DateFormatter()
        formatter.dateFormat = "MM-dd HH:mm"
        return formatter.string(from: date)
    }
}

import AppKit
import RhythmCore
import SwiftUI

struct MenuBarView: View {
    @ObservedObject var timerEngine: TimerEngine
    @ObservedObject var settingsStore: SettingsStore
    @ObservedObject var sessionStore: SessionStore
    @ObservedObject var launchAtLoginManager: LaunchAtLoginManager

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            headerSection
            statusSection
            configSection
            sessionsSection
            actionSection
        }
        .padding(14)
        .frame(width: 368)
        .frame(maxWidth: .infinity, alignment: .leading)
    }

    private var headerSection: some View {
        HStack(spacing: 10) {
            RhythmBrandBadge()

            Spacer(minLength: 0)

            Text(timerEngine.mode == .focusing ? "专注中" : "休息中")
                .font(.caption.weight(.semibold))
                .foregroundStyle(.secondary)
                .padding(.horizontal, 8)
                .padding(.vertical, 4)
                .background(
                    Capsule(style: .continuous)
                        .fill(Color.secondary.opacity(0.12))
                )
        }
    }

    private var statusSection: some View {
        sectionContainer {
            if timerEngine.mode == .focusing {
                sectionHeading("距离休息")

                HStack(alignment: .center, spacing: 12) {
                    VStack(alignment: .leading, spacing: 3) {
                        if settingsStore.skipRestEnabled {
                            Text("不休息模式：到点自动跳过并记录")
                                .font(.caption2)
                                .foregroundStyle(.secondary)
                        }
                    }

                    Spacer(minLength: 0)

                    Text(formatDuration(timerEngine.secondsUntilBreak))
                        .font(.system(size: 36, weight: .semibold, design: .rounded))
                        .monospacedDigit()
                }
            } else {
                sectionHeading("休息进行中")

                HStack(alignment: .center, spacing: 12) {
                    VStack(alignment: .leading, spacing: 3) {
                        Text("休息遮罩已显示")
                            .font(.caption)
                            .foregroundStyle(.secondary)
                    }

                    Spacer(minLength: 0)

                    Text("ESC 跳过")
                        .font(.caption.weight(.semibold))
                        .foregroundStyle(.secondary)
                }
            }
        }
    }

    private var configSection: some View {
        sectionContainer {
            sectionHeading("节奏设置")

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

            toggleSettingRow(
                title: "不休息",
                isOn: Binding(
                    get: { settingsStore.skipRestEnabled },
                    set: { settingsStore.skipRestEnabled = $0 }
                )
            )

            toggleSettingRow(
                title: "开机启动",
                isOn: Binding(
                    get: { launchAtLoginManager.isEnabled },
                    set: { launchAtLoginManager.setEnabled($0) }
                ),
                disabled: launchAtLoginManager.isApplying || launchAtLoginManager.isToggleDisabled
            )

            if let status = launchAtLoginManager.statusMessage {
                Text(status)
                    .font(.caption2)
                    .foregroundStyle(.secondary)
            }
        }
    }

    private var sessionsSection: some View {
        sectionContainer {
            HStack {
                sectionHeading("最近记录")
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
                            .monospacedDigit()
                            .foregroundStyle(.secondary)
                        Spacer()
                        Text(sessionResultLabel(session))
                            .font(.caption)
                            .monospacedDigit()
                            .foregroundStyle(session.skipped ? .orange : .green)
                    }
                }
            }
        }
    }

    private var actionSection: some View {
        HStack(spacing: 8) {
            if timerEngine.mode == .focusing {
                Button("立即休息") {
                    timerEngine.startBreakNow()
                }
                .buttonStyle(.bordered)
            } else {
                Button("跳过本次休息") {
                    timerEngine.skipBreak()
                }
                .buttonStyle(.bordered)
            }

            Button("重置计时") {
                timerEngine.resetCycle()
            }
            .buttonStyle(.bordered)

            Spacer()

            Button("退出") {
                NSApplication.shared.terminate(nil)
            }
            .buttonStyle(.borderless)
            .foregroundStyle(.secondary)
        }
        .controlSize(.small)
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
                .frame(width: 64, alignment: .leading)

            Spacer(minLength: 0)

            HStack(spacing: 8) {
                compactAdjustButton(systemImage: "minus", enabled: canDecrease, action: onDecrease)

                Text(value)
                    .frame(width: 96, alignment: .center)
                    .monospacedDigit()
                    .foregroundStyle(.secondary)

                compactAdjustButton(systemImage: "plus", enabled: canIncrease, action: onIncrease)
            }
            .frame(width: 154, alignment: .trailing)
        }
        .font(.subheadline)
    }

    @ViewBuilder
    private func toggleSettingRow(
        title: String,
        isOn: Binding<Bool>,
        disabled: Bool = false
    ) -> some View {
        HStack(spacing: 10) {
            Text(title)
                .frame(width: 64, alignment: .leading)

            Spacer(minLength: 0)

            Toggle("", isOn: isOn)
                .labelsHidden()
                .toggleStyle(.switch)
                .controlSize(.small)
                .disabled(disabled)
        }
        .font(.subheadline)
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

    private func sessionResultLabel(_ session: RestSession) -> String {
        if session.skipped {
            if session.skipReason == "no_rest" {
                return "不休息 \(formatDuration(session.actualRestSeconds))"
            }
            return "跳过 \(formatDuration(session.actualRestSeconds))"
        }
        return "完成 \(formatDuration(session.actualRestSeconds))"
    }

    @ViewBuilder
    private func sectionContainer<Content: View>(@ViewBuilder content: () -> Content) -> some View {
        VStack(alignment: .leading, spacing: 9) {
            content()
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(12)
        .background(
            RoundedRectangle(cornerRadius: 10, style: .continuous)
                .fill(Color.primary.opacity(0.055))
        )
    }

    @ViewBuilder
    private func sectionHeading(_ title: String) -> some View {
        Text(title)
            .font(.subheadline.weight(.semibold))
    }
}

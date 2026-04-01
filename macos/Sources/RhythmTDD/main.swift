import Foundation
import RhythmCore

@main
@MainActor
struct RhythmTDDRunner {
    static func main() {
        var failures = 0
        let includeUI = ProcessInfo.processInfo.environment["RHYTHM_TDD_UI"] != "0"

        failures += run("settings change callback fires once") {
            let isolated = makeIsolatedDefaults()
            defer { isolated.defaults.removePersistentDomain(forName: isolated.suiteName) }

            let store = SettingsStore(userDefaults: isolated.defaults)
            var callbackCount = 0
            store.onDidChange = {
                callbackCount += 1
            }
            store.focusMinutes = 35

            guard store.focusMinutes == 35 else { return false }
            return callbackCount == 1
        }

        failures += run("default settings are 30m focus and 1m rest") {
            let isolated = makeIsolatedDefaults()
            defer { isolated.defaults.removePersistentDomain(forName: isolated.suiteName) }

            let store = SettingsStore(userDefaults: isolated.defaults)
            guard store.focusMinutes == 30 else { return false }
            return store.restSeconds == 60
        }

        failures += run("settings normalization keeps configured range and rest presets") {
            let isolated = makeIsolatedDefaults()
            defer { isolated.defaults.removePersistentDomain(forName: isolated.suiteName) }

            let store = SettingsStore(userDefaults: isolated.defaults)
            store.focusMinutes = 0
            guard store.focusMinutes == 10 else { return false }

            store.restSeconds = 1
            guard store.restSeconds == 30 else { return false }

            store.restSeconds = 250
            guard store.restSeconds == 240 else { return false }

            store.focusMinutes = 119
            guard store.focusMinutes == 120 else { return false }

            store.restSeconds = 589
            return store.restSeconds == 600
        }

        failures += run("legacy rest minutes migrates to rest seconds") {
            let isolated = makeIsolatedDefaults()
            defer { isolated.defaults.removePersistentDomain(forName: isolated.suiteName) }

            isolated.defaults.set(3, forKey: SettingsStore.legacyRestMinutesKey)
            let store = SettingsStore(userDefaults: isolated.defaults)
            return store.restSeconds == 180
        }

        failures += run("timer skip records rest session") {
            let clock = TestClock(now: Date(timeIntervalSince1970: 1_000))
            let settings = FakeSettings(focusSeconds: 10, restSeconds: 5)
            let sessions = FakeSessionStore()
            let overlay = FakeOverlay()
            let lock = FakeLockMonitor()

            let engine = TimerEngine(
                settingsStore: settings,
                sessionStore: sessions,
                overlayManager: overlay,
                lockMonitor: lock,
                nowProvider: { clock.now },
                autoStart: false,
                useSystemTimer: false
            )

            engine.start()
            clock.now = clock.now.addingTimeInterval(10)
            engine.processTick(now: clock.now)
            guard engine.mode == .resting else { return false }
            guard overlay.lastPresentedRestSeconds == 5 else { return false }

            clock.now = clock.now.addingTimeInterval(2)
            overlay.onSkipped?()

            guard engine.mode == .focusing else { return false }
            guard engine.secondsUntilBreak == 10 else { return false }
            guard sessions.captured.count == 1 else { return false }
            guard sessions.captured[0].scheduledRestSeconds == 5 else { return false }
            guard sessions.captured[0].actualRestSeconds == 2 else { return false }
            guard sessions.captured[0].skipped else { return false }
            return sessions.captured[0].skipReason == "esc"
        }

        failures += run("no-rest mode records auto skipped session") {
            let clock = TestClock(now: Date(timeIntervalSince1970: 1_500))
            let settings = FakeSettings(focusSeconds: 12, restSeconds: 90, skipRestEnabled: true)
            let sessions = FakeSessionStore()
            let overlay = FakeOverlay()
            let lock = FakeLockMonitor()

            let engine = TimerEngine(
                settingsStore: settings,
                sessionStore: sessions,
                overlayManager: overlay,
                lockMonitor: lock,
                nowProvider: { clock.now },
                autoStart: false,
                useSystemTimer: false
            )

            engine.start()
            clock.now = clock.now.addingTimeInterval(12)
            engine.processTick(now: clock.now)

            guard engine.mode == .focusing else { return false }
            guard engine.secondsUntilBreak == 12 else { return false }
            guard overlay.lastPresentedRestSeconds == nil else { return false }
            guard sessions.captured.count == 1 else { return false }
            guard sessions.captured[0].skipped else { return false }
            guard sessions.captured[0].skipReason == "no_rest" else { return false }
            guard sessions.captured[0].scheduledRestSeconds == 90 else { return false }
            return sessions.captured[0].actualRestSeconds == 0
        }

        failures += run("screen lock resets cycle") {
            let clock = TestClock(now: Date(timeIntervalSince1970: 2_000))
            let settings = FakeSettings(focusSeconds: 12, restSeconds: 4)
            let sessions = FakeSessionStore()
            let overlay = FakeOverlay()
            let lock = FakeLockMonitor()

            let engine = TimerEngine(
                settingsStore: settings,
                sessionStore: sessions,
                overlayManager: overlay,
                lockMonitor: lock,
                nowProvider: { clock.now },
                autoStart: false,
                useSystemTimer: false
            )

            engine.start()
            clock.now = clock.now.addingTimeInterval(5)
            engine.processTick(now: clock.now)
            guard engine.secondsUntilBreak == 7 else { return false }

            lock.fireLock()

            guard engine.mode == .focusing else { return false }
            guard engine.secondsUntilBreak == 12 else { return false }
            return overlay.dismissCallCount == 1
        }

        if includeUI {
            failures += run("overlay smoke is visible and focusable") {
                runOverlayFocusSmokeCheck()
            }
        } else {
            print("SKIP: overlay smoke check (RHYTHM_TDD_UI=0)")
        }

        if failures == 0 {
            print("All TDD checks passed.")
            exit(0)
        } else {
            print("TDD checks failed: \(failures)")
            exit(1)
        }
    }
}

@MainActor
private func run(_ name: String, check: () -> Bool) -> Int {
    let passed = check()
    if passed {
        print("PASS: \(name)")
        return 0
    } else {
        print("FAIL: \(name)")
        return 1
    }
}

private func makeIsolatedDefaults() -> (defaults: UserDefaults, suiteName: String) {
    let suiteName = "RhythmTDD.\(UUID().uuidString)"
    return (UserDefaults(suiteName: suiteName)!, suiteName)
}

private func runOverlayFocusSmokeCheck() -> Bool {
    let cwd = FileManager.default.currentDirectoryPath
    let binaryCandidates = [
        "\(cwd)/.build/arm64-apple-macosx/debug/Rhythm",
        "\(cwd)/.build/x86_64-apple-macosx/debug/Rhythm"
    ]
    let binaryPath = binaryCandidates.first { FileManager.default.fileExists(atPath: $0) }

    var env = ProcessInfo.processInfo.environment
    env["RHYTHM_SMOKE_OVERLAY"] = "1"
    env["RHYTHM_OVERLAY_DEBUG"] = "1"

    let result: ProcessResult
    if let binaryPath {
        result = runProcess(
            executable: binaryPath,
            arguments: [],
            environment: env,
            timeout: 15
        )
    } else {
        result = runProcess(
            executable: "/bin/zsh",
            arguments: ["-lc", "swift run Rhythm"],
            environment: env,
            timeout: 40
        )
    }

    guard !result.timedOut else {
        print("overlay smoke timed out")
        return false
    }
    guard result.exitCode == 0 else {
        print("overlay smoke non-zero exit: \(result.exitCode)")
        print(result.output)
        return false
    }
    guard !result.output.localizedCaseInsensitiveContains("uncaught exception") else {
        print("overlay smoke crashed")
        print(result.output)
        return false
    }

    let requiredTokens = [
        "[RhythmSmoke] start",
        "[RhythmSmoke] trigger break",
        "[RhythmOverlay] present frame=",
        "[RhythmOverlay] post-check visible=true key=true main=true",
        "[RhythmSmoke] end"
    ]
    for token in requiredTokens where !result.output.contains(token) {
        print("missing smoke token: \(token)")
        print(result.output)
        return false
    }
    return true
}

private struct ProcessResult {
    let exitCode: Int32
    let output: String
    let timedOut: Bool
}

private func runProcess(
    executable: String,
    arguments: [String],
    environment: [String: String],
    timeout: TimeInterval
) -> ProcessResult {
    let process = Process()
    process.executableURL = URL(fileURLWithPath: executable)
    process.arguments = arguments
    process.environment = environment

    let pipe = Pipe()
    process.standardOutput = pipe
    process.standardError = pipe

    do {
        try process.run()
    } catch {
        return ProcessResult(exitCode: -1, output: "failed to run \(executable): \(error)", timedOut: false)
    }

    let deadline = Date().addingTimeInterval(timeout)
    while process.isRunning && Date() < deadline {
        Thread.sleep(forTimeInterval: 0.1)
    }

    let timedOut = process.isRunning
    if timedOut {
        process.terminate()
    }
    process.waitUntilExit()
    let outputData = pipe.fileHandleForReading.readDataToEndOfFile()
    let output = String(data: outputData, encoding: .utf8) ?? ""
    return ProcessResult(exitCode: process.terminationStatus, output: output, timedOut: timedOut)
}

@MainActor
private final class FakeSettings: RhythmSettings {
    var focusSeconds: Int
    var restSeconds: Int
    var skipRestEnabled: Bool
    var onDidChange: (() -> Void)?

    init(focusSeconds: Int, restSeconds: Int, skipRestEnabled: Bool = false) {
        self.focusSeconds = focusSeconds
        self.restSeconds = restSeconds
        self.skipRestEnabled = skipRestEnabled
    }
}

@MainActor
private final class FakeSessionStore: RestSessionStoring {
    private(set) var captured: [RestSession] = []

    func add(_ session: RestSession) {
        captured.append(session)
    }
}

@MainActor
private final class FakeOverlay: RestOverlaying {
    var onSkipped: (() -> Void)?
    var onCompleted: (() -> Void)?
    private(set) var dismissCallCount = 0
    private(set) var lastPresentedRestSeconds: Int?

    func present(restSeconds: Int) {
        lastPresentedRestSeconds = restSeconds
    }

    func dismiss() {
        dismissCallCount += 1
    }

    func skipByEscape() {
        onSkipped?()
    }
}

@MainActor
private final class FakeLockMonitor: ScreenLockMonitoring {
    var onScreenLocked: (() -> Void)?
    func start() {}
    func stop() {}
    func fireLock() {
        onScreenLocked?()
    }
}

private final class TestClock {
    var now: Date

    init(now: Date) {
        self.now = now
    }
}

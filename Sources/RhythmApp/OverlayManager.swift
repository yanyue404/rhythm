import AppKit
import RhythmCore
import SwiftUI

@MainActor
final class OverlayManager: ObservableObject {
    @Published private(set) var remainingSeconds: Int = 0
    @Published private(set) var isShowing: Bool = false

    var onSkipped: (() -> Void)?
    var onCompleted: (() -> Void)?

    private var overlayWindow: OverlayWindow?
    private var keyMonitor: Any?
    private var countdownTimer: Timer?
    private var focusEnforcerTimer: Timer?
    private var restEndAt: Date?

    func present(restSeconds: Int) {
        dismiss()

        remainingSeconds = max(1, restSeconds)
        restEndAt = Date().addingTimeInterval(TimeInterval(restSeconds))
        guard let screen = activeScreen() ?? NSScreen.main ?? NSScreen.screens.first else {
            restEndAt = nil
            remainingSeconds = 0
            isShowing = false
            return
        }
        isShowing = true

        let contentView = OverlayView(
            model: self,
            skipAction: { [weak self] in self?.skipByEscape() }
        )

        let window = OverlayWindow(
            contentRect: screen.frame,
            styleMask: [.borderless],
            backing: .buffered,
            defer: false
        )
        window.level = .screenSaver
        window.collectionBehavior = [.canJoinAllSpaces, .fullScreenAuxiliary, .moveToActiveSpace]
        window.isOpaque = false
        window.backgroundColor = .clear
        window.hasShadow = false
        window.ignoresMouseEvents = false
        window.isReleasedWhenClosed = false
        window.hidesOnDeactivate = false

        let hostingView = EscapeAwareHostingView(rootView: contentView)
        hostingView.onEscape = { [weak self] in
            self?.skipByEscape()
        }
        window.contentView = hostingView
        window.onEscape = { [weak self] in
            self?.skipByEscape()
        }

        overlayWindow = window
        activateAndFocus(firstResponder: hostingView)

        keyMonitor = NSEvent.addLocalMonitorForEvents(matching: .keyDown) { [weak self] event in
            guard let self else { return event }
            if event.keyCode == 53 {
                self.skipByEscape()
                return nil
            }
            return event
        }

        startFocusEnforcer()

        countdownTimer = Timer.scheduledTimer(withTimeInterval: 1, repeats: true) { [weak self] _ in
            Task { @MainActor [weak self] in
                self?.tick()
            }
        }
        RunLoop.main.add(countdownTimer!, forMode: .common)
    }

    func dismiss() {
        countdownTimer?.invalidate()
        countdownTimer = nil
        focusEnforcerTimer?.invalidate()
        focusEnforcerTimer = nil
        restEndAt = nil

        if let keyMonitor {
            NSEvent.removeMonitor(keyMonitor)
        }
        keyMonitor = nil

        overlayWindow?.orderOut(nil)
        overlayWindow = nil

        isShowing = false
        remainingSeconds = 0
    }

    func skipByEscape() {
        guard isShowing else { return }
        dismiss()
        onSkipped?()
    }

    private func tick() {
        guard let restEndAt else { return }
        let nextRemaining = max(0, Int(ceil(restEndAt.timeIntervalSinceNow)))
        remainingSeconds = nextRemaining

        if nextRemaining == 0 {
            dismiss()
            onCompleted?()
        }
    }

    private func startFocusEnforcer() {
        focusEnforcerTimer?.invalidate()
        focusEnforcerTimer = Timer.scheduledTimer(withTimeInterval: 0.25, repeats: true) { [weak self] _ in
            Task { @MainActor [weak self] in
                guard
                    let self,
                    self.isShowing,
                    let window = self.overlayWindow
                else { return }

                if !NSApp.isActive || !window.isKeyWindow {
                    self.activateAndFocus(firstResponder: window.contentView)
                }
            }
        }
        RunLoop.main.add(focusEnforcerTimer!, forMode: .common)
    }

    private func activateAndFocus(firstResponder: NSView?) {
        guard let overlayWindow else { return }
        NSRunningApplication.current.activate(options: [.activateIgnoringOtherApps, .activateAllWindows])
        NSApp.activate(ignoringOtherApps: true)
        overlayWindow.level = .screenSaver
        overlayWindow.makeKeyAndOrderFront(nil)
        overlayWindow.orderFrontRegardless()
        overlayWindow.makeMain()
        if let firstResponder {
            overlayWindow.makeFirstResponder(firstResponder)
        }
    }

    private func activeScreen() -> NSScreen? {
        let mouseLocation = NSEvent.mouseLocation
        return NSScreen.screens.first { screen in
            NSMouseInRect(mouseLocation, screen.frame, false)
        }
    }
}

private struct OverlayView: View {
    @ObservedObject var model: OverlayManager
    let skipAction: () -> Void

    var body: some View {
        ZStack {
            Color.black.opacity(0.55)
                .ignoresSafeArea()
            VStack(spacing: 16) {
                Text("休息时间")
                    .font(.system(size: 56, weight: .bold, design: .rounded))
                    .foregroundStyle(.white)
                Text(Self.format(model.remainingSeconds))
                    .font(.system(size: 64, weight: .heavy, design: .rounded))
                    .foregroundStyle(.white)
                    .monospacedDigit()
                Text("按 ESC 跳过本次休息")
                    .font(.system(size: 20, weight: .medium, design: .rounded))
                    .foregroundStyle(.white.opacity(0.9))
                Button("跳过") {
                    skipAction()
                }
                .keyboardShortcut(.cancelAction)
                .buttonStyle(.borderedProminent)
                .tint(.white.opacity(0.2))
            }
        }
    }

    private static func format(_ seconds: Int) -> String {
        let minute = max(0, seconds) / 60
        let second = max(0, seconds) % 60
        return String(format: "%02d:%02d", minute, second)
    }
}

private final class OverlayWindow: NSWindow {
    var onEscape: (() -> Void)?

    override var canBecomeKey: Bool { true }
    override var canBecomeMain: Bool { true }

    override func keyDown(with event: NSEvent) {
        if event.keyCode == 53 {
            onEscape?()
            return
        }
        super.keyDown(with: event)
    }
}

private final class EscapeAwareHostingView<Content: View>: NSHostingView<Content> {
    var onEscape: (() -> Void)?

    override var acceptsFirstResponder: Bool { true }

    override func keyDown(with event: NSEvent) {
        if event.keyCode == 53 {
            onEscape?()
            return
        }
        super.keyDown(with: event)
    }
}

extension OverlayManager: RestOverlaying {}

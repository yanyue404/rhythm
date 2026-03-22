import AppKit
import SwiftUI

struct RhythmMenuBarLabel: View {
    var body: some View {
        Image(nsImage: RhythmMenuBarTemplateIcon.image)
            .renderingMode(.template)
            .accessibilityLabel("Rhythm")
    }
}

struct RhythmBrandBadge: View {
    var body: some View {
        HStack(spacing: 10) {
            ZStack {
                RoundedRectangle(cornerRadius: 7, style: .continuous)
                    .fill(Color.primary.opacity(0.08))
                RhythmMenuLogo(size: 16)
                    .foregroundStyle(.primary.opacity(0.9))
            }
            .frame(width: 24, height: 24)

            VStack(alignment: .leading, spacing: 1) {
                Text("Rhythm")
                    .font(.headline)
                Text("专注与休息节奏")
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }
        }
    }
}

struct RhythmMenuLogo: View {
    var size: CGFloat = 16

    var body: some View {
        ZStack {
            Circle()
                .stroke(lineWidth: max(1.15, size * 0.10))
            RhythmPulseShape()
                .stroke(
                    style: StrokeStyle(
                        lineWidth: max(1.05, size * 0.10),
                        lineCap: .round,
                        lineJoin: .round
                    )
                )
                .padding(size * 0.18)
        }
        .frame(width: size, height: size)
    }
}

private enum RhythmMenuBarTemplateIcon {
    static let image: NSImage = {
        let size = NSSize(width: 18, height: 18)
        let image = NSImage(size: size, flipped: false) { rect in
            drawTemplateIcon(in: rect)
            return true
        }
        image.isTemplate = true
        return image
    }()

    private static func drawTemplateIcon(in rect: NSRect) {
        let ringRect = rect.insetBy(dx: 2.4, dy: 2.4)
        let ring = NSBezierPath(ovalIn: ringRect)
        ring.lineWidth = 1.45
        NSColor.black.setStroke()
        ring.stroke()

        let pulse = NSBezierPath()
        let midY = ringRect.midY
        pulse.move(to: NSPoint(x: ringRect.minX + ringRect.width * 0.08, y: midY))
        pulse.line(to: NSPoint(x: ringRect.minX + ringRect.width * 0.29, y: midY))
        pulse.line(to: NSPoint(x: ringRect.minX + ringRect.width * 0.42, y: ringRect.minY + ringRect.height * 0.24))
        pulse.line(to: NSPoint(x: ringRect.minX + ringRect.width * 0.56, y: ringRect.maxY - ringRect.height * 0.19))
        pulse.line(to: NSPoint(x: ringRect.minX + ringRect.width * 0.70, y: ringRect.minY + ringRect.height * 0.34))
        pulse.line(to: NSPoint(x: ringRect.minX + ringRect.width * 0.83, y: midY))
        pulse.line(to: NSPoint(x: ringRect.minX + ringRect.width * 0.92, y: midY))
        pulse.lineWidth = 1.35
        pulse.lineCapStyle = .round
        pulse.lineJoinStyle = .round
        pulse.stroke()
    }
}

private struct RhythmPulseShape: Shape {
    func path(in rect: CGRect) -> Path {
        var path = Path()
        let midY = rect.midY
        path.move(to: CGPoint(x: rect.minX + rect.width * 0.08, y: midY))
        path.addLine(to: CGPoint(x: rect.minX + rect.width * 0.29, y: midY))
        path.addLine(to: CGPoint(x: rect.minX + rect.width * 0.42, y: rect.minY + rect.height * 0.24))
        path.addLine(to: CGPoint(x: rect.minX + rect.width * 0.56, y: rect.maxY - rect.height * 0.19))
        path.addLine(to: CGPoint(x: rect.minX + rect.width * 0.70, y: rect.minY + rect.height * 0.34))
        path.addLine(to: CGPoint(x: rect.minX + rect.width * 0.83, y: midY))
        path.addLine(to: CGPoint(x: rect.minX + rect.width * 0.92, y: midY))
        return path
    }
}

import SwiftUI
import AppKit

enum Theme {
    static let accent = Color(red: 1, green: 0.733, blue: 0)          // #FFBB00
    static let accentNS = NSColor(srgbRed: 1, green: 0.733, blue: 0, alpha: 1)

    private static func dyn(_ dark: NSColor, _ light: NSColor) -> Color {
        Color(nsColor: NSColor(name: nil) { ap in
            ap.bestMatch(from: [.darkAqua, .aqua]) == .darkAqua ? dark : light
        })
    }
    private static func g(_ v: CGFloat, _ a: CGFloat = 1) -> NSColor { NSColor(white: v, alpha: a) }

    static let canvasBG = dyn(g(0.085), g(0.86))
    static let panelBG = dyn(g(0.135), g(0.955))
    static let panelBG2 = dyn(g(0.165), g(0.925))
    static let rowHover = dyn(g(1, 0.05), g(0, 0.045))
    static let rowSelected = dyn(NSColor(srgbRed: 1, green: 0.733, blue: 0, alpha: 0.16), NSColor(srgbRed: 1, green: 0.62, blue: 0, alpha: 0.18))
    static let field = dyn(g(1, 0.06), g(0, 0.05))
    static let fieldStrong = dyn(g(1, 0.1), g(0, 0.09))
    static let separator = dyn(g(1, 0.08), g(0, 0.1))
    static let text = dyn(g(0.92), g(0.1))
    static let textDim = dyn(g(0.6), g(0.38))
    static let textFaint = dyn(g(0.42), g(0.55))
    static let track = dyn(g(1, 0.09), g(0, 0.1))
    static let trackFill = dyn(NSColor(srgbRed: 1, green: 0.733, blue: 0, alpha: 0.55), NSColor(srgbRed: 0.95, green: 0.62, blue: 0, alpha: 0.7))
    static let accentText = dyn(NSColor(srgbRed: 1, green: 0.76, blue: 0.1, alpha: 1), NSColor(srgbRed: 0.75, green: 0.47, blue: 0, alpha: 1))

    static let labelFont = Font.system(size: 11.5)
    static let valueFont = Font.system(size: 11.5, design: .rounded).monospacedDigit()
    static let sectionFont = Font.system(size: 10.5, weight: .semibold)
}

struct PanelSection<Content: View>: View {
    let title: String
    var dot: Color? = nil
    var trailing: AnyView? = nil
    @State private var open = true
    @ViewBuilder var content: () -> Content

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            HStack(spacing: 6) {
                Image(systemName: "chevron.right")
                    .font(.system(size: 8.5, weight: .bold))
                    .rotationEffect(.degrees(open ? 90 : 0))
                    .foregroundStyle(Theme.textFaint)
                if let dot {
                    Circle().fill(dot).frame(width: 9, height: 9)
                        .overlay(Circle().stroke(Color.black.opacity(0.25), lineWidth: 0.5))
                }
                Text(tr(title).uppercased())
                    .font(Theme.sectionFont)
                    .tracking(0.6)
                    .foregroundStyle(Theme.textDim)
                Spacer()
                if let trailing { trailing }
            }
            .padding(.horizontal, 14)
            .padding(.vertical, 9)
            .contentShape(Rectangle())
            .onTapGesture { withAnimation(.snappy(duration: 0.2)) { open.toggle() } }
            if open {
                VStack(alignment: .leading, spacing: 10) { content() }
                    .padding(.horizontal, 14)
                    .padding(.bottom, 14)
            }
            Rectangle().fill(Theme.separator).frame(height: 1)
        }
    }
}

struct IconButton: View {
    let symbol: String
    var help: String = ""
    var active = false
    var size: CGFloat = 13
    let action: () -> Void
    @State private var hover = false

    var body: some View {
        Button(action: action) {
            Image(systemName: symbol)
                .font(.system(size: size, weight: .medium))
                .foregroundStyle(active ? Theme.accentText : (hover ? Theme.text : Theme.textDim))
                .frame(width: 26, height: 24)
                .background(RoundedRectangle(cornerRadius: 6).fill(hover || active ? Theme.rowHover : .clear))
                .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .onHover { hover = $0 }
        .help(tr(help))
    }
}

struct PillButtonStyle: ButtonStyle {
    var prominent = false
    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .font(.system(size: 12, weight: .medium))
            .padding(.horizontal, 12)
            .padding(.vertical, 6)
            .foregroundStyle(prominent ? Color.black : Theme.text)
            .background(RoundedRectangle(cornerRadius: 7).fill(prominent ? Theme.accent : Theme.fieldStrong))
            .opacity(configuration.isPressed ? 0.75 : 1)
    }
}

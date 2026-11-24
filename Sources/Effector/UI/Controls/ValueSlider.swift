import SwiftUI
import AppKit

/// Compact slider: drag anywhere on the track, ⌥ for fine control,
/// double-click to reset, click the value to type a number.
struct ValueSlider: View {
    let label: String
    let value: Double
    let range: ClosedRange<Double>
    var defaultValue: Double = 0
    var step: Double = 0
    var unit: String = ""
    /// Position → value mapping exponent (> 1 = finer control near the minimum).
    var curve: Double = 1
    /// Optional colours painted on the track.
    var track: [Color]? = nil
    let onChange: (Double) -> Void

    @State private var dragStart: Double?   // slider position (0…1) at drag start
    @State private var editing = false
    @State private var text = ""
    @State private var hover = false
    @FocusState private var focused: Bool

    private var span: Double { range.upperBound - range.lowerBound }
    private func position(of v: Double) -> Double {
        let lin = min(1, max(0, (v - range.lowerBound) / span))
        return curve == 1 ? lin : pow(lin, 1 / curve)
    }
    private func value(at pos: Double) -> Double {
        let t = min(1, max(0, pos))
        return range.lowerBound + span * (curve == 1 ? t : pow(t, curve))
    }
    private var fraction: Double { position(of: value) }
    private var originFraction: Double { position(of: min(max(0, range.lowerBound), range.upperBound)) }

    private var formatted: String {
        let decimals: Int
        if step >= 1 { decimals = 0 }
        else if step > 0 { decimals = step >= 0.1 ? 1 : 2 }
        else if span <= 4 || (curve > 1 && value < 1 && span < 30) { decimals = 2 }
        else if span <= 20 || (curve > 1 && value < 10) { decimals = 1 }
        else { decimals = 0 }
        return String(format: "%.\(decimals)f", value) + unit
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 5) {
            HStack {
                Text(tr(label)).font(Theme.labelFont).foregroundStyle(Theme.textDim)
                Spacer()
                if editing {
                    TextField("", text: $text)
                        .textFieldStyle(.plain)
                        .font(Theme.valueFont)
                        .multilineTextAlignment(.trailing)
                        .frame(width: 60)
                        .focused($focused)
                        .onSubmit { commitText() }
                        .onChange(of: focused) { _, f in if !f { commitText() } }
                } else {
                    Text(formatted)
                        .font(Theme.valueFont)
                        .foregroundStyle(abs(value - defaultValue) > 1e-9 ? Theme.text : Theme.textFaint)
                        .onTapGesture {
                            text = String(format: step >= 1 ? "%.0f" : "%.2f", value)
                            editing = true
                            DispatchQueue.main.async { focused = true }
                        }
                }
            }
            GeometryReader { geo in
                let w = geo.size.width
                let active = hover || dragStart != nil
                ZStack(alignment: .leading) {
                    if let track {
                        Capsule().fill(LinearGradient(colors: track, startPoint: .leading, endPoint: .trailing))
                            .frame(height: 5)
                            .overlay(Capsule().stroke(Theme.separator, lineWidth: 0.5))
                    } else {
                        Capsule().fill(Theme.track).frame(height: 4)
                        let a = min(fraction, originFraction), b = max(fraction, originFraction)
                        Capsule().fill(Theme.trackFill)
                            .frame(width: max(0, (b - a) * w), height: 4)
                            .offset(x: a * w)
                    }
                    Circle()
                        .fill(Color.white)
                        .shadow(color: .black.opacity(0.35), radius: 1.5, y: 0.5)
                        .frame(width: active ? 12 : 10, height: active ? 12 : 10)
                        .offset(x: fraction * w - (active ? 6 : 5))
                }
                .frame(height: 14)
                .contentShape(Rectangle())
                .gesture(
                    DragGesture(minimumDistance: 0)
                        .onChanged { g in
                            let fine = NSEvent.modifierFlags.contains(.option)
                            if dragStart == nil {
                                if fine {
                                    dragStart = fraction
                                } else {
                                    // jump to click position
                                    let pos = Double(g.startLocation.x / w)
                                    dragStart = position(of: clampValue(value(at: pos)))
                                    set(value(at: pos))
                                }
                            }
                            let dx = Double(g.translation.width / w) * (fine ? 0.1 : 1)
                            set(value(at: (dragStart ?? fraction) + dx))
                        }
                        .onEnded { _ in dragStart = nil }
                )
                .simultaneousGesture(TapGesture(count: 2).onEnded { onChange(defaultValue) })
            }
            .frame(height: 14)
        }
        .onHover { hover = $0 }
    }

    private func clampValue(_ v: Double) -> Double {
        var v = min(range.upperBound, max(range.lowerBound, v))
        if step > 0 { v = (v / step).rounded() * step }
        return min(range.upperBound, max(range.lowerBound, v))
    }

    private func set(_ v: Double) {
        let c = clampValue(v)
        if abs(c - value) > 1e-12 { onChange(c) }
    }

    private func commitText() {
        editing = false
        let cleaned = text.replacingOccurrences(of: ",", with: ".").filter { "-0123456789.".contains($0) }
        if let v = Double(cleaned) { set(v) }
    }
}

/// Small labeled row (label left, control right).
struct LabeledRow<Content: View>: View {
    let label: String
    @ViewBuilder var content: () -> Content
    var body: some View {
        HStack(spacing: 8) {
            Text(tr(label)).font(Theme.labelFont).foregroundStyle(Theme.textDim)
            Spacer(minLength: 4)
            content()
        }
    }
}

struct ToggleRow: View {
    let label: String
    let isOn: Bool
    let onChange: (Bool) -> Void
    var body: some View {
        LabeledRow(label: label) {
            Toggle("", isOn: Binding(get: { isOn }, set: onChange))
                .toggleStyle(.switch)
                .controlSize(.mini)
                .tint(Theme.accent)
                .labelsHidden()
        }
    }
}

struct ChoiceRow: View {
    let label: String
    let options: [ChoiceOption]
    let selected: String
    var scope: String? = nil
    let onChange: (String) -> Void
    var body: some View {
        LabeledRow(label: label) {
            PickerField(scope: scope ?? "choice." + label,
                        items: options.map { PickerItem(id: $0.id, label: tr($0.label), group: $0.group.map { tr($0) }) },
                        selected: selected, onSelect: onChange)
        }
    }
}

struct SegmentedRow: View {
    let label: String
    let options: [ChoiceOption]
    let selected: String
    let onChange: (String) -> Void
    var body: some View {
        VStack(alignment: .leading, spacing: 5) {
            if !label.isEmpty {
                Text(tr(label)).font(Theme.labelFont).foregroundStyle(Theme.textDim)
            }
            HStack(spacing: 2) {
                ForEach(options, id: \.id) { o in
                    Text(tr(o.label))
                        .font(.system(size: 11, weight: o.id == selected ? .semibold : .regular))
                        .lineLimit(1)
                        .minimumScaleFactor(0.8)
                        .frame(maxWidth: .infinity)
                        .padding(.vertical, 4)
                        .foregroundStyle(o.id == selected ? Theme.text : Theme.textDim)
                        .background(RoundedRectangle(cornerRadius: 5).fill(o.id == selected ? Theme.fieldStrong : .clear))
                        .contentShape(Rectangle())
                        .onTapGesture { onChange(o.id) }
                }
            }
            .padding(2)
            .background(RoundedRectangle(cornerRadius: 7).fill(Theme.field))
        }
    }
}

struct ColorRow: View {
    let label: String
    let color: RGBA
    let onChange: (RGBA) -> Void
    var body: some View {
        LabeledRow(label: label) {
            Text(color.hex).font(Theme.valueFont).foregroundStyle(Theme.textFaint)
            ColorPicker("", selection: Binding(get: { color.color }, set: { onChange(RGBA($0)) }), supportsOpacity: false)
                .labelsHidden()
                .frame(width: 34)
        }
    }
}

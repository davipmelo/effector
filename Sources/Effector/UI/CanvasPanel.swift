import SwiftUI

/// Controls for the aspect-ratio canvas (borders around the image, e.g. for Instagram).
struct CanvasControls: View {
    @ObservedObject var app = AppState.shared

    private var spec: CanvasSpec { app.state.canvas ?? CanvasSpec(enabled: false) }

    private func set(_ coalesce: String? = nil, _ body: @escaping (inout CanvasSpec) -> Void) {
        app.edit(coalesce.map { "canvas-" + $0 }) { s in
            var c = s.canvas ?? CanvasSpec(enabled: false)
            body(&c)
            s.canvas = c
        }
    }

    var body: some View {
        let c = spec
        VStack(alignment: .leading, spacing: 10) {
            ToggleRow(label: "Fit in Canvas", isOn: c.enabled) { v in set { $0.enabled = v } }
                .help(tr("Place the image on a canvas of another aspect ratio, with borders"))
            Group {
                LabeledRow(label: "Aspect Ratio") {
                    PickerField(scope: "canvas.ratio",
                                items: CanvasSpec.ratios.map { PickerItem(id: $0.id, label: tr($0.label), group: tr($0.group)) },
                                selected: c.ratio) { v in set { $0.ratio = v; $0.enabled = true } }
                }
                if c.ratio == "custom" {
                    ValueSlider(label: "Width", value: c.customW, range: 1...30, defaultValue: 4, step: 0.1) { v in set("w") { $0.customW = v } }
                    ValueSlider(label: "Height", value: c.customH, range: 1...30, defaultValue: 5, step: 0.1) { v in set("h") { $0.customH = v } }
                }
                ValueSlider(label: "Border", value: c.padding, range: 0...40, defaultValue: 4, step: 0.5, unit: "%") { v in set("pad") { $0.padding = v } }
                SegmentedRow(label: "Fill", options: opts([("color", "Color"), ("blur", "Blurred Photo"), ("gradient", "Gradient")]),
                             selected: c.fill) { v in set { $0.fill = v } }
                if c.fill != "blur" {
                    ColorRow(label: c.fill == "gradient" ? "Top Color" : "Color", color: c.color) { v in set("color") { $0.color = v } }
                }
                if c.fill == "gradient" {
                    ColorRow(label: "Bottom Color", color: c.color2) { v in set("color2") { $0.color2 = v } }
                }
                if c.fill == "blur" {
                    ValueSlider(label: "Background Blur", value: c.blur, range: 0...100, defaultValue: 60, step: 1, unit: "%") { v in set("blur") { $0.blur = v } }
                }
                HStack(spacing: 6) {
                    ForEach([("#FFFFFF", "White"), ("#0A0A0A", "Black"), ("#F3EFE6", "Cream"), ("#FFBB00", "Yellow")], id: \.0) { hex, name in
                        Circle().fill(RGBA(hex: hex).color).frame(width: 18, height: 18)
                            .overlay(Circle().stroke(Theme.separator))
                            .onTapGesture { set { $0.fill = "color"; $0.color = RGBA(hex: hex) } }
                            .help(tr(name))
                    }
                    Spacer()
                }
                ValueSlider(label: "Position X", value: c.offsetX, range: -100...100, defaultValue: 0, step: 1, unit: "%") { v in set("x") { $0.offsetX = v } }
                ValueSlider(label: "Position Y", value: c.offsetY, range: -100...100, defaultValue: 0, step: 1, unit: "%") { v in set("y") { $0.offsetY = v } }
                ValueSlider(label: "Corner Radius", value: c.radius, range: 0...50, defaultValue: 0, step: 0.5, unit: "%") { v in set("r") { $0.radius = v } }
                ValueSlider(label: "Shadow", value: c.shadow, range: 0...100, defaultValue: 0, step: 1, unit: "%") { v in set("s") { $0.shadow = v } }
            }
            .opacity(c.enabled ? 1 : 0.4)
            .allowsHitTesting(c.enabled)
            if let src = app.source, c.enabled {
                let (w, h) = app.outputSize()
                let (iw, ih) = src.croppedSize(app.state.crop)
                Text(tr("Canvas %@ px · image %@ px", "\(w) × \(h)", "\(iw) × \(ih)"))
                    .font(.system(size: 10.5)).foregroundStyle(Theme.textFaint)
            }
        }
    }
}

/// Popover shown from the toolbar's canvas button.
struct CanvasPopover: View {
    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            header(tr("Canvas & Borders"), symbol: "aspectratio")
            ScrollView { CanvasControls().padding(14) }
                .frame(maxHeight: 560)
        }
        .frame(width: 300)
        .background(Theme.panelBG)
    }
}

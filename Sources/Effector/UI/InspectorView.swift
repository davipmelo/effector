import SwiftUI

struct InspectorView: View {
    @ObservedObject var app = AppState.shared
    @ObservedObject var settings = AppSettings.shared

    var body: some View {
        VStack(spacing: 0) {
            if let layer = app.selectedLayer {
                LayerInspector(layer: layer)
                    .id(layer.id)
            } else {
                DocumentInspector()
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .top)
        .background(Theme.panelBG)
    }
}

struct DocumentInspector: View {
    @ObservedObject var app = AppState.shared
    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            header(tr("Image"))
            if let src = app.source {
                let (w, h) = src.croppedSize(app.state.crop)
                VStack(alignment: .leading, spacing: 8) {
                    infoRow("File", src.fileName)
                    infoRow("Size", "\(w) × \(h) px")
                    infoRow("Layers", "\(app.state.layers.flattened().count)")
                    if !app.state.crop.isFull { infoRow("Original", "\(src.width) × \(src.height) px") }
                }
                .padding(14)
                Rectangle().fill(Theme.separator).frame(height: 1)
                VStack(alignment: .leading, spacing: 8) {
                    Text(tr("Select a layer to edit its settings, or add an effect with the + button."))
                        .font(.system(size: 11.5)).foregroundStyle(Theme.textFaint)
                        .fixedSize(horizontal: false, vertical: true)
                }
                .padding(14)
            } else {
                Text(tr("No image open")).font(.system(size: 12)).foregroundStyle(Theme.textFaint).padding(14)
            }
            Spacer()
        }
    }

    private func infoRow(_ k: String, _ v: String) -> some View {
        HStack(alignment: .top) {
            Text(tr(k)).font(Theme.labelFont).foregroundStyle(Theme.textDim)
            Spacer()
            Text(v).font(Theme.valueFont).foregroundStyle(Theme.text).lineLimit(1).truncationMode(.middle)
        }
    }
}

func header(_ title: String, symbol: String? = nil) -> some View {
    HStack(spacing: 8) {
        if let symbol { Image(systemName: symbol).foregroundStyle(Theme.accentText).font(.system(size: 12, weight: .semibold)) }
        Text(title).font(.system(size: 13, weight: .semibold)).foregroundStyle(Theme.text)
        Spacer()
    }
    .padding(.horizontal, 14)
    .frame(height: 40)
    .overlay(alignment: .bottom) { Rectangle().fill(Theme.separator).frame(height: 1) }
}

struct LayerInspector: View {
    let layer: LayerNode
    @ObservedObject var app = AppState.shared
    @State private var name = ""

    private var effect: Effect? { EffectRegistry.effect(layer.effectID) }

    var body: some View {
        VStack(spacing: 0) {
            HStack(spacing: 8) {
                Image(systemName: layer.isGroup ? "folder" : (effect?.symbol ?? "square"))
                    .foregroundStyle(Theme.accentText).font(.system(size: 12, weight: .semibold))
                TextField("", text: $name)
                    .textFieldStyle(.plain)
                    .font(.system(size: 13, weight: .semibold))
                    .onSubmit { commitName() }
                    .onAppear { name = layer.name }
                Spacer()
                if !layer.isGroup {
                    IconButton(symbol: "arrow.counterclockwise", help: "Reset to defaults") { app.resetParams(layer.id) }
                }
                IconButton(symbol: layer.enabled ? "eye" : "eye.slash", help: "Show / Hide") { app.toggleVisibility(layer.id) }
            }
            .padding(.horizontal, 14)
            .frame(height: 40)
            .overlay(alignment: .bottom) { Rectangle().fill(Theme.separator).frame(height: 1) }

            ScrollView {
                VStack(spacing: 0) {
                    PanelSection(title: layer.isGroup ? "Group" : "Layer") {
                        ChoiceRow(label: "Blend", options: BlendMode.allCases.map { ChoiceOption(id: $0.rawValue, label: $0.label) },
                                  selected: layer.blend.rawValue) { v in
                            app.updateLayer(layer.id) { $0.blend = BlendMode(rawValue: v) ?? .normal }
                        }
                        ValueSlider(label: "Opacity", value: layer.opacity * 100, range: 0...100, defaultValue: 100, step: 1, unit: "%") { v in
                            app.updateLayer(layer.id, coalesce: "\(layer.id)-opacity") { $0.opacity = v / 100 }
                        }
                    }
                    if let effect {
                        ParamSections(layer: layer, defs: effect.params)
                    } else if layer.isGroup {
                        Text(tr("%d layers in this group", layer.children.count))
                            .font(.system(size: 11.5)).foregroundStyle(Theme.textFaint)
                            .frame(maxWidth: .infinity, alignment: .leading)
                            .padding(14)
                    }
                }
            }
            .scrollIndicators(.automatic)
        }
        .onChange(of: layer.name) { _, n in name = n }
    }

    private func commitName() {
        let n = name.trimmingCharacters(in: .whitespaces)
        if !n.isEmpty && n != layer.name { app.updateLayer(layer.id) { $0.name = n } }
    }
}

struct ParamSections: View {
    let layer: LayerNode
    let defs: [ParamDef]

    var body: some View {
        let visible = defs.filter { $0.visible?(layer.params) ?? true }
        let sections = orderedSections(visible)
        ForEach(sections, id: \.0) { title, items in
            PanelSection(title: title) {
                ForEach(items, id: \.key) { def in
                    ParamControl(layer: layer, def: def)
                }
            }
        }
    }

    private func orderedSections(_ defs: [ParamDef]) -> [(String, [ParamDef])] {
        var order: [String] = []
        var map: [String: [ParamDef]] = [:]
        for d in defs {
            let s = d.section ?? "Settings"
            if map[s] == nil { order.append(s); map[s] = [] }
            map[s]!.append(d)
        }
        return order.map { ($0, map[$0]!) }
    }
}

struct ParamControl: View {
    let layer: LayerNode
    let def: ParamDef
    @ObservedObject var app = AppState.shared

    private func set(_ v: ParamValue, coalesce: Bool = true) { app.setParam(layer.id, def.key, v, coalesce: coalesce) }

    var body: some View {
        let p = layer.params
        switch def.kind {
        case .slider(let mn, let mx, let d, let step, let unit):
            ValueSlider(label: def.label, value: p.num(def.key, d), range: mn...mx, defaultValue: d, step: step, unit: unit) {
                set(.number($0))
            }
        case .toggle(let d):
            ToggleRow(label: def.label, isOn: p.bool(def.key, d)) { set(.bool($0), coalesce: false) }
        case .choice(let o, let d):
            ChoiceRow(label: def.label, options: o, selected: p.choice(def.key, d)) { set(.choice($0), coalesce: false) }
        case .segmented(let o, let d):
            SegmentedRow(label: def.label, options: o, selected: p.choice(def.key, d)) { set(.choice($0), coalesce: false) }
        case .color(let d):
            ColorRow(label: def.label, color: p.color(def.key, d)) { set(.color($0)) }
        case .curves:
            CurveEditor(curves: p.curves(def.key)) { set(.curves($0)) }
        case .gradient(let d):
            GradientEditor(gradient: p.gradient(def.key) ?? d) { set(.gradient($0)) }
        case .palette(let d):
            PaletteEditor(palette: p.palette(def.key) ?? d) { set(.palette($0), coalesce: false) }
        case .text(let d):
            TextParamRow(label: def.label, value: p.text(def.key, d)) { set(.text($0)) }
        case .lut:
            LUTControl(lut: p.lut(def.key)) { set(.lut($0), coalesce: false) }
        case .seed:
            SeedRow(value: p.num(def.key, 1)) { set(.number($0), coalesce: false) }
        }
    }
}

struct TextParamRow: View {
    let label: String
    let value: String
    let onChange: (String) -> Void
    @State private var text = ""
    var body: some View {
        VStack(alignment: .leading, spacing: 5) {
            Text(tr(label)).font(Theme.labelFont).foregroundStyle(Theme.textDim)
            TextField("", text: $text)
                .textFieldStyle(.plain)
                .font(.system(size: 12))
                .padding(.horizontal, 8).padding(.vertical, 5)
                .background(RoundedRectangle(cornerRadius: 6).fill(Theme.field))
                .onAppear { text = value }
                .onChange(of: text) { _, t in if t != value { onChange(t) } }
        }
    }
}

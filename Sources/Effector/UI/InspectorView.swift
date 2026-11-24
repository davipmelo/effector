import SwiftUI

struct InspectorView: View {
    @ObservedObject var app = AppState.shared
    @ObservedObject var settings = AppSettings.shared

    var body: some View {
        VStack(spacing: 0) {
            if app.isMultiSelection {
                MultiInspector()
            } else if let layer = app.selectedLayer {
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

/// Shared settings for several selected layers.
struct MultiInspector: View {
    @ObservedObject var app = AppState.shared
    var body: some View {
        let nodes = app.selectionRoots.compactMap { app.state.layers.find($0) }
        let opacity = nodes.map(\.opacity).reduce(0, +) / Double(max(1, nodes.count))
        let blend = Set(nodes.map(\.blend)).count == 1 ? nodes.first?.blend.rawValue ?? "normal" : ""
        VStack(spacing: 0) {
            header(tr("%d layers selected", app.selectedIDs.count), symbol: "square.stack.3d.up")
            ScrollView {
                VStack(spacing: 0) {
                    PanelSection(title: "Layers") {
                        ChoiceRow(label: "Blend", options: BlendMode.menuOrder.map { ChoiceOption(id: $0.rawValue, label: $0.label, group: $0.group) },
                                  selected: blend, scope: "blend") { v in
                            app.updateSelected { $0.blend = BlendMode(rawValue: v) ?? .normal }
                        }
                        ValueSlider(label: "Opacity", value: opacity * 100, range: 0...100, defaultValue: 100, step: 1, unit: "%") { v in
                            app.updateSelected(coalesce: "multi-opacity") { $0.opacity = v / 100 }
                        }
                    }
                    VStack(alignment: .leading, spacing: 8) {
                        HStack(spacing: 8) {
                            Button { app.groupSelected() } label: { Label(tr("Group"), systemImage: "folder.badge.plus") }
                                .help(tr("Group selected layers (⌘G)"))
                            Button { app.duplicateSelected() } label: { Label(tr("Duplicate"), systemImage: "plus.square.on.square") }
                                .help(tr("Duplicate (⌘D)"))
                        }
                        HStack(spacing: 8) {
                            Button { app.toggleSelectedVisibility() } label: { Label(tr("Show / Hide"), systemImage: "eye") }
                                .help(tr("Show / Hide") + " (H)")
                            Button(role: .destructive) { app.deleteSelected() } label: { Label(tr("Delete"), systemImage: "trash") }
                                .help(tr("Delete") + " (⌘⌫)")
                        }
                        Text(tr("⇧-click selects a range, ⌘-click adds or removes one layer. Drag any selected layer to move them all."))
                            .font(.system(size: 11)).foregroundStyle(Theme.textFaint)
                            .fixedSize(horizontal: false, vertical: true)
                            .padding(.top, 4)
                    }
                    .buttonStyle(PillButtonStyle())
                    .padding(14)
                }
            }
        }
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
                    IconButton(symbol: "dice", help: "Randomize settings (R)") { app.randomizeParams(layer.id) }
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
                        ChoiceRow(label: "Blend", options: BlendMode.menuOrder.map { ChoiceOption(id: $0.rawValue, label: $0.label, group: $0.group) },
                                  selected: layer.blend.rawValue, scope: "blend") { v in
                            app.updateLayer(layer.id) { $0.blend = BlendMode(rawValue: v) ?? .normal }
                        }
                        ValueSlider(label: "Opacity", value: layer.opacity * 100, range: 0...100, defaultValue: 100, step: 1, unit: "%") { v in
                            app.updateLayer(layer.id, coalesce: "\(layer.id)-opacity") { $0.opacity = v / 100 }
                        }
                    }
                    if let effect {
                        if !effect.looks.isEmpty { LookRow(layer: layer, effect: effect) }
                        ParamSections(layer: layer, defs: effect.params, colors: effect.sectionColors)
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

struct LookRow: View {
    let layer: LayerNode
    let effect: Effect
    @ObservedObject var app = AppState.shared
    var body: some View {
        let looks = effect.looks
        HStack(spacing: 8) {
            Image(systemName: "sparkles").font(.system(size: 11)).foregroundStyle(Theme.accentText)
            Text(tr("Look")).font(Theme.labelFont).foregroundStyle(Theme.textDim)
            Spacer(minLength: 4)
            PickerField(scope: "look." + effect.id,
                        items: looks.map { PickerItem(id: $0.id, label: tr($0.name), group: $0.group.map { tr($0) }) },
                        selected: layer.params.text("_look", ""),
                        title: looks.first { $0.id == layer.params.text("_look", "") }.map { tr($0.name) } ?? tr("Custom")) { id in
                guard let look = looks.first(where: { $0.id == id }) else { return }
                app.applyLook(layer.id, look)
            }
        }
        .padding(.horizontal, 14)
        .padding(.vertical, 10)
        .overlay(alignment: .bottom) { Rectangle().fill(Theme.separator).frame(height: 1) }
    }
}

struct ParamSections: View {
    let layer: LayerNode
    let defs: [ParamDef]
    var colors: [String: RGBA] = [:]

    var body: some View {
        let visible = defs.filter { $0.visible?(layer.params) ?? true }
        let sections = orderedSections(visible)
        ForEach(sections, id: \.0) { title, items in
            let sw = items.first { $0.isSectionSwitch }
            let on = sw.map { d -> Bool in
                if case .toggle(let def) = d.kind { return layer.params.bool(d.key, def) }
                return true
            } ?? true
            PanelSection(title: title, dot: colors[title]?.color,
                         trailing: sw.map { d in AnyView(SectionSwitch(layer: layer, def: d, on: on)) }) {
                ForEach(items.filter { !$0.isSectionSwitch }, id: \.key) { def in
                    ParamControl(layer: layer, def: def)
                }
                .opacity(on ? 1 : 0.38)
                .allowsHitTesting(on)
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
        case .slider(let mn, let mx, let d, let step, let unit, let curve):
            ValueSlider(label: def.label, value: p.num(def.key, d), range: mn...mx, defaultValue: d, step: step, unit: unit,
                        curve: curve, track: def.track?.map(\.color)) {
                set(.number($0))
            }
        case .toggle(let d):
            ToggleRow(label: def.label, isOn: p.bool(def.key, d)) { set(.bool($0), coalesce: false) }
        case .choice(let o, let d):
            ChoiceRow(label: def.label, options: o, selected: p.choice(def.key, d), scope: layer.effectID + "." + def.key) {
                set(.choice($0), coalesce: false)
            }
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

/// The mini switch in a section header.
struct SectionSwitch: View {
    let layer: LayerNode
    let def: ParamDef
    let on: Bool
    @ObservedObject var app = AppState.shared
    var body: some View {
        Toggle("", isOn: Binding(get: { on }, set: { app.setParam(layer.id, def.key, .bool($0), coalesce: false) }))
            .toggleStyle(.switch)
            .controlSize(.mini)
            .tint(Theme.accent)
            .labelsHidden()
            .help(on ? tr("Turn this part off") : tr("Turn this part on"))
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

import SwiftUI
import AppKit
import UniformTypeIdentifiers

struct GradientEditor: View {
    let gradient: GradientSpec
    let onChange: (GradientSpec) -> Void
    @State private var selected: UUID?
    @ObservedObject private var lib = SwatchLibrary.shared

    private var gradientItems: [PickerItem] {
        lib.gradients.map { PickerItem(id: "s:" + $0.name, label: $0.name, group: tr("Saved"), gradient: $0.previewColors) }
        + GradientPresets.groups.flatMap { g in
            g.1.map { PickerItem(id: "b:" + $0.name, label: $0.name, group: tr(g.0), gradient: $0.previewColors) }
        }
    }
    private var selectedID: String {
        if lib.gradients.contains(where: { $0.name == gradient.name }) { return "s:" + gradient.name }
        return "b:" + gradient.name
    }
    private func lookup(_ id: String) -> GradientSpec? {
        let name = String(id.dropFirst(2))
        return id.hasPrefix("s:") ? lib.gradients.first { $0.name == name } : GradientPresets.all.first { $0.name == name }
    }
    private var gradientActions: [PickerAction] {
        var a = [PickerAction(label: tr("Save Gradient…"), symbol: "square.and.arrow.down") { saveGradient() },
                 PickerAction(label: tr("Import Gradients…"), symbol: "square.and.arrow.down.on.square") { lib.importGradients() },
                 PickerAction(label: tr("Export Gradient…"), symbol: "square.and.arrow.up") { lib.exportGradient(gradient) }]
        if lib.gradients.contains(gradient) {
            a.append(PickerAction(label: tr("Delete Saved Gradient"), symbol: "trash", destructive: true) { lib.deleteGradient(gradient) })
        }
        return a
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack {
                PickerField(scope: "gradient", items: gradientItems, selected: selectedID, title: gradient.name,
                            actions: gradientActions) { id in
                    if let g = lookup(id) { onChange(g) }
                }
                Spacer()
                Button { reverse() } label: { Image(systemName: "arrow.left.arrow.right") }
                    .buttonStyle(.plain).foregroundStyle(Theme.textDim).help(tr("Reverse"))
            }
            GeometryReader { geo in
                let w = geo.size.width
                let sorted = gradient.stops.sorted { $0.position < $1.position }
                ZStack(alignment: .topLeading) {
                    RoundedRectangle(cornerRadius: 5)
                        .fill(LinearGradient(stops: sorted.map { .init(color: $0.color.color, location: $0.position) },
                                             startPoint: .leading, endPoint: .trailing))
                        .frame(height: 22)
                        .overlay(RoundedRectangle(cornerRadius: 5).stroke(Theme.separator))
                        .contentShape(Rectangle())
                        .onTapGesture(coordinateSpace: .local) { loc in addStop(at: loc.x / w) }
                    ForEach(gradient.stops) { s in
                        stopHandle(s)
                            .position(x: s.position * w, y: 32)
                            .gesture(DragGesture(minimumDistance: 1).onChanged { g in
                                selected = s.id
                                move(s.id, to: min(1, max(0, g.location.x / w)), removing: g.location.y > 60)
                            })
                            .onTapGesture { selected = s.id }
                    }
                }
            }
            .frame(height: 42)
            if let sid = selected, let stop = gradient.stops.first(where: { $0.id == sid }) {
                HStack(spacing: 8) {
                    Text(tr("Stop")).font(Theme.labelFont).foregroundStyle(Theme.textDim)
                    Text("\(Int(stop.position * 100))%").font(Theme.valueFont).foregroundStyle(Theme.textFaint)
                    Spacer()
                    ColorPicker("", selection: Binding(get: { stop.color.color }, set: { c in
                        var g = gradient
                        if let i = g.stops.firstIndex(where: { $0.id == sid }) { g.stops[i].color = RGBA(c) }
                        onChange(g)
                    }), supportsOpacity: false).labelsHidden().frame(width: 34)
                    Button { remove(sid) } label: { Image(systemName: "trash") }
                        .buttonStyle(.plain).foregroundStyle(Theme.textDim)
                        .disabled(gradient.stops.count <= 2)
                }
            } else {
                Text(tr("Click the bar to add a stop")).font(.system(size: 10)).foregroundStyle(Theme.textFaint)
            }
        }
    }

    private func stopHandle(_ s: GradientStop) -> some View {
        VStack(spacing: 0) {
            Triangle().fill(selected == s.id ? Theme.accent : Theme.textDim).frame(width: 8, height: 5)
            RoundedRectangle(cornerRadius: 3)
                .fill(s.color.color)
                .frame(width: 12, height: 12)
                .overlay(RoundedRectangle(cornerRadius: 3).stroke(selected == s.id ? Theme.accent : Color.white.opacity(0.7), lineWidth: 1.5))
        }
    }

    private func addStop(at t: Double) {
        var g = gradient
        let s = GradientStop(position: min(1, max(0, t)), color: g.sample(t))
        g.stops.append(s)
        g.name = tr("Custom")
        selected = s.id
        onChange(g)
    }

    private func move(_ id: UUID, to t: Double, removing: Bool) {
        var g = gradient
        if removing && g.stops.count > 2 {
            g.stops.removeAll { $0.id == id }
            selected = nil
        } else if let i = g.stops.firstIndex(where: { $0.id == id }) {
            g.stops[i].position = t
        }
        onChange(g)
    }

    private func remove(_ id: UUID) {
        guard gradient.stops.count > 2 else { return }
        var g = gradient
        g.stops.removeAll { $0.id == id }
        selected = nil
        onChange(g)
    }

    private func reverse() {
        var g = gradient
        for i in g.stops.indices { g.stops[i].position = 1 - g.stops[i].position }
        onChange(g)
    }

    private func saveGradient() {
        if let name = promptText(tr("Save Gradient"), tr("Name"), gradient.name) {
            var g = gradient; g.name = name
            lib.saveGradient(g)
            onChange(g)
        }
    }
}

struct Triangle: Shape {
    func path(in r: CGRect) -> Path {
        Path { p in
            p.move(to: CGPoint(x: r.midX, y: r.minY))
            p.addLine(to: CGPoint(x: r.maxX, y: r.maxY))
            p.addLine(to: CGPoint(x: r.minX, y: r.maxY))
            p.closeSubpath()
        }
    }
}

/// Simple modal text prompt.
func promptText(_ title: String, _ label: String, _ initial: String) -> String? {
    let a = NSAlert()
    a.messageText = title
    a.informativeText = label
    let f = NSTextField(frame: NSRect(x: 0, y: 0, width: 260, height: 24))
    f.stringValue = initial
    a.accessoryView = f
    a.addButton(withTitle: tr("OK"))
    a.addButton(withTitle: tr("Cancel"))
    a.window.initialFirstResponder = f
    guard a.runModal() == .alertFirstButtonReturn else { return nil }
    let s = f.stringValue.trimmingCharacters(in: .whitespaces)
    return s.isEmpty ? nil : s
}

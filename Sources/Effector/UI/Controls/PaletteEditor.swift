import SwiftUI
import AppKit
import UniformTypeIdentifiers

struct PaletteEditor: View {
    let palette: PaletteSpec
    let onChange: (PaletteSpec) -> Void
    @ObservedObject private var lib = SwatchLibrary.shared
    @ObservedObject private var app = AppState.shared
    @State private var extractCount = 6.0
    @State private var selected: Int?

    private var paletteItems: [PickerItem] {
        lib.palettes.map { PickerItem(id: "s:" + $0.name, label: $0.name, group: tr("Saved"), swatches: $0.colors) }
        + PalettePresets.groups.flatMap { g in
            g.1.map { PickerItem(id: "b:" + $0.name, label: $0.name, group: tr(g.0), swatches: $0.colors) }
        }
    }
    private var selectedID: String {
        if lib.palettes.contains(where: { $0.name == palette.name }) { return "s:" + palette.name }
        return "b:" + palette.name
    }
    private func lookup(_ id: String) -> PaletteSpec? {
        let name = String(id.dropFirst(2))
        return id.hasPrefix("s:") ? lib.palettes.first { $0.name == name } : PalettePresets.all.first { $0.name == name }
    }
    private var paletteActions: [PickerAction] {
        var a = [PickerAction(label: tr("Save Palette…"), symbol: "square.and.arrow.down") { save() },
                 PickerAction(label: tr("Import Palettes…"), symbol: "square.and.arrow.down.on.square") {
                     if let p = lib.importPalettes() { onChange(p) } },
                 PickerAction(label: tr("Export Palette…"), symbol: "square.and.arrow.up") { lib.exportPalette(palette) }]
        if lib.palettes.contains(palette) {
            a.append(PickerAction(label: tr("Delete Saved Palette"), symbol: "trash", destructive: true) { lib.deletePalette(palette) })
        }
        return a
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack {
                PickerField(scope: "palette", items: paletteItems, selected: selectedID, title: palette.name,
                            actions: paletteActions) { id in
                    if let p = lookup(id) { onChange(p) }
                }
                Spacer()
                Text("\(palette.colors.count)").font(Theme.valueFont).foregroundStyle(Theme.textFaint)
            }
            LazyVGrid(columns: Array(repeating: GridItem(.flexible(), spacing: 4), count: 8), spacing: 4) {
                ForEach(Array(palette.colors.enumerated()), id: \.offset) { i, c in
                    RoundedRectangle(cornerRadius: 4)
                        .fill(c.color)
                        .aspectRatio(1, contentMode: .fit)
                        .overlay(RoundedRectangle(cornerRadius: 4).stroke(selected == i ? Theme.accent : Theme.separator, lineWidth: selected == i ? 2 : 1))
                        .onTapGesture { selected = (selected == i ? nil : i) }
                        .help(c.hex + " · " + tr("click to edit"))
                }
                if palette.colors.count < 64 {
                    RoundedRectangle(cornerRadius: 4)
                        .strokeBorder(Theme.textFaint, style: StrokeStyle(lineWidth: 1, dash: [3, 2]))
                        .aspectRatio(1, contentMode: .fit)
                        .overlay(Image(systemName: "plus").font(.system(size: 9, weight: .bold)).foregroundStyle(Theme.textDim))
                        .help(tr("Add a color"))
                        .onTapGesture {
                            var p = palette
                            p.colors.append(p.colors.last ?? .white)
                            p.name = tr("Custom")
                            selected = p.colors.count - 1
                            onChange(p)
                        }
                }
            }
            if let i = selected, i < palette.colors.count {
                HStack {
                    Text(palette.colors[i].hex).font(Theme.valueFont).foregroundStyle(Theme.textDim)
                    Spacer()
                    ColorPicker("", selection: Binding(get: { palette.colors[i].color }, set: { c in
                        var p = palette; p.colors[i] = RGBA(c); p.name = tr("Custom"); onChange(p)
                    }), supportsOpacity: false).labelsHidden().frame(width: 34)
                    Button {
                        var p = palette
                        if p.colors.count > 2 { p.colors.remove(at: i); selected = nil; p.name = tr("Custom"); onChange(p) }
                    } label: { Image(systemName: "trash") }
                        .buttonStyle(.plain).foregroundStyle(Theme.textDim)
                        .disabled(palette.colors.count <= 2)
                }
            }
            HStack(spacing: 8) {
                Button {
                    if let src = app.source { onChange(PalettePresets.extract(from: src.cgImage, count: Int(extractCount))) }
                } label: {
                    Label(tr("Extract from image"), systemImage: "eyedropper")
                }
                .buttonStyle(PillButtonStyle())
                .help(tr("Build a palette from the colors of your image"))
                Stepper(value: $extractCount, in: 2...16) {
                    Text("\(Int(extractCount))").font(Theme.valueFont).foregroundStyle(Theme.textDim)
                }
                .controlSize(.small)
            }
        }
    }

    private func save() {
        if let name = promptText(tr("Save Palette"), tr("Name"), palette.name) {
            var p = palette; p.name = name
            lib.savePalette(p)
            onChange(p)
        }
    }
}

/// Saved LUTs (copies of .cube files in Application Support/EFFECTOR/LUTs).
final class LUTLibrary: ObservableObject {
    static let shared = LUTLibrary()
    @Published private(set) var names: [String] = []
    private var cache: [String: LUTData] = [:]

    var dir: URL {
        let base = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0]
        let d = base.appendingPathComponent("EFFECTOR/LUTs", isDirectory: true)
        try? FileManager.default.createDirectory(at: d, withIntermediateDirectories: true)
        return d
    }

    private init() { reload() }

    func reload() {
        let files = (try? FileManager.default.contentsOfDirectory(at: dir, includingPropertiesForKeys: nil)) ?? []
        names = files.filter { $0.pathExtension.lowercased() == "cube" }
            .map { $0.deletingPathExtension().lastPathComponent }
            .sorted { $0.localizedStandardCompare($1) == .orderedAscending }
    }

    func contains(_ name: String) -> Bool { names.contains(name) }

    func load(_ name: String) -> LUTData? {
        if let c = cache[name] { return c }
        guard let l = try? CubeLUT.parse(url: dir.appendingPathComponent(name + ".cube")) else { return nil }
        cache[name] = l
        return l
    }

    /// Copies .cube files into the library. Returns the last imported LUT.
    @discardableResult
    func add(urls: [URL]) -> LUTData? {
        var last: LUTData?
        for u in urls where u.pathExtension.lowercased() == "cube" {
            guard let l = try? CubeLUT.parse(url: u) else { continue }
            let dest = dir.appendingPathComponent(u.lastPathComponent)
            if dest.standardizedFileURL != u.standardizedFileURL {
                try? FileManager.default.removeItem(at: dest)
                try? FileManager.default.copyItem(at: u, to: dest)
            }
            cache[l.name] = l
            last = l
        }
        reload()
        return last
    }

    /// Writes a LUT held in memory (e.g. from a project) into the library.
    func save(_ l: LUTData) {
        var text = "TITLE \"\(l.name)\"\nLUT_3D_SIZE \(l.size)\n"
        for i in stride(from: 0, to: l.values.count, by: 3) {
            text += String(format: "%.6f %.6f %.6f\n", l.values[i], l.values[i + 1], l.values[i + 2])
        }
        try? text.write(to: dir.appendingPathComponent(l.name + ".cube"), atomically: true, encoding: .utf8)
        cache[l.name] = l
        reload()
    }

    func delete(_ name: String) {
        try? FileManager.default.removeItem(at: dir.appendingPathComponent(name + ".cube"))
        cache[name] = nil
        reload()
    }
}

struct LUTControl: View {
    let lut: LUTData?
    let onChange: (LUTData) -> Void
    @ObservedObject private var lib = LUTLibrary.shared
    @State private var targeted = false

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack(spacing: 8) {
                Image(systemName: "cube").foregroundStyle(targeted ? Theme.accentText : Theme.textDim)
                if lib.names.isEmpty && lut == nil {
                    Text(tr("No LUT loaded")).font(Theme.labelFont).foregroundStyle(Theme.textFaint)
                    Spacer()
                } else {
                    PickerField(scope: "lut", items: lib.names.map { PickerItem(id: $0, label: $0) },
                                selected: lut?.name ?? "", title: lut?.name ?? tr("Choose a LUT"),
                                actions: actions, expand: true) { name in
                        if let l = lib.load(name) { onChange(l) }
                    }
                }
                Button(tr("Load .cube…")) { load() }.buttonStyle(PillButtonStyle())
                    .help(tr("Load .cube files (they are saved to your LUT library)"))
            }
            Text(targeted ? tr("Drop to load and save the LUT") : tr("Drop .cube files here · loaded LUTs are saved to your library"))
                .font(.system(size: 10.5)).foregroundStyle(targeted ? Theme.accentText : Theme.textFaint)
        }
        .padding(8)
        .background(RoundedRectangle(cornerRadius: 8).strokeBorder(targeted ? Theme.accent : Theme.separator,
                                                                   style: StrokeStyle(lineWidth: targeted ? 1.5 : 1, dash: [4, 3])))
        .onDrop(of: [.fileURL], isTargeted: $targeted) { providers in
            Task { @MainActor in
                var urls: [URL] = []
                for pr in providers {
                    if let u = try? await pr.loadItem(forTypeIdentifier: UTType.fileURL.identifier) as? Data,
                       let url = URL(dataRepresentation: u, relativeTo: nil) { urls.append(url) }
                }
                if let l = lib.add(urls: urls) { onChange(l) }
                else { AppState.shared.flash(tr("Invalid .cube file")) }
            }
            return true
        }
    }

    private var actions: [PickerAction] {
        var a: [PickerAction] = [PickerAction(label: tr("Load .cube…"), symbol: "folder") { load() },
                                 PickerAction(label: tr("Show LUT Folder"), symbol: "folder.badge.gearshape") {
                                     NSWorkspace.shared.open(lib.dir) }]
        if let l = lut, !lib.contains(l.name) {
            a.insert(PickerAction(label: tr("Save to Library"), symbol: "square.and.arrow.down") { lib.save(l) }, at: 0)
        }
        if let l = lut, lib.contains(l.name) {
            a.append(PickerAction(label: tr("Remove from Library"), symbol: "trash", destructive: true) { lib.delete(l.name) })
        }
        return a
    }

    private func load() {
        let p = NSOpenPanel()
        p.allowedContentTypes = [UTType(filenameExtension: "cube") ?? .data]
        p.allowsMultipleSelection = true
        guard p.runModal() == .OK else { return }
        if let l = lib.add(urls: p.urls) { onChange(l) }
        else { AppState.shared.alert(tr("Could not read LUT"), tr("Invalid .cube file")) }
    }
}

enum CubeLUT {
    struct ParseError: LocalizedError { var errorDescription: String? { tr("Invalid .cube file") } }

    static func parse(url: URL) throws -> LUTData {
        let text = try String(contentsOf: url, encoding: .utf8)
        var size = 0
        var vals: [Float] = []
        var domainMin: [Float] = [0, 0, 0], domainMax: [Float] = [1, 1, 1]
        for raw in text.split(whereSeparator: \.isNewline) {
            let line = raw.trimmingCharacters(in: .whitespaces)
            if line.isEmpty || line.hasPrefix("#") { continue }
            let parts = line.split(whereSeparator: { $0 == " " || $0 == "\t" })
            if line.hasPrefix("LUT_3D_SIZE") { size = Int(parts.last ?? "") ?? 0; continue }
            if line.hasPrefix("DOMAIN_MIN") { domainMin = parts.dropFirst().compactMap { Float($0) }; continue }
            if line.hasPrefix("DOMAIN_MAX") { domainMax = parts.dropFirst().compactMap { Float($0) }; continue }
            if line.hasPrefix("TITLE") || line.hasPrefix("LUT_1D") || line.first?.isLetter == true { continue }
            if parts.count >= 3, let r = Float(parts[0]), let g = Float(parts[1]), let b = Float(parts[2]) {
                vals += [r, g, b]
            }
        }
        guard size >= 2, vals.count == size * size * size * 3 else { throw ParseError() }
        if domainMin.count == 3 && domainMax.count == 3 && (domainMin != [0, 0, 0] || domainMax != [1, 1, 1]) {
            for i in stride(from: 0, to: vals.count, by: 3) {
                for c in 0..<3 { vals[i + c] = (vals[i + c] - domainMin[c]) / max(1e-6, domainMax[c] - domainMin[c]) }
            }
        }
        return LUTData(name: url.deletingPathExtension().lastPathComponent, size: size, values: vals)
    }
}

struct SeedRow: View {
    let value: Double
    let onChange: (Double) -> Void
    var body: some View {
        LabeledRow(label: "Seed") {
            Text("\(Int(value))").font(Theme.valueFont).foregroundStyle(Theme.textDim)
            Button {
                onChange(Double(Int.random(in: 1...9999)))
            } label: {
                Image(systemName: "dice")
            }
            .buttonStyle(PillButtonStyle())
            .help(tr("New random seed"))
        }
    }
}

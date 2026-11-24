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

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack {
                Menu {
                    Section(tr("Classic")) {
                        ForEach(PalettePresets.all, id: \.name) { p in Button(p.name) { onChange(p) } }
                    }
                    if !lib.palettes.isEmpty {
                        Section(tr("Saved")) { ForEach(lib.palettes, id: \.self) { p in Button(p.name) { onChange(p) } } }
                    }
                    Divider()
                    Button(tr("Save Palette…")) { save() }
                    Button(tr("Import Palettes…")) { if let p = lib.importPalettes() { onChange(p) } }
                    Button(tr("Export Palette…")) { lib.exportPalette(palette) }
                    if lib.palettes.contains(palette) {
                        Button(tr("Delete Saved Palette"), role: .destructive) { lib.deletePalette(palette) }
                    }
                } label: {
                    Text(palette.name).font(Theme.labelFont)
                }
                .menuStyle(.borderlessButton)
                .fixedSize()
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
                }
                if palette.colors.count < 64 {
                    RoundedRectangle(cornerRadius: 4)
                        .strokeBorder(Theme.textFaint, style: StrokeStyle(lineWidth: 1, dash: [3, 2]))
                        .aspectRatio(1, contentMode: .fit)
                        .overlay(Image(systemName: "plus").font(.system(size: 9, weight: .bold)).foregroundStyle(Theme.textDim))
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

struct LUTControl: View {
    let lut: LUTData?
    let onChange: (LUTData) -> Void
    var body: some View {
        HStack {
            Image(systemName: "cube").foregroundStyle(Theme.textDim)
            Text(lut?.name ?? tr("No LUT loaded"))
                .font(Theme.labelFont)
                .foregroundStyle(lut == nil ? Theme.textFaint : Theme.text)
                .lineLimit(1).truncationMode(.middle)
            Spacer()
            Button(tr("Load .cube…")) { load() }.buttonStyle(PillButtonStyle())
        }
    }

    private func load() {
        let p = NSOpenPanel()
        p.allowedContentTypes = [UTType(filenameExtension: "cube") ?? .data]
        guard p.runModal() == .OK, let url = p.url else { return }
        do { onChange(try CubeLUT.parse(url: url)) }
        catch { AppState.shared.alert(tr("Could not read LUT"), error.localizedDescription) }
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

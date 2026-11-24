import Foundation
import AppKit
import UniformTypeIdentifiers

enum GradientPresets {
    static let thermalIron = GradientSpec.from("Thermal · Iron", ["#000000", "#1B0A5C", "#6B0F8E", "#C02A5A", "#F0602A", "#FFB52E", "#FFF7B0"])
    static let groups: [(String, [GradientSpec])] = [
        ("Thermal", [
            thermalIron,
            .from("Thermal · Rainbow", ["#000010", "#1E0B8C", "#0060FF", "#00D8D0", "#3CE840", "#FFF000", "#FF7A00", "#FF0000", "#FFFFFF"]),
            .from("Thermal · White Hot", ["#000000", "#FFFFFF"]),
            .from("Thermal · Black Hot", ["#FFFFFF", "#000000"]),
            .from("Thermal · Arctic", ["#04071E", "#0C2A6B", "#1E78C8", "#7FD3F0", "#F4F2DC", "#F8B04C", "#E8502A"]),
            .from("Thermal · Lava", ["#000000", "#3A0000", "#9A0A00", "#FF4A00", "#FFB000", "#FFFFFF"]),
            .from("Thermal · Plasma", ["#0D0887", "#6A00A8", "#B12A90", "#E16462", "#FCA636", "#F0F921"]),
            .from("Thermal · Medical", ["#000000", "#0000FF", "#00FFFF", "#00FF00", "#FFFF00", "#FF0000", "#FFFFFF"]),
            .from("Night Vision", ["#000A00", "#0C3A0C", "#3CB43C", "#C8FFB4"]),
        ]),
        ("Duotone", [
            .from("Duotone · Riso", ["#1F2A6B", "#FF5A7A"]),
            .from("Duotone · Sunset", ["#2B0F3A", "#FF8A3D"]),
            .from("Duotone · Mint", ["#0E2A2A", "#9CF0C8"]),
            .from("Duotone · Yellow", ["#111111", "#FFBB00"]),
            .from("Duotone · Ultramarine", ["#0B0B3B", "#3C6BFF", "#E8EEFF"]),
            .from("Duotone · Bubblegum", ["#2A0F2F", "#FF9CDA"]),
            .from("Duotone · Lime", ["#10230F", "#C6FF3D"]),
            .from("Duotone · Coral", ["#1E1B3A", "#FF6F59"]),
            .from("Duotone · Teal & Orange", ["#0B3C49", "#FF9F43"]),
            .from("Duotone · Red Riso", ["#2A0A0A", "#FF3B30", "#FFF4EE"]),
        ]),
        ("Photographic Tone", [
            .from("Cyanotype", ["#0A1E3C", "#1F4F8C", "#E8F0F4"]),
            .from("Sepia", ["#1A0F08", "#704A2C", "#D8B888", "#F8F0E0"]),
            .from("Selenium", ["#0F0E12", "#4A4550", "#B8B2B5", "#F2EFEA"]),
            .from("Platinum", ["#1C1712", "#6E6254", "#CFC3AE", "#F7F1E6"]),
            .from("Tintype", ["#12100C", "#4B4232", "#A39470", "#E9DFC6"]),
            .from("Bleach", ["#0A0A0A", "#5A5F5C", "#CBD0C9", "#FFFFFF"]),
            .from("Receipt", ["#2E2B28", "#8F8A82", "#F3F0E8"]),
        ]),
        ("Retro & Y2K", [
            .from("Y2K Chrome", ["#0A0F2C", "#2448A8", "#7FB2F0", "#E6F0FF", "#FFFFFF"]),
            .from("Y2K Ice", ["#0B1E3F", "#3A7BD5", "#00D2FF", "#E0F7FF"]),
            .from("Frutiger Aero", ["#04395E", "#2A9FD6", "#7FE0A8", "#F2FFF0"]),
            .from("Holographic", ["#2B1055", "#7597DE", "#B8F2E6", "#FFC2E2", "#FFFFFF"]),
            .from("Vaporwave", ["#1A0033", "#7A1FA2", "#FF4FD8", "#4FE3FF", "#F4F4FF"]),
            .from("Synthwave", ["#0D0221", "#261447", "#FF3864", "#FFD319"]),
            .from("Gameboy", ["#0F380F", "#306230", "#8BAC0F", "#9BBC0F"]),
            .from("Matrix", ["#000000", "#003B00", "#00A12B", "#9BFF9B"]),
            .from("Dreamcore", ["#2D2A4A", "#8EC5FF", "#C9B6FF", "#FFD6E7", "#FFFDF5"]),
        ]),
        ("Nature", [
            .from("Fire", ["#000000", "#7A0000", "#FF3000", "#FFC000", "#FFFFE0"]),
            .from("Ocean", ["#000814", "#003566", "#0077B6", "#48CAE4", "#CAF0F8"]),
            .from("Forest", ["#0B1A0F", "#1F4D2B", "#6FA15A", "#E3F2C1"]),
            .from("Dusk", ["#120A26", "#4B2C69", "#D0587E", "#FFB38A", "#FFE8C8"]),
            .from("Golden Hour", ["#1A0E05", "#7A3B12", "#E08E3C", "#FFD89A", "#FFF6E5"]),
        ]),
    ]
    static let all: [GradientSpec] = groups.flatMap { $0.1 }
    static func group(of name: String) -> String? { groups.first { $0.1.contains { $0.name == name } }?.0 }
}

enum PalettePresets {
    static let oneBit = PaletteSpec.from("1-bit", ["#000000", "#FFFFFF"])
    static let groups: [(String, [PaletteSpec])] = [
        ("1-bit & Mono", [
            oneBit,
            .from("Mac Classic", ["#000000", "#FFFFFF"]),
            .from("Paper & Ink", ["#1B1B1B", "#EEE7D7"]),
            .from("Receipt", ["#2E2B28", "#F3F0E8"]),
            .from("Newsprint", ["#1C1C1C", "#EDE8DC"]),
            .from("Blueprint", ["#0D3B78", "#E6F0FF"]),
            .from("Amber Monitor", ["#1A0D00", "#FFB000"]),
            .from("Green Phosphor", ["#001A00", "#33FF33"]),
            .from("Yellow Effector", ["#0A0A0A", "#FFBB00"]),
            .from("Grayscale 4", ["#000000", "#555555", "#AAAAAA", "#FFFFFF"]),
            .from("Grayscale 8", ["#000000", "#242424", "#494949", "#6D6D6D", "#929292", "#B6B6B6", "#DBDBDB", "#FFFFFF"]),
        ]),
        ("Game Consoles", [
            .from("Game Boy", ["#0F380F", "#306230", "#8BAC0F", "#9BBC0F"]),
            .from("Game Boy Pocket", ["#000000", "#555555", "#AAAAAA", "#FFFFFF"]),
            .from("Virtual Boy", ["#000000", "#550000", "#AA0000", "#FF0000"]),
            .from("Ice Cream GB", ["#7C3F58", "#EB6B6F", "#F9A875", "#FFF6D3"]),
            .from("Kirokaze GB", ["#332C50", "#46878F", "#94E344", "#E2F3E4"]),
            .from("Mist GB", ["#2D1B00", "#1E606E", "#5AB9A8", "#C4F0C2"]),
            .from("Rustic GB", ["#2C2137", "#764462", "#A96868", "#EDB4A1"]),
            .from("Nostalgia", ["#405010", "#708028", "#A0A840", "#D0D058"]),
            .from("NES (subset)", ["#000000", "#FCFCFC", "#BCBCBC", "#7C7C7C", "#A4E4FC", "#3CBCFC", "#0078F8", "#0000FC",
                                  "#B8B8F8", "#6888FC", "#0058F8", "#F8B8F8", "#F878F8", "#D800CC", "#F8A4C0", "#F85898",
                                  "#F0D0B0", "#FCA044", "#E45C10", "#F8D878", "#F8B800", "#AC7C00", "#B8F8B8", "#58D854", "#00A800"]),
        ]),
        ("Computers", [
            .from("CGA 1", ["#000000", "#55FFFF", "#FF55FF", "#FFFFFF"]),
            .from("CGA 2", ["#000000", "#55FF55", "#FF5555", "#FFFF55"]),
            .from("EGA", ["#000000", "#0000AA", "#00AA00", "#00AAAA", "#AA0000", "#AA00AA", "#AA5500", "#AAAAAA",
                          "#555555", "#5555FF", "#55FF55", "#55FFFF", "#FF5555", "#FF55FF", "#FFFF55", "#FFFFFF"]),
            .from("Windows 16", ["#000000", "#800000", "#008000", "#808000", "#000080", "#800080", "#008080", "#C0C0C0",
                                 "#808080", "#FF0000", "#00FF00", "#FFFF00", "#0000FF", "#FF00FF", "#00FFFF", "#FFFFFF"]),
            .from("C64", ["#000000", "#FFFFFF", "#880000", "#AAFFEE", "#CC44CC", "#00CC55", "#0000AA", "#EEEE77",
                          "#DD8855", "#664400", "#FF7777", "#333333", "#777777", "#AAFF66", "#0088FF", "#BBBBBB"]),
            .from("ZX Spectrum", ["#000000", "#0000D7", "#D70000", "#D700D7", "#00D700", "#00D7D7", "#D7D700", "#D7D7D7"]),
            .from("Apple II", ["#000000", "#DD0033", "#000099", "#DD22DD", "#007722", "#555555", "#2222FF", "#66AAFF",
                               "#885500", "#FF6600", "#AAAAAA", "#FF9988", "#11DD00", "#FFFF00", "#44FF99", "#FFFFFF"]),
            .from("Teletext", ["#000000", "#FF0000", "#00FF00", "#FFFF00", "#0000FF", "#FF00FF", "#00FFFF", "#FFFFFF"]),
        ]),
        ("Pixel Art", [
            .from("PICO-8", ["#000000", "#1D2B53", "#7E2553", "#008751", "#AB5236", "#5F574F", "#C2C3C7", "#FFF1E8",
                             "#FF004D", "#FFA300", "#FFEC27", "#00E436", "#29ADFF", "#83769C", "#FF77A8", "#FFCCAA"]),
            .from("Sweetie 16", ["#1A1C2C", "#5D275D", "#B13E53", "#EF7D57", "#FFCD75", "#A7F070", "#38B764", "#257179",
                                 "#29366F", "#3B5DC9", "#41A6F6", "#73EFF7", "#F4F4F4", "#94B0C2", "#566C86", "#333C57"]),
            .from("Endesga 16", ["#E4A672", "#B86F50", "#743F39", "#3F2832", "#9E2835", "#E53B44", "#FB922B", "#FFE762",
                                 "#63C64D", "#327345", "#193D3F", "#4F6781", "#AFBFD2", "#FFFFFF", "#2CE8F4", "#0484D1"]),
            .from("Oil 6", ["#272744", "#494D7E", "#8B6D9C", "#C69FA5", "#F2D3AB", "#FBF5EF"]),
            .from("Hollow", ["#0F0F1B", "#565A75", "#C6B7BE", "#FAFBF6"]),
        ]),
        ("Print & Riso", [
            .from("Riso Blue/Pink", ["#F4F0E6", "#3255A4", "#FF48B0"]),
            .from("Riso Fluoro", ["#F4F0E6", "#FF48B0", "#FFE800", "#0078BF", "#1A1A1A"]),
            .from("Riso Teal/Orange", ["#F4F0E6", "#00838A", "#FF6C2F"]),
            .from("CMYK", ["#FFFFFF", "#00AEEF", "#EC008C", "#FFF200", "#231F20"]),
            .from("Xerox", ["#151515", "#8C8C8C", "#F0EFEA"]),
            .from("Thermal Iron", ["#000000", "#3A0D6B", "#A21E6A", "#F0602A", "#FFC83A", "#FFF7B0"]),
        ]),
        ("Moods", [
            .from("Y2K Blue", ["#050B26", "#1B3FAE", "#4A8CFF", "#B5D4FF", "#F0F6FF"]),
            .from("Frutiger Aero", ["#0E4C6E", "#3BA4D9", "#8FE3B0", "#F4FFF7"]),
            .from("Dreamcore", ["#2D2A4A", "#8EC5FF", "#C9B6FF", "#FFD6E7", "#F6E7FF"]),
            .from("Pastel", ["#FFD1DC", "#FFE5B4", "#FDFD96", "#C1E1C1", "#AEC6CF", "#CBAACB"]),
            .from("Earth", ["#2B1D14", "#5C4033", "#8B6B4A", "#C8A97E", "#EFE3C8"]),
            .from("Neon", ["#0A0A12", "#FF00A0", "#00F0FF", "#B6FF00", "#FFFFFF"]),
            .from("Cyberpunk", ["#0D0221", "#2DE2E6", "#F706CF", "#FD1D53", "#F9C80E"]),
        ]),
    ]
    static let all: [PaletteSpec] = groups.flatMap { $0.1 }
    static func group(of name: String) -> String? { groups.first { $0.1.contains { $0.name == name } }?.0 }

    /// K-means palette extraction from an image.
    static func extract(from cg: CGImage, count: Int) -> PaletteSpec {
        let size = 96
        let w = size, h = max(1, Int(Double(size) * Double(cg.height) / Double(cg.width)))
        let ctx = CGContext(data: nil, width: w, height: h, bitsPerComponent: 8, bytesPerRow: w * 4,
                            space: CGColorSpace(name: CGColorSpace.sRGB)!,
                            bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue)!
        ctx.interpolationQuality = .medium
        ctx.draw(cg, in: CGRect(x: 0, y: 0, width: w, height: h))
        let buf = Array(UnsafeBufferPointer(start: ctx.data!.assumingMemoryBound(to: UInt8.self), count: w * h * 4))
        var pts: [SIMD3<Double>] = []
        pts.reserveCapacity(w * h)
        for i in 0..<(w * h) {
            pts.append(SIMD3(Double(buf[i * 4]) / 255, Double(buf[i * 4 + 1]) / 255, Double(buf[i * 4 + 2]) / 255))
        }
        let k = max(2, min(32, count))
        // k-means++ init (deterministic)
        var rng = SplitMix(seed: 42)
        var centers: [SIMD3<Double>] = [pts[pts.count / 2]]
        while centers.count < k {
            var dists = pts.map { p in centers.map { simd_len2(p - $0) }.min()! }
            let total = dists.reduce(0, +)
            if total <= 0 { centers.append(pts[Int(rng.next() % UInt64(pts.count))]); continue }
            var r = Double(rng.next() % 1_000_000) / 1_000_000 * total
            var idx = 0
            for (i, d) in dists.enumerated() { r -= d; if r <= 0 { idx = i; break } }
            centers.append(pts[idx])
            dists.removeAll()
        }
        for _ in 0..<12 {
            var sums = [SIMD3<Double>](repeating: .zero, count: k)
            var counts = [Int](repeating: 0, count: k)
            for p in pts {
                var best = 0; var bd = Double.infinity
                for (j, c) in centers.enumerated() { let d = simd_len2(p - c); if d < bd { bd = d; best = j } }
                sums[best] += p; counts[best] += 1
            }
            for j in 0..<k where counts[j] > 0 { centers[j] = sums[j] / Double(counts[j]) }
        }
        centers.sort { ($0.x * 0.2126 + $0.y * 0.7152 + $0.z * 0.0722) < ($1.x * 0.2126 + $1.y * 0.7152 + $1.z * 0.0722) }
        return PaletteSpec(name: tr("Extracted"), colors: centers.map { RGBA(r: $0.x, g: $0.y, b: $0.z) })
    }
}

private func simd_len2(_ v: SIMD3<Double>) -> Double { (v * v).sum() }

struct SplitMix {
    var state: UInt64
    init(seed: UInt64) { state = seed }
    mutating func next() -> UInt64 {
        state &+= 0x9E3779B97F4A7C15
        var z = state
        z = (z ^ (z >> 30)) &* 0xBF58476D1CE4E5B9
        z = (z ^ (z >> 27)) &* 0x94D049BB133111EB
        return z ^ (z >> 31)
    }
    mutating func unit() -> Double { Double(next() % 10_000_000) / 10_000_000 }
}

/// User-saved palettes and gradients.
final class SwatchLibrary: ObservableObject {
    static let shared = SwatchLibrary()
    @Published private(set) var palettes: [PaletteSpec] = []
    @Published private(set) var gradients: [GradientSpec] = []

    private struct Store: Codable { var palettes: [PaletteSpec]; var gradients: [GradientSpec] }

    private var file: URL {
        let base = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0]
        let d = base.appendingPathComponent("EFFECTOR", isDirectory: true)
        try? FileManager.default.createDirectory(at: d, withIntermediateDirectories: true)
        return d.appendingPathComponent("swatches.json")
    }

    private init() {
        if let d = try? Data(contentsOf: file), let s = try? JSONDecoder().decode(Store.self, from: d) {
            palettes = s.palettes; gradients = s.gradients
        }
    }

    private func persist() {
        if let d = try? JSONEncoder().encode(Store(palettes: palettes, gradients: gradients)) { try? d.write(to: file, options: .atomic) }
    }

    func savePalette(_ p: PaletteSpec) { palettes.removeAll { $0.name == p.name }; palettes.insert(p, at: 0); persist() }
    func deletePalette(_ p: PaletteSpec) { palettes.removeAll { $0 == p }; persist() }
    func saveGradient(_ g: GradientSpec) { gradients.removeAll { $0.name == g.name }; gradients.insert(g, at: 0); persist() }
    func deleteGradient(_ g: GradientSpec) { gradients.removeAll { $0 == g }; persist() }

    // MARK: Palette files (.hex, .gpl, .txt, .json)

    func exportPalette(_ p: PaletteSpec) {
        let panel = NSSavePanel()
        panel.allowedContentTypes = [UTType(filenameExtension: "hex") ?? .plainText, UTType(filenameExtension: "gpl") ?? .plainText]
        panel.nameFieldStringValue = p.name + ".hex"
        guard panel.runModal() == .OK, let url = panel.url else { return }
        let text: String
        if url.pathExtension.lowercased() == "gpl" {
            text = "GIMP Palette\nName: \(p.name)\nColumns: 8\n#\n" + p.colors.map {
                String(format: "%3d %3d %3d\t%@", Int($0.r * 255), Int($0.g * 255), Int($0.b * 255), $0.hex)
            }.joined(separator: "\n") + "\n"
        } else {
            text = p.colors.map { String($0.hex.dropFirst()) }.joined(separator: "\n") + "\n"
        }
        try? text.write(to: url, atomically: true, encoding: .utf8)
    }

    func importPalettes() -> PaletteSpec? {
        let panel = NSOpenPanel()
        panel.allowsMultipleSelection = true
        panel.allowedContentTypes = ["hex", "gpl", "txt", "json"].compactMap { UTType(filenameExtension: $0) }
        guard panel.runModal() == .OK else { return nil }
        var last: PaletteSpec?
        for url in panel.urls {
            guard let text = try? String(contentsOf: url, encoding: .utf8) else { continue }
            let name = url.deletingPathExtension().lastPathComponent
            var colors: [RGBA] = []
            if url.pathExtension.lowercased() == "json", let d = text.data(using: .utf8),
               let p = try? JSONDecoder().decode(PaletteSpec.self, from: d) {
                colors = p.colors
            } else {
                for line in text.split(whereSeparator: \.isNewline) {
                    let l = line.trimmingCharacters(in: .whitespaces)
                    if l.isEmpty || l.hasPrefix("GIMP") || l.hasPrefix("Name") || l.hasPrefix("Columns") || l.hasPrefix(";") { continue }
                    if l.hasPrefix("#") && l.count < 7 { continue }
                    let parts = l.split(whereSeparator: { $0 == " " || $0 == "\t" }).map(String.init)
                    if parts.count >= 3, let r = Int(parts[0]), let g = Int(parts[1]), let b = Int(parts[2]) {
                        colors.append(RGBA(r: Double(r) / 255, g: Double(g) / 255, b: Double(b) / 255))
                    } else if let first = parts.first {
                        let hex = first.hasPrefix("#") ? String(first.dropFirst()) : first
                        if hex.count == 6 || hex.count == 8, hex.allSatisfy(\.isHexDigit) {
                            colors.append(RGBA(hex: String(hex.suffix(6))))
                        }
                    }
                }
            }
            if !colors.isEmpty {
                let p = PaletteSpec(name: name, colors: Array(colors.prefix(64)))
                savePalette(p)
                last = p
            }
        }
        return last
    }

    // MARK: Gradient files (.json)

    func exportGradient(_ g: GradientSpec) {
        let panel = NSSavePanel()
        panel.allowedContentTypes = [.json]
        panel.nameFieldStringValue = g.name + ".effectorgradient.json"
        guard panel.runModal() == .OK, let url = panel.url else { return }
        let enc = JSONEncoder(); enc.outputFormatting = .prettyPrinted
        try? enc.encode([g]).write(to: url)
    }

    func importGradients() {
        let panel = NSOpenPanel()
        panel.allowsMultipleSelection = true
        panel.allowedContentTypes = [.json]
        guard panel.runModal() == .OK else { return }
        for url in panel.urls {
            guard let d = try? Data(contentsOf: url) else { continue }
            if let gs = try? JSONDecoder().decode([GradientSpec].self, from: d) { gs.forEach(saveGradient) }
            else if let g = try? JSONDecoder().decode(GradientSpec.self, from: d) { saveGradient(g) }
        }
    }
}

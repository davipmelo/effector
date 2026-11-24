import Foundation
import AppKit
import UniformTypeIdentifiers

enum GradientPresets {
    static let thermalIron = GradientSpec.from("Thermal · Iron", ["#000000", "#1B0A5C", "#6B0F8E", "#C02A5A", "#F0602A", "#FFB52E", "#FFF7B0"])
    static let all: [GradientSpec] = [
        thermalIron,
        .from("Thermal · Rainbow", ["#000010", "#1E0B8C", "#0060FF", "#00D8D0", "#3CE840", "#FFF000", "#FF7A00", "#FF0000", "#FFFFFF"]),
        .from("Thermal · White Hot", ["#000000", "#FFFFFF"]),
        .from("Thermal · Black Hot", ["#FFFFFF", "#000000"]),
        .from("Thermal · Arctic", ["#04071E", "#0C2A6B", "#1E78C8", "#7FD3F0", "#F4F2DC", "#F8B04C", "#E8502A"]),
        .from("Thermal · Lava", ["#000000", "#3A0000", "#9A0A00", "#FF4A00", "#FFB000", "#FFFFFF"]),
        .from("Night Vision", ["#000A00", "#0C3A0C", "#3CB43C", "#C8FFB4"]),
        .from("Duotone · Riso", ["#1F2A6B", "#FF5A7A"]),
        .from("Duotone · Sunset", ["#2B0F3A", "#FF8A3D"]),
        .from("Duotone · Mint", ["#0E2A2A", "#9CF0C8"]),
        .from("Duotone · Yellow", ["#111111", "#FFBB00"]),
        .from("Cyanotype", ["#0A1E3C", "#1F4F8C", "#E8F0F4"]),
        .from("Sepia", ["#1A0F08", "#704A2C", "#D8B888", "#F8F0E0"]),
        .from("Vaporwave", ["#1A0033", "#7A1FA2", "#FF4FD8", "#4FE3FF", "#F4F4FF"]),
        .from("Gameboy", ["#0F380F", "#306230", "#8BAC0F", "#9BBC0F"]),
        .from("Fire", ["#000000", "#7A0000", "#FF3000", "#FFC000", "#FFFFE0"]),
        .from("Ocean", ["#000814", "#003566", "#0077B6", "#48CAE4", "#CAF0F8"]),
    ]
}

enum PalettePresets {
    static let oneBit = PaletteSpec.from("1-bit", ["#000000", "#FFFFFF"])
    static let all: [PaletteSpec] = [
        oneBit,
        .from("Game Boy", ["#0F380F", "#306230", "#8BAC0F", "#9BBC0F"]),
        .from("Game Boy Pocket", ["#000000", "#555555", "#AAAAAA", "#FFFFFF"]),
        .from("Mac Classic", ["#000000", "#FFFFFF"]),
        .from("CGA 1", ["#000000", "#55FFFF", "#FF55FF", "#FFFFFF"]),
        .from("CGA 2", ["#000000", "#55FF55", "#FF5555", "#FFFF55"]),
        .from("EGA", ["#000000", "#0000AA", "#00AA00", "#00AAAA", "#AA0000", "#AA00AA", "#AA5500", "#AAAAAA",
                      "#555555", "#5555FF", "#55FF55", "#55FFFF", "#FF5555", "#FF55FF", "#FFFF55", "#FFFFFF"]),
        .from("PICO-8", ["#000000", "#1D2B53", "#7E2553", "#008751", "#AB5236", "#5F574F", "#C2C3C7", "#FFF1E8",
                         "#FF004D", "#FFA300", "#FFEC27", "#00E436", "#29ADFF", "#83769C", "#FF77A8", "#FFCCAA"]),
        .from("C64", ["#000000", "#FFFFFF", "#880000", "#AAFFEE", "#CC44CC", "#00CC55", "#0000AA", "#EEEE77",
                      "#DD8855", "#664400", "#FF7777", "#333333", "#777777", "#AAFF66", "#0088FF", "#BBBBBB"]),
        .from("ZX Spectrum", ["#000000", "#0000D7", "#D70000", "#D700D7", "#00D700", "#00D7D7", "#D7D700", "#D7D7D7"]),
        .from("NES (subset)", ["#000000", "#FCFCFC", "#BCBCBC", "#7C7C7C", "#A4E4FC", "#3CBCFC", "#0078F8", "#0000FC",
                              "#B8B8F8", "#6888FC", "#0058F8", "#F8B8F8", "#F878F8", "#D800CC", "#F8A4C0", "#F85898",
                              "#F0D0B0", "#FCA044", "#E45C10", "#F8D878", "#F8B800", "#AC7C00", "#B8F8B8", "#58D854", "#00A800"]),
        .from("Grayscale 4", ["#000000", "#555555", "#AAAAAA", "#FFFFFF"]),
        .from("Grayscale 8", ["#000000", "#242424", "#494949", "#6D6D6D", "#929292", "#B6B6B6", "#DBDBDB", "#FFFFFF"]),
        .from("Amber Monitor", ["#1A0D00", "#FFB000"]),
        .from("Green Phosphor", ["#001A00", "#33FF33"]),
        .from("Riso Blue/Pink", ["#F4F0E6", "#3255A4", "#FF48B0"]),
        .from("Newsprint", ["#1C1C1C", "#EDE8DC"]),
        .from("Thermal Iron", ["#000000", "#3A0D6B", "#A21E6A", "#F0602A", "#FFC83A", "#FFF7B0"]),
        .from("Yellow Effector", ["#0A0A0A", "#FFBB00"]),
    ]

    /// K-means palette extraction from an image.
    static func extract(from cg: CGImage, count: Int) -> PaletteSpec {
        let size = 96
        let w = size, h = max(1, Int(Double(size) * Double(cg.height) / Double(cg.width)))
        var buf = [UInt8](repeating: 0, count: w * h * 4)
        let ctx = CGContext(data: &buf, width: w, height: h, bitsPerComponent: 8, bytesPerRow: w * 4,
                            space: CGColorSpace(name: CGColorSpace.sRGB)!,
                            bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue)!
        ctx.interpolationQuality = .medium
        ctx.draw(cg, in: CGRect(x: 0, y: 0, width: w, height: h))
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

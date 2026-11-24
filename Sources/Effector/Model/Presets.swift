import Foundation
import AppKit

struct Preset: Codable, Identifiable, Hashable {
    var id = UUID()
    var name: String
    var layers: [LayerNode]
    var isFactory = false
    var thumbnail: Data? = nil

    static let fileVersion = 1
}

private struct PresetFile: Codable {
    var version = Preset.fileVersion
    var app = "EFFECTOR"
    var presets: [Preset]
}

/// Stores user presets on disk and provides factory presets.
final class PresetStore: ObservableObject {
    static let shared = PresetStore()

    @Published private(set) var user: [Preset] = []
    @Published var factoryVisible = true
    @Published var thumbs: [UUID: CGImage] = [:]

    var all: [Preset] { (factoryVisible ? FactoryPresets.all : []) + user }

    private var dir: URL {
        let base = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0]
        let d = base.appendingPathComponent("EFFECTOR", isDirectory: true)
        try? FileManager.default.createDirectory(at: d, withIntermediateDirectories: true)
        return d
    }
    private var file: URL { dir.appendingPathComponent("presets.json") }

    private init() {
        factoryVisible = UserDefaults.standard.object(forKey: "factoryPresetsVisible") as? Bool ?? true
        load()
    }

    private func load() {
        guard let d = try? Data(contentsOf: file), let f = try? JSONDecoder().decode(PresetFile.self, from: d) else { return }
        user = f.presets
    }

    private func persist() {
        let enc = JSONEncoder()
        if let d = try? enc.encode(PresetFile(presets: user)) { try? d.write(to: file, options: .atomic) }
    }

    func setFactoryVisible(_ v: Bool) {
        factoryVisible = v
        UserDefaults.standard.set(v, forKey: "factoryPresetsVisible")
    }

    func add(name: String, layers: [LayerNode], thumbnail: CGImage?) {
        var p = Preset(name: name, layers: layers.map { $0.reidentified() })
        if let t = thumbnail { p.thumbnail = NSBitmapImageRep(cgImage: t).representation(using: .jpeg, properties: [.compressionFactor: 0.8]) }
        user.insert(p, at: 0)
        persist()
    }

    func rename(_ id: UUID, _ name: String) {
        if let i = user.firstIndex(where: { $0.id == id }) { user[i].name = name; persist() }
    }

    func delete(_ id: UUID) {
        user.removeAll { $0.id == id }
        persist()
    }

    // MARK: Import / export

    func export(_ presets: [Preset], to url: URL) throws {
        let enc = JSONEncoder()
        enc.outputFormatting = [.prettyPrinted]
        let ps = presets.map { p -> Preset in var p = p; p.isFactory = false; return p }
        try enc.encode(PresetFile(presets: ps)).write(to: url, options: .atomic)
    }

    @discardableResult
    func importFile(_ url: URL) throws -> Int {
        let d = try Data(contentsOf: url)
        let f = try JSONDecoder().decode(PresetFile.self, from: d)
        let imported = f.presets.map { p -> Preset in
            var p = p; p.id = UUID(); p.isFactory = false
            p.layers = p.layers.map { $0.reidentified() }
            return p
        }
        user.insert(contentsOf: imported, at: 0)
        persist()
        return imported.count
    }

    func exportPanel(_ presets: [Preset]) {
        let p = NSSavePanel()
        p.allowedContentTypes = [.effectorPreset]
        p.nameFieldStringValue = (presets.count == 1 ? presets[0].name : "EFFECTOR Presets") + ".effectorpreset"
        guard p.runModal() == .OK, let url = p.url else { return }
        do { try export(presets, to: url) } catch { AppState.shared.alert(tr("Export failed"), error.localizedDescription) }
    }

    func importPanel() {
        let p = NSOpenPanel()
        p.allowedContentTypes = [.effectorPreset, .json]
        p.allowsMultipleSelection = true
        guard p.runModal() == .OK else { return }
        var n = 0
        for u in p.urls {
            do { n += try importFile(u) } catch { AppState.shared.alert(tr("Could not import presets"), error.localizedDescription) }
        }
        if n > 0 { AppState.shared.flash(tr("%d presets imported", n)) }
    }

    /// Renders thumbnails of all presets on the current image.
    func regenerateThumbs() {
        let app = AppState.shared
        guard app.hasImage else { return }
        let list = all
        DispatchQueue.global(qos: .utility).async {
            var out: [UUID: CGImage] = [:]
            for p in list {
                if let cg = app.renderThumbnail(layers: p.layers, size: 180) { out[p.id] = cg }
            }
            DispatchQueue.main.async { self.thumbs = out }
        }
    }
}

enum FactoryPresets {
    /// Builds an effect layer with overrides. Values: Double/Int, Bool, String (choice), RGBA,
    /// PaletteSpec, GradientSpec, or `.text("...")` via ParamValue.
    static func fx(_ id: String, name: String? = nil, opacity: Double = 1, blend: BlendMode = .normal,
                   _ values: [String: Any] = [:]) -> LayerNode {
        var p = Params()
        for (k, v) in values {
            switch v {
            case let d as Double: p[k] = .number(d)
            case let i as Int: p[k] = .number(Double(i))
            case let b as Bool: p[k] = .bool(b)
            case let s as String: p[k] = .choice(s)
            case let c as RGBA: p[k] = .color(c)
            case let pal as PaletteSpec: p[k] = .palette(pal)
            case let g as GradientSpec: p[k] = .gradient(g)
            case let pv as ParamValue: p[k] = pv
            default: break
            }
        }
        var n = LayerNode.effect(id, params: p)
        if let name { n.name = name }
        n.opacity = opacity
        n.blend = blend
        return n
    }

    static func palette(_ name: String) -> PaletteSpec { PalettePresets.all.first { $0.name == name } ?? PalettePresets.oneBit }
    static func gradient(_ name: String) -> GradientSpec { GradientPresets.all.first { $0.name == name } ?? GradientPresets.thermalIron }

    private static func make(_ name: String, _ layersBottomFirst: [LayerNode]) -> Preset {
        Preset(name: name, layers: layersBottomFirst.reversed(), isFactory: true)
    }

    static let all: [Preset] = [
        make("Portra Glow", [
            fx("film", ["stock": "portra400", "grain": 30.0, "halation": 35.0]),
            fx("bloom", opacity: 0.6, ["threshold": 72.0, "intensity": 70.0, "radius": 40.0]),
        ]),
        make("Tri-X Push", [
            fx("basic", ["contrast": 18.0, "blacks": -15.0]),
            fx("film", ["stock": "trix400", "grain": 60.0]),
        ]),
        make("CineStill Night", [
            fx("film", ["stock": "cinestill800t", "halation": 85.0, "grain": 40.0]),
            fx("bloom", opacity: 0.7, ["threshold": 78.0, "intensity": 90.0, "radius": 25.0]),
        ]),
        make("Golden Summer", [
            fx("film", ["stock": "gold200", "warmth": 25.0, "grain": 35.0]),
            fx("glow", opacity: 0.5, ["intensity": 50.0, "radius": 30.0, "threshold": 55.0]),
        ]),
        make("Velvia Punch", [
            fx("film", ["stock": "velvia50", "grain": 15.0]),
            fx("basic", ["vibrance": 15.0]),
        ]),
        make("Silver Grain", [
            fx("film", ["stock": "delta3200", "grain": 38.0, "grainSize": 1.2]),
            fx("basic", ["contrast": 25.0]),
        ]),
        make("VHS 1994", [
            fx("basic", ["saturation": -10.0, "contrast": -8.0]),
            fx("vhs"),
        ]),
        make("Thermal Cam", [
            fx("blur", ["radius": 3.0]),
            fx("gradientmap", ["gradient": gradient("Thermal · Iron")]),
            fx("blob", ["detect": "brightness", "threshold": 72.0, "shape": "cross", "label": "coords", "connect": "none",
                        "color": RGBA(hex: "#FFFFFF"), "maxBlobs": 6.0]),
        ]),
        make("Surveillance", [
            fx("bw", ["contrast": 15.0]),
            fx("grain", ["amount": 45.0]),
            fx("blob", ["detect": "contours", "shape": "corners", "label": "hex", "connect": "nearest",
                        "color": RGBA(hex: "#FFBB00"), "dimOutside": 20.0]),
            fx("vhs", ["tracking": false, "chroma": false, "wobble": false, "soft": false, "dropouts": false,
                       "osdTop": ParamValue.text("● REC  CAM 04"), "osdBottom": ParamValue.text("2026-11-24  03:14:07")]),
        ]),
        make("Game Boy", [
            fx("dither", ["algorithm": "bayer4", "mode": "palette", "palette": palette("Game Boy"), "pixelSize": 4.0, "contrast": 15.0]),
        ]),
        make("Mac Classic", [
            fx("dither", ["algorithm": "atkinson", "pixelSize": 2.0, "dark": RGBA(hex: "#000000"), "light": RGBA(hex: "#FFFFFF")]),
        ]),
        make("PICO-8 Pixels", [
            fx("dither", ["algorithm": "floyd", "mode": "palette", "palette": palette("PICO-8"), "pixelSize": 5.0, "strength": 70.0]),
        ]),
        make("Riso Duo", [
            fx("gradientmap", ["gradient": gradient("Duotone · Riso")]),
            fx("halftone", opacity: 0.55, blend: .multiply, ["mode": "bw", "cell": 5.0, "angle": 22.0]),
            fx("grain", ["amount": 30.0]),
        ]),
        make("CMYK Newsprint", [
            fx("halftone", ["mode": "cmyk", "cell": 7.0, "paper": RGBA(hex: "#F1EEE6")]),
            fx("grain", ["amount": 20.0]),
        ]),
        make("ASCII Terminal", [
            fx("ascii", ["charset": "standard", "cell": 7.0, "fg": RGBA(hex: "#5CFF7A"), "bg": RGBA(hex: "#04110A")]),
            fx("glow", ["intensity": 80.0, "radius": 12.0, "threshold": 10.0]),
        ]),
        make("Matrix Rain", [
            fx("ascii", ["charset": "matrix", "cell": 9.0, "fg": RGBA(hex: "#3BFF6E"), "bg": RGBA(hex: "#000000"), "contrast": 45.0, "brightness": -20.0]),
            fx("bloom", ["threshold": 40.0, "intensity": 120.0, "radius": 15.0]),
        ]),
        make("Dreamy Bloom", [
            fx("basic", ["highlights": -20.0, "shadows": 15.0]),
            fx("bloom", ["threshold": 55.0, "intensity": 150.0, "radius": 60.0]),
            fx("grain", ["amount": 20.0]),
        ]),
        make("Ghost Trail", [
            fx("trail", ["distance": 90.0, "copies": 10.0, "blend": "screen", "rgbShift": 40.0]),
            fx("glow", opacity: 0.6, ["intensity": 60.0]),
        ]),
    ]
}

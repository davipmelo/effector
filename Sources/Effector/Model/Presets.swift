import Foundation
import AppKit

struct Preset: Codable, Identifiable, Hashable {
    var id = UUID()
    var name: String
    var layers: [LayerNode]
    var isFactory = false
    var thumbnail: Data? = nil
    /// Section in the presets panel (factory presets only).
    var category: String? = nil

    /// Factory preset names are translated; user names are shown as typed.
    var displayName: String { isFactory ? tr(name) : name }

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

/// Converts a literal (Double/Int/Bool/String/RGBA/PaletteSpec/GradientSpec/ParamValue) to a ParamValue.
func paramValue(_ v: Any) -> ParamValue? {
    switch v {
    case let d as Double: return .number(d)
    case let i as Int: return .number(Double(i))
    case let b as Bool: return .bool(b)
    case let s as String: return .choice(s)
    case let c as RGBA: return .color(c)
    case let pal as PaletteSpec: return .palette(pal)
    case let g as GradientSpec: return .gradient(g)
    case let pv as ParamValue: return pv
    default: return nil
    }
}

enum FactoryPresets {
    /// Builds an effect layer with overrides. Values: Double/Int, Bool, String (choice), RGBA,
    /// PaletteSpec, GradientSpec, or `.text("...")` via ParamValue.
    static func fx(_ id: String, name: String? = nil, opacity: Double = 1, blend: BlendMode = .normal,
                   _ values: [String: Any] = [:]) -> LayerNode {
        var p = Params()
        for (k, v) in values { if let pv = paramValue(v) { p[k] = pv } }
        var n = LayerNode.effect(id, params: p)
        if let name { n.name = name }
        n.opacity = opacity
        n.blend = blend
        return n
    }

    static func palette(_ name: String) -> PaletteSpec { PalettePresets.all.first { $0.name == name } ?? PalettePresets.oneBit }
    static func gradient(_ name: String) -> GradientSpec { GradientPresets.all.first { $0.name == name } ?? GradientPresets.thermalIron }

    /// An effect layer starting from one of the effect's Looks, with optional overrides.
    static func look(_ id: String, _ lookID: String, name: String? = nil, opacity: Double = 1, blend: BlendMode = .normal,
                     _ extra: [String: Any] = [:]) -> LayerNode {
        var vals = EffectRegistry.effect(id)?.looks.first { $0.id == lookID }?.values ?? [:]
        for (k, v) in extra { vals[k] = v }
        vals["_look"] = ParamValue.text(lookID)
        return fx(id, name: name, opacity: opacity, blend: blend, vals)
    }

    static let categories = ["Film", "Y2K & Digital", "Analog Video", "Print & Paper", "Dither & Pixel", "Light & Mood", "Tracking"]

    private static func make(_ name: String, _ category: String, _ layersBottomFirst: [LayerNode]) -> Preset {
        Preset(name: name, layers: layersBottomFirst.reversed(), isFactory: true, category: category)
    }

    static let all: [Preset] = [
        // MARK: Film
        make("Portra Glow", "Film", [
            fx("film", ["stock": "portra400", "grain": 30.0, "halation": 35.0]),
            fx("bloom", opacity: 0.6, ["threshold": 72.0, "intensity": 70.0, "radius": 40.0]),
        ]),
        make("Golden Summer", "Film", [
            fx("film", ["stock": "gold200", "warmth": 25.0, "grain": 35.0]),
            fx("glow", opacity: 0.5, ["intensity": 50.0, "radius": 30.0, "threshold": 55.0]),
        ]),
        make("Disposable Camera", "Film", [
            fx("film", ["stock": "ultramax400", "grain": 55.0, "exposure": 0.25]),
            fx("vignette", ["amount": 40.0, "midpoint": 45.0]),
            fx("chromab", ["amount": 3.0]),
            fx("leak", opacity: 0.6, ["edge": "right", "size": 30.0, "intensity": 70.0]),
        ]),
        make("Expired Film", "Film", [
            fx("film", ["stock": "expired", "grain": 60.0, "fade": 30.0]),
            fx("leak", opacity: 0.8, ["edge": "tl", "colorA": RGBA(hex: "#FF1E6E"), "colorB": RGBA(hex: "#FF8A3D"), "size": 40.0]),
            look("dust", "scan", ["amount": 25.0]),
        ]),
        make("CineStill Night", "Film", [
            fx("film", ["stock": "cinestill800t", "halation": 85.0, "grain": 40.0]),
            fx("bloom", opacity: 0.7, ["threshold": 78.0, "intensity": 90.0, "radius": 25.0]),
        ]),
        make("Velvia Punch", "Film", [
            fx("film", ["stock": "velvia50", "grain": 15.0]),
            fx("basic", ["vibrance": 15.0]),
        ]),
        make("Kodachrome Slide", "Film", [
            fx("film", ["stock": "kodachrome64", "grain": 20.0]),
            fx("frame", ["style": "slide"]),
        ]),
        make("Polaroid", "Film", [
            fx("film", ["stock": "polaroid600", "grain": 20.0]),
            fx("vignette", ["amount": 25.0]),
            fx("frame", ["style": "polaroid"]),
        ]),
        make("Instax Party", "Film", [
            fx("film", ["stock": "instax", "exposure": 0.3]),
            fx("bloom", opacity: 0.5, ["threshold": 70.0, "radius": 30.0]),
            fx("frame", ["style": "instax"]),
        ]),
        make("Cross Process", "Film", [
            fx("film", ["stock": "crossprocess", "grain": 40.0]),
            fx("vignette", ["amount": 35.0]),
        ]),
        make("LomoChrome Purple", "Film", [
            fx("film", ["stock": "lomopurple", "grain": 45.0]),
        ]),
        make("Infrared", "Film", [
            fx("film", ["stock": "aerochrome", "grain": 35.0]),
            fx("glow", opacity: 0.5, ["intensity": 40.0, "radius": 25.0]),
        ]),
        make("Bleach Bypass", "Film", [
            fx("film", ["stock": "bleach", "grain": 30.0]),
            fx("sharpen", ["amount": 60.0]),
        ]),
        make("Tri-X Push", "Film", [
            fx("basic", ["contrast": 18.0, "blacks": -15.0]),
            fx("film", ["stock": "trix400", "grain": 60.0]),
        ]),
        make("Silver Grain", "Film", [
            fx("film", ["stock": "delta3200", "grain": 38.0, "grainSize": 1.2]),
            fx("basic", ["contrast": 25.0]),
        ]),
        make("Contact Sheet", "Film", [
            fx("film", ["stock": "hp5", "grain": 45.0]),
            fx("frame", ["style": "film", "edgeText": ParamValue.text("ILFORD HP5 PLUS")]),
        ]),

        // MARK: Y2K & digital
        make("Digicam 2003", "Y2K & Digital", [
            look("digicam", "cybershot"),
        ]),
        make("Party Flash", "Y2K & Digital", [
            look("digicam", "flash"),
            fx("vignette", ["amount": 30.0]),
        ]),
        make("Y2K Blue", "Y2K & Digital", [
            fx("basic", ["exposure": 0.25, "contrast": -10.0]),
            fx("gradientmap", opacity: 0.55, blend: .color, ["gradient": gradient("Y2K Ice")]),
            fx("bloom", ["threshold": 75.0, "intensity": 60.0, "radius": 45.0, "tint": RGBA(hex: "#BFD8FF")]),
            fx("chromab", ["amount": 4.0]),
            fx("grain", ["amount": 25.0]),
        ]),
        make("Chrome Dream", "Y2K & Digital", [
            fx("gradientmap", opacity: 0.85, ["gradient": gradient("Y2K Chrome")]),
            fx("sharpen", ["amount": 150.0, "radius": 2.0]),
            fx("bloom", ["threshold": 88.0, "intensity": 60.0, "radius": 20.0]),
        ]),
        make("Frutiger Aero", "Y2K & Digital", [
            fx("basic", ["exposure": 0.2, "vibrance": 40.0, "highlights": -20.0]),
            fx("gradientmap", opacity: 0.35, blend: .softLight, ["gradient": gradient("Frutiger Aero")]),
            fx("bloom", ["threshold": 65.0, "intensity": 90.0, "radius": 40.0]),
            fx("plastic", opacity: 0.6, ["intensity": 40.0]),
        ]),
        make("Dreamcore", "Y2K & Digital", [
            fx("gradientmap", opacity: 0.5, blend: .softLight, ["gradient": gradient("Dreamcore")]),
            fx("bloom", ["threshold": 68.0, "intensity": 80.0, "radius": 70.0]),
            fx("grain", ["amount": 30.0]),
            fx("vignette", ["amount": -30.0]),
        ]),
        make("Flip Phone", "Y2K & Digital", [
            look("digicam", "phone"),
        ]),
        make("Webcam 2002", "Y2K & Digital", [
            look("digicam", "webcam"),
            fx("vignette", ["amount": 45.0]),
        ]),
        make("Deep Fried", "Y2K & Digital", [
            fx("basic", ["saturation": 80.0, "contrast": 60.0]),
            fx("sharpen", ["amount": 400.0, "radius": 2.5]),
            fx("jpeg", ["quality": 4.0, "generations": 4, "block": 3.0]),
        ]),

        // MARK: Analog video
        make("VHS 1994", "Analog Video", [look("vhs", "rental94")]),
        make("Home Video '86", "Analog Video", [look("vhs", "home86")]),
        make("Camcorder '98", "Analog Video", [look("vhs", "camcorder98")]),
        make("Music Video '89", "Analog Video", [
            look("vhs", "music89"),
            fx("glow", opacity: 0.6, ["intensity": 60.0, "radius": 25.0]),
        ]),
        make("Dubbed Tape", "Analog Video", [look("vhs", "dubbed")]),
        make("MiniDV 2003", "Analog Video", [look("vhs", "minidv")]),
        make("CRT Monitor", "Analog Video", [
            look("crt", "pc"),
        ]),
        make("Arcade Screen", "Analog Video", [
            fx("dither", ["algorithm": "bayer4", "mode": "palette", "palette": palette("Sweetie 16"), "pixelSize": 3.0]),
            look("crt", "arcade"),
        ]),
        make("Surveillance", "Analog Video", [
            fx("bw", ["contrast": 15.0]),
            fx("grain", ["amount": 45.0]),
            fx("blob", ["detect": "contours", "shape": "corners", "label": "hex", "connect": "nearest",
                        "color": RGBA(hex: "#FFBB00"), "dimOutside": 20.0]),
            look("vhs", "cctv"),
        ]),

        // MARK: Print & paper
        make("Receipt", "Print & Paper", [
            fx("receipt"),
        ]),
        make("Receipt Scan", "Print & Paper", [
            fx("receipt", ["fade": 35.0, "dead": 30.0]),
            look("paper", "receipt"),
            look("dust", "subtle"),
            fx("vignette", ["amount": 20.0, "color": RGBA(hex: "#000000")]),
        ]),
        make("Xerox", "Print & Paper", [
            fx("xerox"),
        ]),
        make("Zine Copy", "Print & Paper", [
            fx("xerox", ["threshold": 58.0, "dither": true, "generations": 2]),
            look("paper", "fine"),
        ]),
        make("Fax", "Print & Paper", [
            fx("dither", ["algorithm": "atkinson", "pixelSize": 2.0, "contrast": 30.0, "dark": RGBA(hex: "#101010"), "light": RGBA(hex: "#F4F4F4")]),
            fx("xerox", opacity: 0.5, blend: .multiply, ["threshold": 70.0, "edge": 20.0, "streaks": 60.0]),
        ]),
        make("Newspaper", "Print & Paper", [
            fx("halftone", ["mode": "bw", "cell": 5.0, "angle": 45.0, "ink": RGBA(hex: "#1A1A1A")]),
            look("paper", "newsprint"),
        ]),
        make("Riso Duo", "Print & Paper", [
            fx("gradientmap", ["gradient": gradient("Duotone · Riso")]),
            fx("halftone", opacity: 0.55, blend: .multiply, ["mode": "bw", "cell": 5.0, "angle": 22.0]),
            fx("grain", ["amount": 30.0]),
        ]),
        make("CMYK Newsprint", "Print & Paper", [
            fx("halftone", ["mode": "cmyk", "cell": 7.0, "paper": RGBA(hex: "#F1EEE6")]),
            fx("grain", ["amount": 20.0]),
        ]),
        make("Cyanotype", "Print & Paper", [
            fx("gradientmap", ["gradient": gradient("Cyanotype")]),
            look("paper", "watercolor"),
            look("dust", "print", ["opacity": 40.0]),
        ]),
        make("Crumpled Print", "Print & Paper", [
            fx("film", ["stock": "gold200", "grain": 25.0]),
            look("paper", "crumpled"),
        ]),

        // MARK: Dither & pixel
        make("Game Boy", "Dither & Pixel", [
            fx("dither", ["algorithm": "bayer4", "mode": "palette", "palette": palette("Game Boy"), "pixelSize": 4.0, "contrast": 15.0]),
        ]),
        make("Mac Classic", "Dither & Pixel", [
            fx("dither", ["algorithm": "atkinson", "pixelSize": 2.0, "dark": RGBA(hex: "#000000"), "light": RGBA(hex: "#FFFFFF")]),
        ]),
        make("PICO-8 Pixels", "Dither & Pixel", [
            fx("dither", ["algorithm": "floyd", "mode": "palette", "palette": palette("PICO-8"), "pixelSize": 5.0, "strength": 70.0]),
        ]),
        make("Windows 98", "Dither & Pixel", [
            fx("dither", ["algorithm": "bayer8", "mode": "palette", "palette": palette("Windows 16"), "pixelSize": 2.0]),
        ]),
        make("ASCII Terminal", "Dither & Pixel", [
            fx("ascii", ["charset": "standard", "cell": 7.0, "fg": RGBA(hex: "#5CFF7A"), "bg": RGBA(hex: "#04110A")]),
            fx("glow", ["intensity": 80.0, "radius": 12.0, "threshold": 10.0]),
        ]),
        make("Matrix Rain", "Dither & Pixel", [
            fx("ascii", ["charset": "matrix", "cell": 9.0, "fg": RGBA(hex: "#3BFF6E"), "bg": RGBA(hex: "#000000"), "contrast": 45.0, "brightness": -20.0]),
            fx("bloom", ["threshold": 40.0, "intensity": 120.0, "radius": 15.0]),
        ]),

        // MARK: Light & mood
        make("Dreamy Bloom", "Light & Mood", [
            fx("basic", ["highlights": -20.0, "shadows": 15.0]),
            fx("bloom", ["threshold": 55.0, "intensity": 150.0, "radius": 60.0]),
            fx("grain", ["amount": 20.0]),
        ]),
        make("Summer Leak", "Light & Mood", [
            fx("film", ["stock": "portra400", "warmth": 20.0]),
            look("leak", "end"),
        ]),
        make("Ghost Trail", "Light & Mood", [
            fx("trail", ["distance": 90.0, "copies": 10.0, "blend": "screen", "rgbShift": 40.0]),
            fx("glow", opacity: 0.6, ["intensity": 60.0]),
        ]),

        // MARK: Tracking
        make("Thermal Cam", "Tracking", [
            fx("blur", ["radius": 3.0]),
            fx("gradientmap", ["gradient": gradient("Thermal · Iron")]),
            fx("blob", ["detect": "brightness", "threshold": 72.0, "shape": "cross", "label": "coords", "connect": "none",
                        "color": RGBA(hex: "#FFFFFF"), "maxBlobs": 6.0]),
        ]),
        make("Target Lock", "Tracking", [
            fx("bw", ["contrast": 25.0]),
            fx("blob", ["detect": "saliency", "shape": "brackets", "boxMode": "fit", "label": "custom",
                        "labelText": ParamValue.text("TARGET"), "color": RGBA(hex: "#FF3B30"), "invertInside": 100.0]),
        ]),
        make("Data Nodes", "Tracking", [
            fx("blob", ["detect": "brightness", "threshold": 60.0, "boxMode": "fixed", "boxSize": 18.0, "maxBlobs": 40.0,
                        "label": "coords", "labelSize": 9.0, "connect": "all", "connectOpacity": 35.0, "minSize": 0.05]),
        ]),
    ]
}

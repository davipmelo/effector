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

    /// A folder of layers inside a preset (children bottom-first, like `make`).
    static func grp(_ name: String, opacity: Double = 1, blend: BlendMode = .normal, _ bottomFirst: [LayerNode]) -> LayerNode {
        var g = LayerNode.group(name, bottomFirst.reversed())
        g.opacity = opacity
        g.blend = blend
        g.expanded = false
        return g
    }

    /// Curves value from (x, y) points in 0…1 per channel (empty = identity).
    static func curves(_ m: [(Double, Double)] = [], r: [(Double, Double)] = [], g: [(Double, Double)] = [],
                       b: [(Double, Double)] = []) -> ParamValue {
        func pts(_ l: [(Double, Double)]) -> [CurvePoint] {
            l.isEmpty ? CurveSet.identity : ([(0.0, 0.0)] + l + [(1.0, 1.0)]).map { CurvePoint(x: $0.0, y: $0.1) }
        }
        var c = CurveSet()
        c.master = pts(m); c.red = pts(r); c.green = pts(g); c.blue = pts(b)
        return .curves(c)
    }

    /// Ready-made curve shapes.
    static let sCurve = curves([(0.25, 0.20), (0.75, 0.80)])
    static let softS = curves([(0.25, 0.22), (0.75, 0.78)])

    static let categories = ["Cinema", "Film", "Portrait & Light", "Y2K & Digital", "Retro Video",
                             "Print & Graphic", "Pixel & Code", "Dream & Surreal", "Tracking & HUD"]

    private static func make(_ name: String, _ category: String, _ layersBottomFirst: [LayerNode]) -> Preset {
        Preset(name: name, layers: layersBottomFirst.reversed(), isFactory: true, category: category)
    }

    /// Film layer with grain kept fine and subtle (presets are showcases, not grain tests).
    static func film(_ stock: String, grain: Double = 0, halation: Double = 30, _ extra: [String: Any] = [:]) -> LayerNode {
        var v: [String: Any] = ["stock": stock, "grainOn": grain > 0, "grain": grain, "grainSize": 0.8,
                                "halationOn": halation > 0, "halation": halation]
        for (k, x) in extra { v[k] = x }
        return fx("film", v)
    }

    static let all: [Preset] = [
        // MARK: Cinema
        make("Neon Noir", "Cinema", [
            grp("Grade", [
                fx("basic", ["contrast": 12.0, "highlights": -30.0, "shadows": 8.0, "blacks": -10.0, "temperature": -18.0]),
                fx("hsl", ["green_h": 60.0, "green_s": -30.0, "aqua_s": 25.0, "blue_h": -15.0, "blue_s": 20.0,
                           "orange_s": 10.0, "magenta_s": 30.0]),
                fx("gradientmap", opacity: 0.35, blend: .softLight, ["gradient": gradient("Duotone · Teal & Orange")]),
            ]),
            film("vision500t", grain: 10, halation: 60, ["intensity": 70.0]),
            grp("Lens", [
                look("anamorphic", "blue", ["intensity": 90.0, "length": 70.0, "threshold": 78.0]),
                fx("bloom", opacity: 0.6, ["threshold": 74.0, "intensity": 70.0, "radius": 45.0]),
                fx("chromab", ["amount": 3.0]),
                fx("vignette", ["amount": 35.0, "feather": 70.0]),
            ]),
            look("frame", "scope"),
        ]),
        make("Golden Epic", "Cinema", [
            grp("Grade", [
                fx("basic", ["temperature": 22.0, "highlights": -35.0, "shadows": 15.0, "vibrance": 12.0, "contrast": 8.0]),
                fx("curves", ["curves": curves([(0.25, 0.21), (0.75, 0.80)], r: [(0.5, 0.53)], b: [(0.25, 0.28), (0.75, 0.70)])]),
            ]),
            film("vision250d", grain: 8, halation: 40, ["intensity": 60.0]),
            fx("glow", opacity: 0.45, ["intensity": 55.0, "radius": 45.0, "threshold": 55.0, "tint": RGBA(hex: "#FFD9A8")]),
            look("leak", "amber", opacity: 0.35, ["edge": "tl", "size": 40.0]),
            fx("vignette", ["amount": 25.0, "feather": 75.0]),
            look("frame", "ultra"),
        ]),
        make("Blockbuster", "Cinema", [
            grp("Teal & Orange", [
                fx("curves", ["curves": curves([(0.25, 0.19), (0.75, 0.81)], r: [(0.3, 0.27), (0.75, 0.79)], b: [(0.25, 0.31), (0.75, 0.70)])]),
                fx("hsl", ["orange_s": 15.0, "orange_l": 6.0, "yellow_h": -20.0, "green_h": 70.0, "green_s": -40.0,
                           "aqua_h": 10.0, "aqua_s": 30.0, "blue_h": -20.0, "blue_s": 15.0]),
                fx("basic", ["vibrance": 10.0, "blacks": -8.0]),
            ]),
            fx("sharpen", ["amount": 55.0, "radius": 1.2]),
            fx("halation", opacity: 0.6, ["intensity": 50.0, "radius": 14.0]),
            fx("vignette", ["amount": 30.0]),
            fx("frame", ["style": "letterbox", "ratio": "2.39"]),
        ]),
        make("Noir", "Cinema", [
            fx("bw", ["reds": 70.0, "yellows": 80.0, "blues": -10.0, "contrast": 25.0]),
            fx("curves", ["curves": curves([(0.18, 0.10), (0.5, 0.5), (0.82, 0.90)])]),
            fx("glow", opacity: 0.35, ["intensity": 50.0, "radius": 30.0, "threshold": 70.0, "mode": "soft"]),
            film("trix400", grain: 12, halation: 0, ["intensity": 40.0]),
            fx("vignette", ["amount": 60.0, "midpoint": 40.0, "feather": 70.0]),
            look("dust", "cinema", ["opacity": 20.0, "scratches": 5, "hairs": 1]),
            look("frame", "academy"),
        ]),
        make("Step-Print Neon", "Cinema", [
            fx("trail", ["style": "smooth", "mode": "dir", "angle": 0.0, "distance": 35.0, "opacity": 70.0, "decay": 70.0, "blend": "lighten"]),
            fx("hsl", ["green_h": 40.0, "green_s": 20.0, "yellow_h": 25.0, "red_s": 20.0, "aqua_h": -20.0]),
            film("cinestill800t", grain: 10, halation: 60),
            fx("bloom", opacity: 0.5, ["threshold": 84.0, "intensity": 50.0, "radius": 35.0]),
            fx("chromab", ["amount": 5.0]),
            fx("vignette", ["amount": 35.0]),
            fx("frame", ["style": "letterbox", "ratio": "1.85"]),
        ]),
        make("Desert Western", "Cinema", [
            fx("basic", ["exposure": 0.2, "temperature": 30.0, "contrast": 18.0, "highlights": -20.0, "shadows": 20.0, "saturation": -15.0]),
            fx("hsl", ["blue_s": -40.0, "aqua_s": -30.0, "orange_h": -5.0, "orange_s": 10.0]),
            film("bleach", grain: 10, halation: 25, ["intensity": 55.0]),
            fx("vignette", ["amount": 22.0, "roundness": 30.0]),
            look("dust", "cinema", ["opacity": 20.0, "amount": 10.0, "scratches": 4]),
            fx("frame", ["style": "letterbox", "ratio": "2.76"]),
        ]),
        make("Moonlight", "Cinema", [
            fx("basic", ["exposure": -0.25, "temperature": -30.0, "contrast": 10.0, "highlights": -15.0]),
            fx("gradientmap", opacity: 0.22, blend: .color, ["gradient": gradient("Duotone · Ultramarine")]),
            fx("hsl", ["orange_s": -10.0, "orange_l": 8.0]),
            fx("glow", opacity: 0.5, ["intensity": 45.0, "radius": 35.0, "threshold": 60.0, "tint": RGBA(hex: "#BCD2FF")]),
            fx("vignette", ["amount": 40.0]),
            look("frame", "scope"),
        ]),
        make("Indie Subtitle", "Cinema", [
            fx("curves", ["curves": curves([(0.0, 0.0), (0.12, 0.10), (0.85, 0.88)], g: [(0.5, 0.52)])]),
            fx("basic", ["blacks": 18.0, "contrast": -6.0, "saturation": -10.0, "temperature": 6.0]),
            film("vision250d", grain: 8, halation: 25, ["intensity": 55.0]),
            fx("vignette", ["amount": 22.0]),
            look("frame", "subtitle", ["caption": ParamValue.text("I don't remember the last time it rained like this."),
                                       "textColor": RGBA(hex: "#F4F1E8"), "textSize": 22.0]),
        ]),

        // MARK: Film
        make("Portra Afternoon", "Film", [
            fx("basic", ["highlights": -25.0, "shadows": 12.0, "temperature": 6.0]),
            film("portra400", grain: 8, halation: 25),
            fx("glow", opacity: 0.35, ["intensity": 45.0, "radius": 40.0, "threshold": 55.0, "tint": RGBA(hex: "#FFE6C8")]),
            fx("vignette", ["amount": 15.0, "feather": 80.0]),
        ]),
        make("Gold Summer", "Film", [
            film("gold200", grain: 10, halation: 30, ["warmth": 15.0]),
            fx("hsl", ["orange_s": 10.0, "yellow_s": 15.0, "blue_h": -10.0, "blue_s": -10.0]),
            look("leak", "end", opacity: 0.45, ["size": 30.0]),
            fx("vignette", ["amount": 22.0]),
            look("frame", "white", ["size": 55.0, "texture": 30.0]),
        ]),
        make("CineStill Streets", "Film", [
            fx("basic", ["contrast": 10.0, "shadows": 10.0]),
            film("cinestill800t", grain: 10, halation: 75),
            fx("bloom", opacity: 0.5, ["threshold": 86.0, "intensity": 45.0, "radius": 30.0]),
            fx("star", opacity: 0.6, ["points": "4", "length": 8.0, "intensity": 50.0, "threshold": 93.0]),
            fx("chromab", ["amount": 2.0]),
            fx("vignette", ["amount": 25.0]),
        ]),
        make("Expired Roll", "Film", [
            film("expired", grain: 16, halation: 35, ["fade": 25.0]),
            look("leak", "magenta", opacity: 0.7, ["size": 35.0]),
            fx("vignette", ["amount": 35.0, "roundness": 70.0]),
            look("dust", "scan", ["amount": 18.0, "opacity": 60.0]),
            look("frame", "rounded"),
        ]),
        make("Polaroid Memory", "Film", [
            fx("basic", ["contrast": -4.0, "blacks": 6.0, "highlights": -15.0, "shadows": 10.0]),
            film("polaroid600", grain: 0, halation: 20, ["intensity": 80.0]),
            fx("glow", opacity: 0.25, ["intensity": 40.0, "radius": 30.0, "threshold": 50.0]),
            fx("vignette", ["amount": 28.0, "roundness": 80.0]),
            fx("frame", ["style": "polaroid", "textOn": true, "caption": ParamValue.text("summer '99"),
                         "textColor": RGBA(hex: "#2B2B6B"), "textSize": 30.0, "textPlace": "bar"]),
        ]),
        make("Instax Party", "Film", [
            fx("basic", ["exposure": 0.25, "contrast": 8.0]),
            film("instax", grain: 0, halation: 15),
            fx("bloom", opacity: 0.45, ["threshold": 84.0, "intensity": 45.0, "radius": 30.0]),
            fx("star", ["points": "6", "length": 7.0, "intensity": 45.0, "spectral": true, "threshold": 93.0]),
            fx("frame", ["style": "polaroid", "color": RGBA(hex: "#FBFBF8")]),
        ]),
        make("Darkroom Silver", "Film", [
            fx("bw", ["reds": 60.0, "yellows": 70.0, "blues": 0.0, "contrast": 15.0]),
            fx("curves", ["curves": sCurve]),
            film("hp5", grain: 14, halation: 0, ["intensity": 40.0]),
            fx("vignette", ["amount": 30.0, "feather": 80.0]),
            look("dust", "print", ["opacity": 35.0, "amount": 15.0]),
            look("frame", "white", ["size": 70.0]),
        ]),
        make("Velvia Landscape", "Film", [
            fx("hsl", ["blue_s": 25.0, "blue_l": -10.0, "aqua_s": 20.0, "green_s": 15.0, "green_h": -10.0]),
            film("velvia50", grain: 0, halation: 10),
            fx("sharpen", ["amount": 70.0, "radius": 1.2]),
            fx("vignette", ["amount": 20.0]),
        ]),
        make("Lomo Cross", "Film", [
            look("fisheye", "mild"),
            film("crossprocess", grain: 12, halation: 30),
            fx("chromab", ["amount": 7.0, "falloff": 2.2]),
            fx("vignette", ["amount": 60.0, "roundness": 90.0, "midpoint": 40.0, "blurOn": true, "blur": 10.0]),
            look("leak", "red", opacity: 0.5),
        ]),
        make("Superia Snapshot", "Film", [
            fx("basic", ["exposure": 0.15, "shadows": 12.0]),
            film("superia400", grain: 10, halation: 20),
            fx("hsl", ["green_h": 15.0, "green_s": 10.0, "aqua_s": 15.0, "magenta_s": 10.0]),
            look("leak", "cool", opacity: 0.3),
            fx("vignette", ["amount": 20.0]),
            look("frame", "black", ["size": 45.0]),
        ]),
        make("Super 8 Memories", "Film", [
            film("kodachrome64", grain: 18, halation: 45, ["warmth": 20.0, "fade": 15.0]),
            fx("blur", ["radius": 2.0]),
            fx("trail", ["style": "smooth", "distance": 12.0, "opacity": 50.0, "angle": 90.0]),
            fx("vignette", ["amount": 55.0, "roundness": 80.0, "feather": 60.0]),
            look("dust", "cinema", ["opacity": 30.0, "scratches": 6]),
            fx("frame", ["style": "rounded", "barColor": RGBA(hex: "#000000"), "color": RGBA(hex: "#050505"), "radius": 30.0]),
        ]),

        // MARK: Portrait & light
        make("Pro-Mist Portrait", "Portrait & Light", [
            fx("basic", ["contrast": -8.0, "shadows": 10.0, "highlights": -10.0]),
            fx("hsl", ["orange_l": 8.0, "orange_s": -6.0, "red_s": -8.0]),
            film("portra160", grain: 0, halation: 30),
            fx("glow", opacity: 0.6, ["intensity": 50.0, "radius": 45.0, "threshold": 45.0, "tint": RGBA(hex: "#FFF1E0")]),
            fx("bloom", opacity: 0.5, ["threshold": 70.0, "intensity": 60.0, "radius": 60.0]),
            fx("vignette", ["amount": 15.0]),
        ]),
        make("Black Mist", "Portrait & Light", [
            fx("basic", ["blacks": 12.0, "highlights": -18.0, "contrast": -5.0]),
            film("vision500t", grain: 6, halation: 50, ["intensity": 60.0]),
            fx("glow", opacity: 0.55, ["intensity": 60.0, "radius": 60.0, "threshold": 40.0, "mode": "soft"]),
            fx("halo", opacity: 0.4, ["intensity": 90.0, "radius": 50.0]),
            fx("vignette", ["amount": 30.0]),
        ]),
        make("Editorial Skin", "Portrait & Light", [
            look("blur", "skin", opacity: 0.6),
            fx("hsl", ["orange_l": 10.0, "orange_s": -10.0, "red_l": 5.0]),
            fx("basic", ["contrast": 10.0, "whites": 8.0, "vibrance": 8.0]),
            fx("sharpen", ["amount": 60.0, "radius": 1.0]),
            fx("glow", opacity: 0.3, ["intensity": 40.0, "radius": 35.0, "threshold": 60.0]),
            look("frame", "white", ["size": 45.0, "texture": 0.0]),
        ]),
        make("Dream Portrait", "Portrait & Light", [
            look("lensblur", "portrait", ["amount": 10.0, "size": 45.0, "falloff": 45.0]),
            fx("basic", ["exposure": 0.1, "contrast": -6.0, "temperature": 8.0]),
            fx("halo", opacity: 0.45),
            fx("bloom", opacity: 0.7, ["threshold": 72.0, "intensity": 60.0, "radius": 60.0]),
            fx("vignette", ["amount": 15.0, "feather": 80.0]),
        ]),
        make("Tilt-Shift Town", "Portrait & Light", [
            look("lensblur", "mini"),
            fx("basic", ["contrast": 15.0, "vibrance": 25.0, "exposure": 0.1]),
            fx("sharpen", ["amount": 50.0]),
            fx("vignette", ["amount": 18.0]),
        ]),
        make("Window Light B&W", "Portrait & Light", [
            fx("bw", ["reds": 85.0, "yellows": 75.0, "blues": -10.0, "contrast": 12.0]),
            fx("curves", ["curves": curves([(0.25, 0.23), (0.7, 0.76)])]),
            fx("glow", opacity: 0.3, ["intensity": 40.0, "radius": 40.0, "threshold": 55.0, "mode": "soft"]),
            film("acros100", grain: 6, halation: 0, ["intensity": 30.0]),
            fx("vignette", ["amount": 35.0]),
        ]),
        make("Star Night", "Portrait & Light", [
            fx("basic", ["exposure": -0.2, "contrast": 15.0, "temperature": -10.0]),
            fx("star", ["points": "6", "length": 16.0, "intensity": 100.0, "spectral": true, "threshold": 90.0]),
            fx("bloom", opacity: 0.6, ["threshold": 84.0, "intensity": 60.0, "radius": 30.0]),
            fx("halation", opacity: 0.6),
            fx("vignette", ["amount": 30.0]),
        ]),

        // MARK: Y2K & digital
        make("Digicam 2003", "Y2K & Digital", [
            look("digicam", "cybershot", ["noise": 15.0, "chromaNoise": 20.0]),
            fx("star", opacity: 0.6, ["points": "4", "length": 8.0, "threshold": 90.0]),
            fx("vignette", ["amount": 15.0]),
        ]),
        make("Party Flash", "Y2K & Digital", [
            look("digicam", "flash", ["noise": 15.0, "chromaNoise": 20.0]),
            fx("bloom", opacity: 0.6, ["threshold": 78.0, "intensity": 70.0, "radius": 25.0]),
            fx("chromab", ["amount": 3.0]),
            fx("vignette", ["amount": 45.0, "roundness": 70.0]),
        ]),
        make("Y2K Ice", "Y2K & Digital", [
            fx("basic", ["exposure": 0.1, "contrast": -8.0, "temperature": -20.0, "highlights": -20.0]),
            fx("gradientmap", opacity: 0.5, blend: .color, ["gradient": gradient("Y2K Ice")]),
            fx("bloom", opacity: 0.7, ["threshold": 82.0, "intensity": 55.0, "radius": 45.0, "tint": RGBA(hex: "#BFD8FF")]),
            fx("star", ["points": "4", "spectral": true, "length": 16.0, "intensity": 90.0]),
            fx("chromab", ["amount": 4.0]),
            fx("frame", ["style": "rounded", "color": RGBA(hex: "#DDE9FF"), "radius": 24.0]),
        ]),
        make("Liquid Chrome", "Y2K & Digital", [
            fx("gradientmap", ["gradient": gradient("Y2K Chrome")]),
            fx("curves", ["curves": curves([(0.3, 0.22), (0.55, 0.62), (0.8, 0.82)])]),
            fx("sharpen", ["amount": 90.0, "radius": 1.5]),
            fx("bloom", opacity: 0.6, ["threshold": 92.0, "intensity": 40.0, "radius": 20.0]),
            fx("star", opacity: 0.7, ["points": "8", "spectral": true, "length": 10.0, "threshold": 96.0, "intensity": 60.0]),
        ]),
        make("Frutiger Aero", "Y2K & Digital", [
            fx("basic", ["exposure": 0.35, "vibrance": 40.0, "highlights": -20.0, "shadows": 25.0]),
            fx("gradientmap", opacity: 0.5, blend: .softLight, ["gradient": gradient("Frutiger Aero")]),
            fx("bloom", opacity: 0.7, ["threshold": 75.0, "intensity": 70.0, "radius": 40.0]),
            fx("halo", opacity: 0.4, ["radius": 80.0]),
            fx("frame", ["style": "rounded", "color": RGBA(hex: "#F4FAFF"), "radius": 30.0]),
        ]),
        make("Holo Sticker", "Y2K & Digital", [
            fx("gradientmap", opacity: 0.5, blend: .overlay, ["gradient": gradient("Holographic")]),
            look("prism", "side", opacity: 0.5),
            fx("star", ["points": "8", "spectral": true, "length": 10.0, "threshold": 93.0, "intensity": 70.0]),
            fx("bloom", opacity: 0.4, ["threshold": 82.0, "radius": 30.0]),
            fx("chromab", ["amount": 5.0]),
        ]),
        make("Flip Phone", "Y2K & Digital", [
            look("digicam", "phone", ["noise": 25.0, "chromaNoise": 25.0]),
            fx("vignette", ["amount": 30.0]),
            look("frame", "black", ["size": 40.0]),
        ]),
        make("Deep Fried", "Y2K & Digital", [
            fx("basic", ["saturation": 80.0, "contrast": 60.0]),
            fx("sharpen", ["amount": 400.0, "radius": 2.5]),
            look("fisheye", "bubble", ["amount": 120.0, "zoom": 120.0]),
            look("digicam", "phone", ["quality": 3.0, "saturation": 160.0]),
            fx("chromab", ["amount": 8.0]),
        ]),

        // MARK: Retro video
        make("Tape on a CRT", "Retro Video", [
            look("vhs", "rental94"),
            look("crt", "tv", ["brightness": 105.0]),
        ]),
        make("Home Video '86", "Retro Video", [
            fx("basic", ["temperature": 12.0]),
            look("vhs", "home86"),
            fx("vignette", ["amount": 20.0]),
        ]),
        make("Music Video '89", "Retro Video", [
            look("prism", "side", opacity: 0.5),
            look("vhs", "music89"),
            fx("glow", opacity: 0.6, ["intensity": 60.0, "radius": 25.0]),
            fx("star", ["points": "4", "length": 18.0, "intensity": 120.0, "threshold": 80.0]),
        ]),
        make("Skate Video", "Retro Video", [
            fx("basic", ["contrast": 15.0, "saturation": 10.0]),
            look("fisheye", "vx"),
            look("vhs", "hi8", ["osdBottom": ParamValue.text("SEP 9 2001")]),
        ]),
        make("Camcorder Beach", "Retro Video", [
            fx("basic", ["exposure": 0.2, "temperature": 15.0]),
            look("leak", "amber", opacity: 0.4),
            look("vhs", "camcorder98"),
        ]),
        make("Late Night PVM", "Retro Video", [
            look("vhs", "latenight"),
            look("crt", "pvm"),
            fx("bloom", opacity: 0.5, ["threshold": 70.0, "radius": 20.0]),
        ]),
        make("CCTV Feed", "Retro Video", [
            fx("bw", ["contrast": 15.0]),
            fx("blob", ["detect": "saliency", "shape": "corners", "label": "custom", "labelText": ParamValue.text("SUBJECT 01"),
                        "connectOn": false, "maxBlobs": 1.0, "color": RGBA(hex: "#FFFFFF"), "labelSize": 14.0]),
            look("vhs", "cctv"),
            fx("crt", ["curvature": 10.0, "maskOn": false, "scan": 30.0, "glow": 20.0]),
        ]),
        make("Arcade Cabinet", "Retro Video", [
            fx("basic", ["contrast": 20.0, "vibrance": 30.0, "shadows": 15.0]),
            fx("dither", ["algorithm": "bayer4", "mode": "palette", "palette": palette("Sweetie 16"), "pixelSize": 3.0]),
            look("crt", "arcade", ["brightness": 100.0]),
            fx("bloom", opacity: 0.4, ["threshold": 70.0, "radius": 18.0]),
        ]),

        // MARK: Print & graphic
        make("Risograph Duo", "Print & Graphic", [
            fx("basic", ["exposure": 0.6, "shadows": 40.0, "contrast": 10.0]),
            fx("gradientmap", ["gradient": gradient("Duotone · Riso")]),
            fx("halftone", opacity: 0.5, blend: .multiply, ["mode": "bw", "cell": 5.0, "angle": 22.0]),
            fx("xerox", opacity: 0.25, blend: .multiply, ["threshold": 80.0, "streaks": 50.0, "noise": 40.0]),
            look("dust", "print", ["opacity": 25.0]),
        ]),
        make("Comic Newsprint", "Print & Graphic", [
            fx("basic", ["exposure": 0.5, "shadows": 40.0, "contrast": 15.0, "vibrance": 30.0]),
            fx("posterize", opacity: 0.6, ["levels": 5.0]),
            fx("halftone", ["mode": "cmyk", "cell": 6.0, "paper": RGBA(hex: "#F1ECDF")]),
            look("dust", "print", ["opacity": 25.0]),
            look("frame", "white", ["size": 45.0, "color": RGBA(hex: "#F1ECDF")]),
        ]),
        make("Zine Xerox", "Print & Graphic", [
            fx("basic", ["exposure": 0.5, "shadows": 40.0, "contrast": 15.0]),
            fx("xerox", ["threshold": 45.0, "dither": true, "generations": 2]),
            look("dust", "scan", ["style": "dark", "opacity": 40.0]),
            fx("frame", ["style": "border", "color": RGBA(hex: "#F2F2F2"), "size": 110.0, "textOn": true,
                         "caption": ParamValue.text("ISSUE #03 — NO FUTURE"), "textColor": RGBA(hex: "#111111"), "textSize": 22.0]),
        ]),
        make("Thermal Receipt", "Print & Graphic", [
            fx("basic", ["exposure": 0.3, "shadows": 25.0, "contrast": 10.0]),
            fx("receipt", ["dead": 8.0, "banding": 20.0]),
            fx("vignette", ["amount": 15.0]),
        ]),
        make("Cyanotype", "Print & Graphic", [
            fx("basic", ["exposure": 0.3, "shadows": 25.0]),
            fx("curves", ["curves": softS]),
            fx("gradientmap", ["gradient": gradient("Cyanotype")]),
            look("dust", "print", ["opacity": 30.0]),
            fx("vignette", ["amount": 25.0, "color": RGBA(hex: "#0B1F4A")]),
            look("frame", "white", ["size": 80.0, "texture": 80.0]),
        ]),
        make("Pop Art", "Print & Graphic", [
            fx("basic", ["exposure": 0.5, "shadows": 45.0, "contrast": 20.0, "saturation": 45.0]),
            fx("posterize", ["levels": 4.0]),
            fx("halftone", opacity: 0.4, blend: .multiply, ["mode": "bw", "cell": 10.0, "angle": 45.0]),
            fx("frame", ["style": "border", "color": RGBA(hex: "#FFD400"), "size": 60.0, "texture": 0.0]),
        ]),
        make("Fax Machine", "Print & Graphic", [
            fx("basic", ["exposure": 0.5, "shadows": 40.0]),
            fx("dither", ["algorithm": "atkinson", "pixelSize": 2.0, "contrast": 20.0, "brightness": 20.0, "dark": RGBA(hex: "#101010"), "light": RGBA(hex: "#F4F4F4")]),
            fx("xerox", opacity: 0.5, blend: .multiply, ["threshold": 70.0, "edge": 20.0, "streaks": 60.0]),
            fx("stretch", opacity: 0.35, blend: .multiply, ["dir": "down", "position": 92.0, "jitter": 40.0]),
        ]),

        // MARK: Pixel & code
        make("Game Boy Camera", "Pixel & Code", [
            fx("basic", ["exposure": 0.4, "shadows": 35.0, "contrast": 15.0]),
            fx("dither", ["algorithm": "bayer4", "mode": "palette", "palette": palette("Game Boy"), "pixelSize": 4.0]),
            fx("vignette", ["amount": 10.0]),
            fx("frame", ["style": "border", "color": RGBA(hex: "#8B956D"), "size": 70.0, "texture": 10.0]),
        ]),
        make("Mac Classic", "Pixel & Code", [
            fx("basic", ["exposure": 0.3, "shadows": 30.0]),
            fx("dither", ["algorithm": "atkinson", "pixelSize": 2.0, "dark": RGBA(hex: "#000000"), "light": RGBA(hex: "#FFFFFF")]),
            fx("crt", ["curvature": 12.0, "maskOn": false, "scan": 15.0, "glow": 15.0, "vignette": 20.0, "noiseOn": false]),
        ]),
        make("Green Terminal", "Pixel & Code", [
            look("ascii", "terminal", ["brightness": 25.0]),
            fx("glow", opacity: 0.5, ["intensity": 60.0, "radius": 10.0, "threshold": 20.0]),
            look("crt", "pc", ["maskOn": false]),
        ]),
        make("Digital Rain", "Pixel & Code", [
            look("ascii", "matrix"),
            fx("trail", ["style": "copies", "angle": 90.0, "distance": 60.0, "copies": 6.0, "decay": 70.0, "blend": "lighten"]),
            fx("bloom", ["threshold": 40.0, "intensity": 120.0, "radius": 15.0]),
            fx("chromab", ["amount": 3.0]),
        ]),
        make("Windows 98", "Pixel & Code", [
            fx("basic", ["vibrance": 25.0]),
            fx("dither", ["algorithm": "bayer8", "mode": "palette", "palette": palette("Windows 16"), "pixelSize": 2.0]),
            look("crt", "pc"),
        ]),
        make("Color ASCII", "Pixel & Code", [
            fx("basic", ["exposure": 0.5, "shadows": 40.0, "contrast": 10.0, "vibrance": 30.0]),
            look("ascii", "color", ["brightness": 30.0]),
            fx("bloom", opacity: 0.5, ["threshold": 55.0, "radius": 12.0]),
        ]),

        // MARK: Dream & surreal
        make("Kaleido Bloom", "Dream & Surreal", [
            look("prism", "k6"),
            fx("hsl", ["orange_s": 15.0, "magenta_s": 25.0, "purple_s": 20.0]),
            fx("bloom", ["threshold": 65.0, "intensity": 90.0, "radius": 40.0]),
            fx("vignette", ["amount": 40.0, "roundness": 100.0]),
        ]),
        make("Mandala", "Dream & Surreal", [
            look("prism", "mandala"),
            fx("gradientmap", opacity: 0.4, blend: .softLight, ["gradient": gradient("Golden Hour")]),
            fx("halo", opacity: 0.6),
            fx("vignette", ["amount": 50.0, "roundness": 100.0]),
        ]),
        make("Glass Shards", "Dream & Surreal", [
            look("prism", "shards"),
            fx("chromab", ["amount": 5.0]),
            fx("hsl", ["blue_s": 20.0, "aqua_s": 20.0, "magenta_s": 20.0]),
            fx("glow", opacity: 0.4, ["intensity": 40.0, "radius": 25.0, "threshold": 70.0]),
            fx("vignette", ["amount": 30.0]),
        ]),
        make("Hyperspace", "Dream & Surreal", [
            look("edgewarp", "warp", ["strength": 160.0]),
            look("radialblur", "zoom", ["amount": 90.0, "protect": 12.0]),
            fx("chromab", ["amount": 10.0]),
            fx("bloom", opacity: 0.7, ["threshold": 82.0, "intensity": 60.0, "radius": 40.0]),
            fx("vignette", ["amount": 45.0]),
        ]),
        make("Vortex", "Dream & Surreal", [
            look("edgewarp", "vortex"),
            look("radialblur", "spin", ["protect": 25.0]),
            fx("gradientmap", opacity: 0.4, blend: .softLight, ["gradient": gradient("Dusk")]),
            fx("glow", opacity: 0.6, ["intensity": 60.0, "radius": 30.0]),
            fx("vignette", ["amount": 40.0]),
        ]),
        make("Liquid Edges", "Dream & Surreal", [
            look("edgewarp", "dream"),
            fx("hsl", ["blue_s": 20.0, "purple_s": 20.0, "magenta_h": -10.0]),
            look("blur", "edges", ["radius": 12.0]),
            fx("glow", opacity: 0.5, ["intensity": 50.0, "radius": 40.0]),
            fx("vignette", ["amount": 25.0]),
        ]),
        make("Vaporwave", "Dream & Surreal", [
            fx("gradientmap", opacity: 0.65, blend: .color, ["gradient": gradient("Vaporwave")]),
            fx("mirror", ["mode": "lr"]),
            look("vhs", "clean"),
            fx("chromab", ["amount": 6.0]),
            fx("star", ["points": "4", "spectral": true]),
        ]),
        make("Ghost Echo", "Dream & Surreal", [
            fx("bw", ["contrast": 20.0]),
            fx("trail", ["distance": 90.0, "copies": 10.0, "blend": "screen", "rgbShift": 60.0]),
            fx("glow", opacity: 0.6, ["intensity": 60.0]),
            fx("vignette", ["amount": 50.0]),
        ]),
        make("Mirror Ball", "Dream & Surreal", [
            look("fisheye", "ball"),
            fx("star", ["points": "8", "spectral": true, "length": 12.0, "threshold": 93.0, "intensity": 70.0]),
            fx("bloom", opacity: 0.6, ["threshold": 84.0, "intensity": 60.0]),
        ]),

        // MARK: Tracking & HUD
        make("Thermal Cam", "Tracking & HUD", [
            fx("blur", ["radius": 3.0]),
            fx("gradientmap", ["gradient": gradient("Thermal · Iron")]),
            fx("blob", ["detect": "brightness", "threshold": 72.0, "shape": "cross", "label": "coords", "connectOn": false,
                        "color": RGBA(hex: "#FFFFFF"), "maxBlobs": 6.0]),
            fx("crt", ["curvature": 0.0, "bezel": false, "maskOn": false, "scan": 25.0, "glow": 25.0, "vigOn": true, "vignette": 30.0]),
        ]),
        make("Night Vision", "Tracking & HUD", [
            fx("basic", ["exposure": 0.6, "contrast": 20.0]),
            fx("gradientmap", ["gradient": gradient("Night Vision")]),
            fx("grain", ["amount": 25.0, "size": 1.2]),
            fx("bloom", opacity: 0.6, ["threshold": 60.0, "intensity": 90.0]),
            fx("blob", ["detect": "saliency", "shape": "brackets", "label": "custom", "labelText": ParamValue.text("CONTACT"),
                        "color": RGBA(hex: "#B8FFB0"), "connectOn": false, "maxBlobs": 1.0]),
            fx("vignette", ["amount": 85.0, "roundness": 100.0, "midpoint": 35.0]),
        ]),
        make("Target Lock", "Tracking & HUD", [
            fx("bw", ["contrast": 25.0]),
            fx("blob", ["detect": "saliency", "shape": "brackets", "boxMode": "fit", "label": "custom",
                        "labelText": ParamValue.text("TARGET"), "color": RGBA(hex: "#FF3B30"), "maxBlobs": 1.0,
                        "dimOutside": 45.0, "connectOn": false]),
            fx("vignette", ["amount": 35.0]),
            fx("chromab", ["amount": 3.0]),
        ]),
        make("Data Nodes", "Tracking & HUD", [
            fx("basic", ["exposure": -0.4, "saturation": -60.0, "contrast": 15.0]),
            fx("gradientmap", opacity: 0.6, blend: .color, ["gradient": gradient("Duotone · Ultramarine")]),
            fx("blob", ["detect": "brightness", "threshold": 45.0, "boxMode": "fixed", "boxSize": 18.0, "maxBlobs": 40.0,
                        "label": "coords", "labelSize": 9.0, "connect": "all", "connectOpacity": 35.0, "minSize": 0.05,
                        "color": RGBA(hex: "#7FE3FF")]),
            fx("glow", opacity: 0.6, ["intensity": 60.0, "radius": 10.0, "threshold": 60.0]),
        ]),
        make("Color Hunter", "Tracking & HUD", [
            fx("hsl", ["yellow_s": -100.0, "green_s": -100.0, "aqua_s": -100.0, "blue_s": -100.0, "purple_s": -100.0,
                       "magenta_s": -60.0, "orange_s": -40.0, "red_s": 20.0]),
            fx("blob", ["detect": "color", "target": RGBA(hex: "#E03020"), "tolerance": 40.0, "minSize": 0.05, "maxSize": 15.0, "shape": "circle",
                        "label": "hex", "color": RGBA(hex: "#FFFFFF"), "connect": "sequence", "dashed": true]),
            fx("vignette", ["amount": 30.0]),
        ]),
    ]
}

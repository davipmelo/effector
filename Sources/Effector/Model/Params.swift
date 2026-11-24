import Foundation
import SwiftUI

// MARK: - Basic value types

struct RGBA: Codable, Hashable {
    var r: Double
    var g: Double
    var b: Double
    var a: Double = 1

    static let black = RGBA(r: 0, g: 0, b: 0)
    static let white = RGBA(r: 1, g: 1, b: 1)

    init(r: Double, g: Double, b: Double, a: Double = 1) {
        self.r = r; self.g = g; self.b = b; self.a = a
    }

    init(hex: String) {
        var s = hex.trimmingCharacters(in: .whitespacesAndNewlines).uppercased()
        if s.hasPrefix("#") { s.removeFirst() }
        var v: UInt64 = 0
        Scanner(string: s).scanHexInt64(&v)
        if s.count >= 6 {
            r = Double((v >> 16) & 0xFF) / 255
            g = Double((v >> 8) & 0xFF) / 255
            b = Double(v & 0xFF) / 255
        } else {
            r = 0; g = 0; b = 0
        }
        a = 1
    }

    var hex: String {
        String(format: "#%02X%02X%02X",
               Int((r.clamped01 * 255).rounded()),
               Int((g.clamped01 * 255).rounded()),
               Int((b.clamped01 * 255).rounded()))
    }

    var color: Color { Color(.sRGB, red: r, green: g, blue: b, opacity: a) }

    init(_ color: Color) {
        let ns = NSColor(color).usingColorSpace(.sRGB) ?? .black
        r = Double(ns.redComponent); g = Double(ns.greenComponent)
        b = Double(ns.blueComponent); a = Double(ns.alphaComponent)
    }

    var floats: [Float] { [Float(r), Float(g), Float(b)] }
    var luma: Double { 0.2126 * r + 0.7152 * g + 0.0722 * b }
}

extension Double {
    var clamped01: Double { Swift.min(1, Swift.max(0, self)) }
}

struct CurvePoint: Codable, Hashable {
    var x: Double
    var y: Double
}

struct CurveSet: Codable, Hashable {
    var master: [CurvePoint] = CurveSet.identity
    var red: [CurvePoint] = CurveSet.identity
    var green: [CurvePoint] = CurveSet.identity
    var blue: [CurvePoint] = CurveSet.identity

    static let identity = [CurvePoint(x: 0, y: 0), CurvePoint(x: 1, y: 1)]

    subscript(channel: Int) -> [CurvePoint] {
        get { [master, red, green, blue][channel] }
        set {
            switch channel {
            case 0: master = newValue
            case 1: red = newValue
            case 2: green = newValue
            default: blue = newValue
            }
        }
    }
}

struct GradientStop: Codable, Hashable, Identifiable {
    var id = UUID()
    var position: Double
    var color: RGBA
}

struct GradientSpec: Codable, Hashable {
    var name: String = "Custom"
    var stops: [GradientStop]

    func sample(_ t: Double) -> RGBA {
        let s = stops.sorted { $0.position < $1.position }
        guard let first = s.first else { return .black }
        if t <= first.position { return first.color }
        for i in 0..<(s.count - 1) {
            let a = s[i], b = s[i + 1]
            if t >= a.position && t <= b.position {
                let f = (b.position - a.position) > 1e-6 ? (t - a.position) / (b.position - a.position) : 0
                return RGBA(r: a.color.r + (b.color.r - a.color.r) * f,
                            g: a.color.g + (b.color.g - a.color.g) * f,
                            b: a.color.b + (b.color.b - a.color.b) * f)
            }
        }
        return s.last!.color
    }

    static func from(_ name: String, _ hexes: [String]) -> GradientSpec {
        let n = max(1, hexes.count - 1)
        return GradientSpec(name: name, stops: hexes.enumerated().map {
            GradientStop(position: Double($0.offset) / Double(n), color: RGBA(hex: $0.element))
        })
    }
}

struct PaletteSpec: Codable, Hashable {
    var name: String = "Custom"
    var colors: [RGBA]

    static func from(_ name: String, _ hexes: [String]) -> PaletteSpec {
        PaletteSpec(name: name, colors: hexes.map { RGBA(hex: $0) })
    }
}

struct LUTData: Codable, Hashable {
    var name: String
    var size: Int
    var values: [Float] // r,g,b triplets, r fastest

    var isEmpty: Bool { size == 0 }
}

// MARK: - Parameter values

enum ParamValue: Codable, Hashable {
    case number(Double)
    case bool(Bool)
    case choice(String)
    case color(RGBA)
    case curves(CurveSet)
    case gradient(GradientSpec)
    case palette(PaletteSpec)
    case text(String)
    case lut(LUTData)
}

/// Bag of parameter values with typed getters that fall back to defaults.
struct Params: Codable, Hashable {
    var values: [String: ParamValue] = [:]

    subscript(key: String) -> ParamValue? {
        get { values[key] }
        set { values[key] = newValue }
    }

    func num(_ k: String, _ d: Double = 0) -> Double {
        if case .number(let v)? = values[k] { return v }
        return d
    }
    func f(_ k: String, _ d: Double = 0) -> Float { Float(num(k, d)) }
    func bool(_ k: String, _ d: Bool = false) -> Bool {
        if case .bool(let v)? = values[k] { return v }
        return d
    }
    func choice(_ k: String, _ d: String = "") -> String {
        if case .choice(let v)? = values[k] { return v }
        return d
    }
    func color(_ k: String, _ d: RGBA = .black) -> RGBA {
        if case .color(let v)? = values[k] { return v }
        return d
    }
    func curves(_ k: String) -> CurveSet {
        if case .curves(let v)? = values[k] { return v }
        return CurveSet()
    }
    func gradient(_ k: String) -> GradientSpec? {
        if case .gradient(let v)? = values[k] { return v }
        return nil
    }
    func palette(_ k: String) -> PaletteSpec? {
        if case .palette(let v)? = values[k] { return v }
        return nil
    }
    func text(_ k: String, _ d: String = "") -> String {
        if case .text(let v)? = values[k] { return v }
        return d
    }
    func lut(_ k: String) -> LUTData? {
        if case .lut(let v)? = values[k] { return v }
        return nil
    }
}

// MARK: - Parameter definitions (drive the inspector UI)

struct ChoiceOption: Hashable {
    let id: String
    let label: String
    /// Optional section title used to group long lists in pickers.
    var group: String? = nil
}

enum ParamKind {
    case slider(min: Double, max: Double, def: Double, step: Double = 0, unit: String = "", curve: Double = 1)
    case toggle(def: Bool)
    case choice([ChoiceOption], def: String)
    case segmented([ChoiceOption], def: String)
    case color(def: RGBA)
    case curves
    case gradient(def: GradientSpec)
    case palette(def: PaletteSpec)
    case text(def: String)
    case lut
    case seed
}

struct ParamDef {
    let key: String
    let label: String
    let kind: ParamKind
    var section: String? = nil
    var visible: ((Params) -> Bool)? = nil
    /// Optional colours drawn on the slider track (e.g. hue ramps).
    var track: [RGBA]? = nil
    /// A boolean shown as an on/off switch in its section header (turns that whole part of the effect on or off).
    var isSectionSwitch = false

    var defaultValue: ParamValue? {
        switch kind {
        case .slider(_, _, let d, _, _, _): return .number(d)
        case .toggle(let d): return .bool(d)
        case .choice(_, let d), .segmented(_, let d): return .choice(d)
        case .color(let d): return .color(d)
        case .curves: return .curves(CurveSet())
        case .gradient(let d): return .gradient(d)
        case .palette(let d): return .palette(d)
        case .text(let d): return .text(d)
        case .lut: return nil
        case .seed: return .number(1)
        }
    }
}

func opts(_ pairs: [(String, String)]) -> [ChoiceOption] {
    pairs.map { ChoiceOption(id: $0.0, label: $0.1) }
}

extension ParamDef {
    /// `curve` > 1 gives finer control at the low end of the range (slider position^curve).
    static func slider(_ key: String, _ label: String, _ min: Double, _ max: Double, _ def: Double,
                       step: Double = 0, unit: String = "", curve: Double = 1, section: String? = nil,
                       track: [RGBA]? = nil, visible: ((Params) -> Bool)? = nil) -> ParamDef {
        ParamDef(key: key, label: label, kind: .slider(min: min, max: max, def: def, step: step, unit: unit, curve: curve),
                 section: section, visible: visible, track: track)
    }
    static func choice(_ key: String, _ label: String, options: [ChoiceOption], _ def: String,
                       section: String? = nil, visible: ((Params) -> Bool)? = nil) -> ParamDef {
        ParamDef(key: key, label: label, kind: .choice(options, def: def), section: section, visible: visible)
    }
    static func toggle(_ key: String, _ label: String, _ def: Bool, section: String? = nil,
                       visible: ((Params) -> Bool)? = nil) -> ParamDef {
        ParamDef(key: key, label: label, kind: .toggle(def: def), section: section, visible: visible)
    }
    static func choice(_ key: String, _ label: String, _ options: [(String, String)], _ def: String,
                       section: String? = nil, visible: ((Params) -> Bool)? = nil) -> ParamDef {
        ParamDef(key: key, label: label, kind: .choice(opts(options), def: def), section: section, visible: visible)
    }
    static func segmented(_ key: String, _ label: String, _ options: [(String, String)], _ def: String,
                          section: String? = nil, visible: ((Params) -> Bool)? = nil) -> ParamDef {
        ParamDef(key: key, label: label, kind: .segmented(opts(options), def: def), section: section, visible: visible)
    }
    static func color(_ key: String, _ label: String, _ def: RGBA, section: String? = nil,
                      visible: ((Params) -> Bool)? = nil) -> ParamDef {
        ParamDef(key: key, label: label, kind: .color(def: def), section: section, visible: visible)
    }
    /// On/off switch for a whole section, drawn in the section header.
    static func sectionSwitch(_ key: String, section: String, _ def: Bool = true) -> ParamDef {
        var d = ParamDef(key: key, label: section, kind: .toggle(def: def), section: section)
        d.isSectionSwitch = true
        return d
    }
    static func seed(_ key: String = "seed", section: String? = nil) -> ParamDef {
        ParamDef(key: key, label: "Seed", kind: .seed, section: section)
    }
}

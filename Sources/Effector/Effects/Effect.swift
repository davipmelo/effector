import Foundation
import Metal

enum EffectCategory: String, CaseIterable {
    case adjust, color, stylize, light, film, glitch, overlay

    var label: String {
        switch self {
        case .adjust: return "Adjustments"
        case .color: return "Color"
        case .stylize: return "Dither & Print"
        case .light: return "Light & Blur"
        case .film: return "Film"
        case .glitch: return "Glitch & Retro"
        case .overlay: return "Tracking"
        }
    }

    var symbol: String {
        switch self {
        case .adjust: return "slider.horizontal.3"
        case .color: return "paintpalette"
        case .stylize: return "circle.grid.3x3"
        case .light: return "sun.max"
        case .film: return "film"
        case .glitch: return "tv"
        case .overlay: return "viewfinder"
        }
    }
}

protocol Effect {
    var id: String { get }
    var name: String { get }
    var category: EffectCategory { get }
    var symbol: String { get }
    var params: [ParamDef] { get }
    /// Produces the effect output from `input`. The renderer blends it with `input`
    /// using the layer opacity / blend mode.
    func apply(_ input: MTLTexture, _ p: Params, _ ctx: RenderContext) -> MTLTexture
}

extension Effect {
    var symbol: String { category.symbol }
    var producesAlpha: Bool { false }
}

enum EffectRegistry {
    static let all: [Effect] = [
        // Adjustments
        BasicAdjustEffect(), CurvesEffect(), HSLEffect(), HueSatEffect(), BlackWhiteEffect(),
        InvertEffect(), PosterizeEffect(), ThresholdEffect(),
        // Color
        GradientMapEffect(), LUTEffect(),
        // Dither & print
        DitherEffect(), HalftoneEffect(), AsciiEffect(),
        // Light & blur
        GlowEffect(), BloomEffect(), HalationEffect(), BlurEffect(), TrailEffect(),
        // Film
        FilmEffect(), GrainEffect(),
        // Glitch
        VHSEffect(),
        // Tracking
        BlobTrackerEffect(),
    ]

    private static let byID: [String: Effect] = Dictionary(uniqueKeysWithValues: all.map { ($0.id, $0) })

    static func effect(_ id: String) -> Effect? { byID[id] }
}

/// Visibility helpers for param defs.
func when(_ key: String, _ value: String) -> (Params) -> Bool { { $0.choice(key) == value } }
func whenNot(_ key: String, _ value: String) -> (Params) -> Bool { { $0.choice(key) != value } }
func whenOn(_ key: String) -> (Params) -> Bool { { $0.bool(key) } }
func whenAny(_ key: String, _ values: [String]) -> (Params) -> Bool { { values.contains($0.choice(key)) } }

import Foundation
import Metal

enum EffectCategory: String, CaseIterable {
    case adjust, color, stylize, light, lens, film, texture, glitch, overlay

    var label: String {
        switch self {
        case .adjust: return "Adjustments"
        case .color: return "Color"
        case .stylize: return "Dither & Print"
        case .light: return "Light & Blur"
        case .lens: return "Lens"
        case .film: return "Film & Camera"
        case .texture: return "Texture & Paper"
        case .glitch: return "Glitch & Retro"
        case .overlay: return "Tracking & Frames"
        }
    }

    var symbol: String {
        switch self {
        case .adjust: return "slider.horizontal.3"
        case .color: return "paintpalette"
        case .stylize: return "circle.grid.3x3"
        case .light: return "sun.max"
        case .lens: return "camera.aperture"
        case .film: return "film"
        case .texture: return "doc.richtext"
        case .glitch: return "tv"
        case .overlay: return "viewfinder"
        }
    }
}

/// A named starting point for an effect's parameters (shown as "Look" in the inspector).
struct EffectLook {
    let id: String
    let name: String
    var group: String? = nil
    let values: [String: Any]
}

protocol Effect {
    var id: String { get }
    var name: String { get }
    var category: EffectCategory { get }
    var symbol: String { get }
    var params: [ParamDef] { get }
    var summary: String { get }
    var looks: [EffectLook] { get }
    var sectionColors: [String: RGBA] { get }
    /// Produces the effect output from `input`. The renderer blends it with `input`
    /// using the layer opacity / blend mode.
    func apply(_ input: MTLTexture, _ p: Params, _ ctx: RenderContext) -> MTLTexture
}

extension Effect {
    var symbol: String { category.symbol }
    var producesAlpha: Bool { false }
    /// One-line description shown in the effect browser.
    var summary: String { "" }
    /// Optional parameter presets for this effect.
    var looks: [EffectLook] { [] }
    /// Colour dot shown next to section titles (e.g. HSL bands).
    var sectionColors: [String: RGBA] { [:] }
}

enum EffectRegistry {
    static let all: [Effect] = [
        // Adjustments
        BasicAdjustEffect(), CurvesEffect(), HSLEffect(), HueSatEffect(), BlackWhiteEffect(),
        SharpenEffect(), InvertEffect(), PosterizeEffect(), ThresholdEffect(),
        // Color
        GradientMapEffect(), LUTEffect(),
        // Dither & print
        DitherEffect(), HalftoneEffect(), PhotocopyEffect(), ThermalPrintEffect(), AsciiEffect(),
        // Light & blur
        GlowEffect(), BloomEffect(), HalationEffect(), LightLeakEffect(), BlurEffect(), TrailEffect(),
        // Lens
        ChromaticAberrationEffect(), VignetteEffect(),
        // Film & camera
        FilmEffect(), GrainEffect(), DigicamEffect(),
        // Texture
        DustEffect(), PaperEffect(), PlasticEffect(),
        // Glitch & retro
        VHSEffect(), CRTEffect(), JPEGEffect(),
        // Tracking & frames
        BlobTrackerEffect(), FrameEffect(),
    ]

    private static let byID: [String: Effect] = Dictionary(uniqueKeysWithValues: all.map { ($0.id, $0) })

    static func effect(_ id: String) -> Effect? { byID[id] }
}

/// Visibility helpers for param defs.
func when(_ key: String, _ value: String) -> (Params) -> Bool { { $0.choice(key) == value } }
func whenNot(_ key: String, _ value: String) -> (Params) -> Bool { { $0.choice(key) != value } }
func whenOn(_ key: String) -> (Params) -> Bool { { $0.bool(key) } }
func whenAny(_ key: String, _ values: [String]) -> (Params) -> Bool { { values.contains($0.choice(key)) } }

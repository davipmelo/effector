import Foundation
import Metal

// Temporary placeholders, replaced stage by stage.
private struct StubEffect {
    static func passthrough(_ input: MTLTexture) -> MTLTexture { input }
}

enum DitherShaders { static let source = "" }
enum HalftoneShaders { static let source = "" }
enum AsciiShaders { static let source = "" }
enum GradientShaders { static let source = "" }
enum GrainShaders { static let source = "" }
enum GlowShaders { static let source = "" }
enum FilmShaders { static let source = "" }
enum TrailShaders { static let source = "" }
enum VHSShaders { static let source = "" }
enum BlobShaders { static let source = "" }

struct GradientMapEffect: Effect { let id = "gradientmap"; let name = "Gradient Map"; let category = EffectCategory.color; let params: [ParamDef] = []; func apply(_ i: MTLTexture, _ p: Params, _ c: RenderContext) -> MTLTexture { i } }
struct LUTEffect: Effect { let id = "lut"; let name = "LUT"; let category = EffectCategory.color; let params: [ParamDef] = []; func apply(_ i: MTLTexture, _ p: Params, _ c: RenderContext) -> MTLTexture { i } }
struct DitherEffect: Effect { let id = "dither"; let name = "Dither"; let category = EffectCategory.stylize; let params: [ParamDef] = []; func apply(_ i: MTLTexture, _ p: Params, _ c: RenderContext) -> MTLTexture { i } }
struct HalftoneEffect: Effect { let id = "halftone"; let name = "Halftone"; let category = EffectCategory.stylize; let params: [ParamDef] = []; func apply(_ i: MTLTexture, _ p: Params, _ c: RenderContext) -> MTLTexture { i } }
struct AsciiEffect: Effect { let id = "ascii"; let name = "ASCII"; let category = EffectCategory.stylize; let params: [ParamDef] = []; func apply(_ i: MTLTexture, _ p: Params, _ c: RenderContext) -> MTLTexture { i } }
struct GlowEffect: Effect { let id = "glow"; let name = "Glow"; let category = EffectCategory.light; let params: [ParamDef] = []; func apply(_ i: MTLTexture, _ p: Params, _ c: RenderContext) -> MTLTexture { i } }
struct BloomEffect: Effect { let id = "bloom"; let name = "Bloom"; let category = EffectCategory.light; let params: [ParamDef] = []; func apply(_ i: MTLTexture, _ p: Params, _ c: RenderContext) -> MTLTexture { i } }
struct HalationEffect: Effect { let id = "halation"; let name = "Halation"; let category = EffectCategory.light; let params: [ParamDef] = []; func apply(_ i: MTLTexture, _ p: Params, _ c: RenderContext) -> MTLTexture { i } }
struct TrailEffect: Effect { let id = "trail"; let name = "Trail / Ghosting"; let category = EffectCategory.light; let params: [ParamDef] = []; func apply(_ i: MTLTexture, _ p: Params, _ c: RenderContext) -> MTLTexture { i } }
struct FilmEffect: Effect { let id = "film"; let name = "Film Emulation"; let category = EffectCategory.film; let params: [ParamDef] = []; func apply(_ i: MTLTexture, _ p: Params, _ c: RenderContext) -> MTLTexture { i } }
struct GrainEffect: Effect { let id = "grain"; let name = "Grain"; let category = EffectCategory.film; let params: [ParamDef] = []; func apply(_ i: MTLTexture, _ p: Params, _ c: RenderContext) -> MTLTexture { i } }
struct VHSEffect: Effect { let id = "vhs"; let name = "VHS"; let category = EffectCategory.glitch; let params: [ParamDef] = []; func apply(_ i: MTLTexture, _ p: Params, _ c: RenderContext) -> MTLTexture { i } }
struct BlobTrackerEffect: Effect { let id = "blob"; let name = "Blob Tracker"; let category = EffectCategory.overlay; let params: [ParamDef] = []; func apply(_ i: MTLTexture, _ p: Params, _ c: RenderContext) -> MTLTexture { i } }

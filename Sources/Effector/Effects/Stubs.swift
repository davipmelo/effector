import Foundation
import Metal

// Temporary placeholders, replaced stage by stage.
private struct StubEffect {
    static func passthrough(_ input: MTLTexture) -> MTLTexture { input }
}

enum AsciiShaders { static let source = "" }
enum VHSShaders { static let source = "" }
enum BlobShaders { static let source = "" }

struct AsciiEffect: Effect { let id = "ascii"; let name = "ASCII"; let category = EffectCategory.stylize; let params: [ParamDef] = []; func apply(_ i: MTLTexture, _ p: Params, _ c: RenderContext) -> MTLTexture { i } }
struct VHSEffect: Effect { let id = "vhs"; let name = "VHS"; let category = EffectCategory.glitch; let params: [ParamDef] = []; func apply(_ i: MTLTexture, _ p: Params, _ c: RenderContext) -> MTLTexture { i } }
struct BlobTrackerEffect: Effect { let id = "blob"; let name = "Blob Tracker"; let category = EffectCategory.overlay; let params: [ParamDef] = []; func apply(_ i: MTLTexture, _ p: Params, _ c: RenderContext) -> MTLTexture { i } }

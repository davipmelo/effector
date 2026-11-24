import Foundation
import Metal

enum GlowShaders {
    static let source = """
    // Highlight extraction. u: threshold, knee, tint rgb (2..4), useTint, saturation
    kernel void k_bright(KIN, texture2d<float> a [[texture(1)]]) {
        GUARD
        float4 s = a.read(gid);
        float3 c = clamp(s.rgb, 0.0, 1.0);
        float l = luma(c);
        float w = smoothstep(u[0] - u[1], u[0] + u[1] + 1e-4, l);
        float3 g = c;
        g = mix(float3(l), g, u[6]);
        if (u[5] > 0.5) g = float3(u[2], u[3], u[4]) * l;
        o.write(float4(g * w, 1.0), gid);
    }
    // Combine glow layers onto base. u: intensity, mode(0 screen, 1 add, 2 soft), w1, w2, w3, tint rgb(5..7), neutral
    kernel void k_addglow(KIN, texture2d<float> base [[texture(1)]], texture2d<float> g1 [[texture(2)]],
                          texture2d<float> g2 [[texture(3)]], texture2d<float> g3 [[texture(4)]]) {
        GUARD
        float4 b = base.read(gid);
        float2 uv = UV;
        float3 g = g1.sample(sLin, uv).rgb * u[2] + g2.sample(sLin, uv).rgb * u[3] + g3.sample(sLin, uv).rgb * u[4];
        g *= float3(u[5], u[6], u[7]) * u[0];
        if (u[8] > 0.5) g = float3(luma(g));
        float3 r;
        int m = int(u[1]);
        if (m == 1) r = b.rgb + g;
        else if (m == 2) r = blendSoftLight(clamp(b.rgb, 0.0, 1.0), clamp(0.5 + g * 0.5, 0.0, 1.0));
        else r = 1.0 - (1.0 - clamp(b.rgb, 0.0, 1.0)) * (1.0 - clamp(g, 0.0, 1.0));
        o.write(float4(clamp(r, 0.0, 1.0), b.a), gid);
    }
    """
}

/// Shared glow pipeline used by Glow, Bloom, Halation and Film.
enum GlowPipeline {
    static func run(_ input: MTLTexture, _ ctx: RenderContext, threshold: Float, knee: Float, radius: Double,
                    intensity: Float, tint: RGBA, colorize: Bool, saturation: Float, mode: Float, multiScale: Bool,
                    neutral: Bool = false) -> MTLTexture {
        let r = max(0.5, ctx.px(radius))
        // work on a downscaled copy for speed
        let f = max(1, Int(r / 6))
        let small = ctx.resize(input, max(1, input.width / f), max(1, input.height / f))
        let bright = ctx.apply("k_bright", [small], [threshold, knee, Float(tint.r), Float(tint.g), Float(tint.b), colorize ? 1 : 0, saturation])
        let sr = r / Double(f)
        let b1 = ctx.blur(bright, sigma: sr * 0.5)
        let b2 = multiScale ? ctx.blur(bright, sigma: sr * 1.2) : b1
        let b3 = multiScale ? ctx.blur(bright, sigma: sr * 3.0) : b1
        let w: [Float] = multiScale ? [0.5, 0.35, 0.3] : [1, 0, 0]
        let t: [Float] = colorize ? [1, 1, 1] : tint.floats
        return ctx.apply("k_addglow", [input, b1, b2, b3], [intensity, mode] + w + t + [neutral ? 1 : 0])
    }
}

struct GlowEffect: Effect {
    let id = "glow"
    let name = "Glow"
    let category = EffectCategory.light
    var symbol: String { "sun.max" }
    var summary: String { "Soft diffusion: the whole image (above a low threshold) gets a dreamy haze." }
    let params: [ParamDef] = [
        .slider("intensity", "Intensity", 0, 200, 60, unit: "%"),
        .slider("radius", "Radius", 1, 200, 20, curve: 2),
        .slider("threshold", "Threshold", 0, 100, 30, unit: "%"),
        .slider("saturation", "Saturation", 0, 200, 100, unit: "%"),
        .choice("mode", "Blend", [("screen", "Screen"), ("add", "Add"), ("soft", "Soft Light")], "screen"),
        .color("tint", "Tint", .white),
    ]
    func apply(_ input: MTLTexture, _ p: Params, _ ctx: RenderContext) -> MTLTexture {
        GlowPipeline.run(input, ctx, threshold: p.f("threshold", 30) / 100, knee: 0.25, radius: p.num("radius", 20),
                         intensity: p.f("intensity", 60) / 100, tint: p.color("tint", .white), colorize: false,
                         saturation: p.f("saturation", 100) / 100,
                         mode: ["screen": 0, "add": 1, "soft": 2][p.choice("mode", "screen")] ?? 0, multiScale: false)
    }
}

struct BloomEffect: Effect {
    let id = "bloom"
    let name = "Bloom"
    let category = EffectCategory.light
    var symbol: String { "sparkles" }
    var summary: String { "Only the brightest highlights spill light, in several sizes, like a lens." }
    let params: [ParamDef] = [
        .slider("intensity", "Intensity", 0, 300, 100, unit: "%"),
        .slider("radius", "Radius", 1, 200, 30, curve: 2),
        .slider("threshold", "Threshold", 0, 100, 70, unit: "%"),
        .slider("knee", "Softness", 0, 50, 15, unit: "%"),
        .slider("saturation", "Saturation", 0, 200, 100, unit: "%"),
        .color("tint", "Tint", .white),
    ]
    func apply(_ input: MTLTexture, _ p: Params, _ ctx: RenderContext) -> MTLTexture {
        GlowPipeline.run(input, ctx, threshold: p.f("threshold", 70) / 100, knee: p.f("knee", 15) / 100, radius: p.num("radius", 30),
                         intensity: p.f("intensity", 100) / 100, tint: p.color("tint", .white), colorize: false,
                         saturation: p.f("saturation", 100) / 100, mode: 1, multiScale: true)
    }
}

struct HalationEffect: Effect {
    let id = "halation"
    let name = "Halation"
    let category = EffectCategory.light
    var symbol: String { "circle.dotted.and.circle" }
    var summary: String { "Red-orange fringe around highlights, from light bouncing in the film base." }
    let params: [ParamDef] = [
        .slider("intensity", "Intensity", 0, 200, 70, unit: "%"),
        .slider("radius", "Radius", 1, 100, 10, curve: 2),
        .slider("threshold", "Threshold", 0, 100, 65, unit: "%"),
        .color("color", "Color", RGBA(hex: "#FF3D14")),
    ]
    func apply(_ input: MTLTexture, _ p: Params, _ ctx: RenderContext) -> MTLTexture {
        GlowPipeline.run(input, ctx, threshold: p.f("threshold", 65) / 100, knee: 0.15, radius: p.num("radius", 10),
                         intensity: p.f("intensity", 70) / 100, tint: p.color("color", RGBA(hex: "#FF3D14")), colorize: true,
                         saturation: 1, mode: 0, multiScale: true)
    }
}

// MARK: - Trail / ghosting

enum TrailShaders {
    static let source = """
    // u: mode(0 dir,1 zoom,2 spin), angle, distance(px), copies, decay, blend, rgbShift, cx, cy, opacity
    kernel void k_trail(KIN, texture2d<float> a [[texture(1)]]) {
        GUARD
        float2 size = OSIZE;
        float2 uv = UV;
        float4 base = a.read(gid);
        float3 col = base.rgb;
        int mode = int(u[0]);
        int n = int(clamp(u[3], 1.0, 48.0));
        float ang = u[1] * 0.0174533;
        float2 dir = float2(cos(ang), -sin(ang)) * u[2] / size;
        float2 center = float2(u[7], u[8]);
        for (int i = n; i >= 1; i--) {
            float t = float(i) / float(n);
            float w = pow(u[4], float(i - 1)) * u[9];
            float3 s;
            for (int ch = 0; ch < 3; ch++) {
                float k = t * (1.0 + (float(ch) - 1.0) * u[6]);
                float2 suv;
                if (mode == 1) {
                    float z = 1.0 + u[2] / max(size.x, size.y) * 4.0 * k;
                    suv = center + (uv - center) / z;
                } else if (mode == 2) {
                    float r = u[1] * 0.0174533 * k * 0.25;
                    float2 d = (uv - center) * size;
                    float c = cos(r), sn = sin(r);
                    d = float2(c * d.x - sn * d.y, sn * d.x + c * d.y);
                    suv = center + d / size;
                } else {
                    suv = uv - dir * k;
                }
                s[ch] = a.sample(sLin, suv)[ch];
            }
            int bm = int(u[5]);
            float3 r;
            if (bm == 0) r = max(col, s);
            else if (bm == 1) r = 1.0 - (1.0 - col) * (1.0 - s);
            else if (bm == 2) r = s;
            else if (bm == 3) r = abs(col - s);
            else r = min(col, s);
            col = mix(col, r, clamp(w, 0.0, 1.0));
        }
        o.write(float4(clamp(col, 0.0, 1.0), base.a), gid);
    }
    """
}

struct TrailEffect: Effect {
    let id = "trail"
    let name = "Trail / Ghosting"
    let category = EffectCategory.light
    var symbol: String { "wind" }
    var summary: String { "Ghost copies: directional, zoom or spin motion trails." }
    let params: [ParamDef] = [
        .segmented("mode", "Mode", [("dir", "Directional"), ("zoom", "Zoom"), ("spin", "Spin")], "dir", section: "Trail"),
        .slider("angle", "Angle", -180, 180, 0, unit: "°", section: "Trail"),
        .slider("distance", "Distance", 0, 400, 60, curve: 1.8, section: "Trail"),
        .slider("copies", "Copies", 1, 48, 8, step: 1, section: "Trail"),
        .slider("decay", "Decay", 0, 100, 75, unit: "%", section: "Trail"),
        .slider("opacity", "Ghost Opacity", 0, 100, 60, unit: "%", section: "Trail"),
        .choice("blend", "Ghost Blend", [("lighten", "Lighten"), ("screen", "Screen"), ("normal", "Normal"),
                                         ("difference", "Difference"), ("darken", "Darken")], "lighten", section: "Trail"),
        .slider("rgbShift", "RGB Split", 0, 100, 0, unit: "%", section: "Trail"),
        .slider("cx", "Center X", 0, 100, 50, unit: "%", section: "Center", visible: whenNot("mode", "dir")),
        .slider("cy", "Center Y", 0, 100, 50, unit: "%", section: "Center", visible: whenNot("mode", "dir")),
    ]
    func apply(_ input: MTLTexture, _ p: Params, _ ctx: RenderContext) -> MTLTexture {
        let mode: Float = ["dir": 0, "zoom": 1, "spin": 2][p.choice("mode", "dir")] ?? 0
        let blend: Float = ["lighten": 0, "screen": 1, "normal": 2, "difference": 3, "darken": 4][p.choice("blend", "lighten")] ?? 0
        return ctx.apply("k_trail", [input], [mode, p.f("angle"), Float(ctx.px(p.num("distance", 60))), p.f("copies", 8),
                                              p.f("decay", 75) / 100, blend, p.f("rgbShift") / 100 * 0.5,
                                              p.f("cx", 50) / 100, p.f("cy", 50) / 100, p.f("opacity", 60) / 100])
    }
}

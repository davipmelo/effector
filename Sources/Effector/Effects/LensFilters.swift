import Foundation
import Metal

enum LensFilterShaders {
    static let source = """
    // One pass of an exponential directional streak. u: dir.x, dir.y, step(px), falloff(px)
    kernel void k_streak(KIN, texture2d<float> a [[texture(1)]]) {
        GUARD
        float2 size = OSIZE;
        float2 uv = UV;
        float2 d = float2(u[0], u[1]) * u[2] / size;
        // weighted maximum: a point light draws a full-strength tapering line,
        // while large flat bright areas are not amplified
        float3 mx = a.sample(sLin, uv).rgb;
        float3 avg = mx;
        float wsum = 1.0;
        for (int i = 1; i <= 8; i++) {
            float w = exp(-float(i) * u[2] / max(u[3], 1e-3));
            float3 s1 = a.sample(sLin, uv + d * float(i)).rgb, s2 = a.sample(sLin, uv - d * float(i)).rgb;
            mx = max(mx, max(s1, s2) * w);
            avg += (s1 + s2) * w;
            wsum += 2.0 * w;
        }
        o.write(float4(mix(mx, avg / wsum, u[4] > 0.5 ? 1.0 : 0.3), 1.0), gid);
    }
    // Keep small lights, suppress large bright areas. u[0] amount
    kernel void k_points(KIN, texture2d<float> a [[texture(1)]], texture2d<float> b [[texture(2)]]) {
        GUARD
        float3 c = a.read(gid).rgb;
        float area = luma(b.sample(sLin, UV).rgb);
        o.write(float4(c * (1.0 - u[0] * smoothstep(0.08, 0.45, area)), 1.0), gid);
    }
    // a + b * gain (small working textures). u[0] gain
    kernel void k_addtex(KIN, texture2d<float> a [[texture(1)]], texture2d<float> b [[texture(2)]]) {
        GUARD
        float2 uv = UV;
        o.write(float4(a.sample(sLin, uv).rgb + b.sample(sLin, uv).rgb * u[0], 1.0), gid);
    }
    // Spectral combine: r from a, g from b, b from c
    kernel void k_rgbsplit(KIN, texture2d<float> a [[texture(1)]], texture2d<float> b [[texture(2)]], texture2d<float> c [[texture(3)]]) {
        GUARD
        float2 uv = UV;
        o.write(float4(a.sample(sLin, uv).r, b.sample(sLin, uv).g, c.sample(sLin, uv).b, 1.0), gid);
    }
    // Ring from two blurs (difference of gaussians). u: ratio
    kernel void k_ring(KIN, texture2d<float> a [[texture(1)]], texture2d<float> b [[texture(2)]]) {
        GUARD
        float2 uv = UV;
        o.write(float4(max(a.sample(sLin, uv).rgb - b.sample(sLin, uv).rgb * u[0], 0.0) * 2.5, 1.0), gid);
    }

    // Prism. u: mode (0 side,1 ring,2 full), angle, offset(px), dispersion, intensity, coverage, feather, copies, blend
    kernel void k_prism(KIN, texture2d<float> a [[texture(1)]], texture2d<float> soft [[texture(2)]]) {
        GUARD
        float2 size = OSIZE;
        float2 uv = UV;
        float an = u[1] * 0.0174533;
        float2 dir = float2(cos(an), -sin(an));
        int mode = int(u[0]);
        float m = 1.0;
        float cov = u[5], fe = max(0.005, u[6]);
        if (mode == 0) {
            float t = dot(uv - 0.5, dir) + 0.5;
            m = smoothstep(1.0 - cov - fe, 1.0 - cov + fe, t);
        } else if (mode == 1) {
            float r = length((uv - 0.5) * float2(size.x / max(size.x, size.y), size.y / max(size.x, size.y))) * 1.42;
            m = smoothstep(1.0 - cov - fe, 1.0 - cov + fe, r);
        }
        float4 base = a.read(gid);
        float3 ghost = float3(0.0);
        int n = int(clamp(u[7], 1.0, 4.0));
        float wsum = 0.0;
        for (int k = 1; k <= n; k++) {
            float2 off = dir * u[2] * float(k) / size;
            float disp = u[3];
            float r = soft.sample(sLin, uv - off * (1.0 + disp)).r;
            float g = soft.sample(sLin, uv - off).g;
            float b = soft.sample(sLin, uv - off * (1.0 - disp)).b;
            float w = pow(0.6, float(k - 1));
            ghost += float3(r, g, b) * w;
            wsum += w;
        }
        ghost /= wsum;
        float amt = clamp(u[4] * m, 0.0, 1.5);
        float3 c = base.rgb;
        int bm = int(u[8]);
        if (bm == 1) c = mix(c, max(c, ghost), clamp(amt, 0.0, 1.0));
        else if (bm == 2) c = mix(c, ghost, clamp(amt * 0.6, 0.0, 1.0));
        else c = 1.0 - (1.0 - c) * (1.0 - clamp(ghost * amt, 0.0, 1.0));
        o.write(float4(clamp(c, 0.0, 1.0), base.a), gid);
    }

    constexpr sampler sMir(coord::normalized, filter::linear, address::mirrored_repeat);

    // Kaleidoscope. u: segments, rotation, cx, cy, zoom, twist, mirror, dispersion, mix, rings, sx, sy
    kernel void k_kaleido(KIN, texture2d<float> a [[texture(1)]], texture2d<float> soft [[texture(2)]]) {
        GUARD
        float2 size = OSIZE;
        float asp = size.x / size.y;
        float2 uv = UV;
        float2 q = (uv - float2(u[2], u[3])) * float2(asp, 1.0);
        float r = length(q) / max(0.05, u[4]);
        float N = max(2.0, floor(u[0]));
        float seg = 6.2831853 / N;
        if (u[9] > 0.5) {
            float k = r * u[9];
            r = abs(fract(k * 0.5) * 2.0 - 1.0) * 0.5;
        }
        float3 col;
        for (int ch = 0; ch < 3; ch++) {
            float th = atan2(q.y, q.x) + u[1] + u[5] * r + (float(ch) - 1.0) * u[7] * seg * 0.12;
            float k = floor(th / seg);
            float tm = th - k * seg;
            if (u[6] > 0.5 && fmod(abs(k), 2.0) > 0.5) tm = seg - tm;
            float2 sp = float2(cos(tm), sin(tm)) * r;
            float2 suv = float2(u[10], u[11]) + sp / float2(asp, 1.0);
            col[ch] = soft.sample(sMir, suv)[ch];
        }
        float4 base = a.read(gid);
        o.write(float4(mix(base.rgb, col, clamp(u[8], 0.0, 1.0)), base.a), gid);
    }

    // Crystal facets. u: unit, cell(ref px), refract(px), dispersion, edges, seed, mix, stretch
    kernel void k_facets(KIN, texture2d<float> a [[texture(1)]], texture2d<float> soft [[texture(2)]]) {
        GUARD
        float2 size = OSIZE;
        float2 p = float2(gid) + 0.5;
        float2 q = p / max(u[0], 1e-3) / max(4.0, u[1]);
        q *= float2(1.0 - u[7] * 0.7, 1.0 + u[7] * 0.7);
        float4 v = voro(q, u[5]);
        float2 dir = float2(v.z - 0.5, v.w - 0.5) * 2.0;
        float2 off = dir * u[2] / size;
        float3 col;
        for (int ch = 0; ch < 3; ch++) {
            float k = 1.0 + (float(ch) - 1.0) * u[3] * 0.5;
            col[ch] = soft.sample(sMir, p / size + off * k)[ch];
        }
        float edge = 1.0 - smoothstep(0.0, 0.06, v.y - v.x);
        col += edge * u[4] * 0.6 * (0.5 + v.z);
        col *= 1.0 + (v.w - 0.5) * 0.12 * u[4];
        float4 base = a.read(gid);
        o.write(float4(clamp(mix(base.rgb, col, clamp(u[6], 0.0, 1.0)), 0.0, 1.0), base.a), gid);
    }

    // Focus mix for lens blur. u: mode (0 tilt-shift band, 1 radial, 2 full), pos x, pos y, size, falloff, angle, saturation
    kernel void k_focusmix(KIN, texture2d<float> base [[texture(1)]], texture2d<float> b1 [[texture(2)]], texture2d<float> b2 [[texture(3)]]) {
        GUARD
        float2 size = OSIZE;
        float2 uv = UV;
        int mode = int(u[0]);
        float m;
        float2 c = float2(u[1], u[2]);
        if (mode == 0) {
            float an = u[5] * 0.0174533;
            float2 nrm = float2(-sin(an), cos(an));
            float d = abs(dot((uv - c) * float2(size.x / size.y, 1.0), nrm));
            m = smoothstep(u[3] * 0.5, u[3] * 0.5 + max(0.01, u[4]), d);
        } else if (mode == 1) {
            float d = length((uv - c) * float2(size.x / size.y, 1.0));
            m = smoothstep(u[3] * 0.5, u[3] * 0.5 + max(0.01, u[4]), d);
        } else {
            m = 1.0;
        }
        float4 s = base.read(gid);
        float3 x1 = b1.sample(sLin, uv).rgb, x2 = b2.sample(sLin, uv).rgb;
        float3 r = m < 0.5 ? mix(s.rgb, x1, m * 2.0) : mix(x1, x2, (m - 0.5) * 2.0);
        r = mix(float3(luma(r)), r, u[6]);
        o.write(float4(clamp(r, 0.0, 1.0), s.a), gid);
    }
    // Highlight boost before blurring (fake bokeh energy). u: boost
    kernel void k_hlboost(KIN, texture2d<float> a [[texture(1)]]) {
        GUARD
        float4 s = a.read(gid);
        float3 c = clamp(s.rgb, 0.0, 1.0);
        float l = luma(c);
        o.write(float4(c * (1.0 + u[0] * 4.0 * smoothstep(0.7, 1.0, l)), s.a), gid);
    }
    """
}

/// Shared helpers for streak / glow style lens filters.
enum LensFilterKit {
    /// Downscale factor and bright-pass of the input.
    static func brights(_ input: MTLTexture, _ ctx: RenderContext, threshold: Float, knee: Float = 0.15, saturation: Float = 1,
                        factor: Int = 4) -> MTLTexture {
        let small = ctx.resize(input, max(1, input.width / factor), max(1, input.height / factor))
        return ctx.apply("k_bright", [small], [threshold, knee, 1, 1, 1, 0, saturation])
    }

    /// Exponential streak along `angle` (degrees), `length` in working-texture pixels.
    /// Suppresses big bright regions so only point-like lights produce streaks.
    static func pointsOnly(_ b: MTLTexture, _ ctx: RenderContext, amount: Float) -> MTLTexture {
        guard amount > 0.001 else { return b }
        let area = ctx.blur(b, sigma: Double(max(b.width, b.height)) * 0.012)
        return ctx.apply("k_points", [b, area], [amount])
    }

    static func streak(_ t: MTLTexture, _ ctx: RenderContext, angle: Double, length: Double) -> MTLTexture {
        let a = angle * .pi / 180
        let dx = Float(cos(a)), dy = Float(-sin(a))
        var out = t
        var step: Float = 1
        let L = Float(max(1, length))
        while step <= L * 1.2 {
            out = ctx.apply("k_streak", [out], [dx, dy, step, L, 0])
            step *= 9
        }
        // a final averaging pass smooths the tap pattern into a clean taper
        return ctx.apply("k_streak", [out], [dx, dy, max(1, L / 14), L * 0.6, 1])
    }

    static func composite(_ input: MTLTexture, _ glow: MTLTexture, _ ctx: RenderContext, intensity: Float, tint: RGBA, add: Bool) -> MTLTexture {
        ctx.apply("k_addglow", [input, glow, glow, glow], [intensity, add ? 1 : 0, 1, 0, 0] + tint.floats + [0])
    }
}

// MARK: - Anamorphic flare

struct AnamorphicEffect: Effect {
    let id = "anamorphic"
    let name = "Anamorphic Flare"
    let category = EffectCategory.lens
    var symbol: String { "line.3.horizontal" }
    var summary: String { "Long horizontal streaks from bright lights, like an anamorphic cinema lens." }
    let params: [ParamDef] = [
        .slider("intensity", "Intensity", 0, 300, 120, step: 1, unit: "%", section: "Streak"),
        .slider("length", "Length", 5, 150, 60, step: 1, unit: "%", section: "Streak"),
        .slider("threshold", "Threshold", 0, 100, 72, step: 1, unit: "%", section: "Streak"),
        .slider("thickness", "Thickness", 0, 10, 1.5, step: 0.1, section: "Streak"),
        .slider("angle", "Angle", -45, 45, 0, step: 0.5, unit: "°", section: "Streak"),
        .color("color", "Color", RGBA(hex: "#3D7CFF"), section: "Color"),
        .slider("colorMix", "Color Amount", 0, 100, 85, step: 1, unit: "%", section: "Color"),
        .toggle("spectral", "Spectral Edges", false, section: "Color"),
        .slider("smallLights", "Small Lights Only", 0, 100, 60, step: 1, unit: "%", section: "Streak"),
        .sectionSwitch("glowOn", section: "Glow"),
        .slider("glow", "Wide Glow", 0, 100, 25, step: 1, unit: "%", section: "Glow"),
    ]
    var looks: [EffectLook] {
        [
            EffectLook(id: "blue", name: "Classic Blue Streak", values: [:]),
            EffectLook(id: "amber", name: "Amber Streak", values: ["color": RGBA(hex: "#FF9A3D")]),
            EffectLook(id: "clean", name: "Clean White", values: ["colorMix": 0.0, "intensity": 90.0]),
            EffectLook(id: "scifi", name: "Sci-Fi", values: ["intensity": 220.0, "length": 120.0, "threshold": 60.0, "spectral": true]),
            EffectLook(id: "subtle", name: "Subtle", values: ["intensity": 60.0, "length": 35.0, "glow": 10.0]),
        ]
    }
    func apply(_ input: MTLTexture, _ p: Params, _ ctx: RenderContext) -> MTLTexture {
        let f = 4
        let mix = p.f("colorMix", 85) / 100
        var b = LensFilterKit.brights(input, ctx, threshold: p.f("threshold", 72) / 100, saturation: 1 - mix, factor: f)
        b = LensFilterKit.pointsOnly(b, ctx, amount: p.f("smallLights", 60) / 100)
        let th = ctx.px(p.num("thickness", 1.5)) / Double(f)
        if th > 0.3 { b = ctx.blur(b, sigma: th) }
        let L = p.num("length", 60) / 100 * Double(b.width) * 0.35
        let ang = p.num("angle")
        var s: MTLTexture
        if p.bool("spectral") {
            let r = LensFilterKit.streak(b, ctx, angle: ang, length: L * 1.15)
            let g = LensFilterKit.streak(b, ctx, angle: ang, length: L)
            let bl = LensFilterKit.streak(b, ctx, angle: ang, length: L * 0.85)
            s = ctx.apply("k_rgbsplit", [r, g, bl])
        } else {
            s = LensFilterKit.streak(b, ctx, angle: ang, length: L)
        }
        let gw = p.bool("glowOn", true) ? p.f("glow", 25) / 100 : 0
        if gw > 0.001 { s = ctx.apply("k_addtex", [s, ctx.blur(b, sigma: Double(b.width) * 0.02)], [gw * 2]) }
        let c = p.color("color", RGBA(hex: "#3D7CFF"))
        let tint = RGBA(r: 1 + (c.r * 1.6 - 1) * Double(mix), g: 1 + (c.g * 1.6 - 1) * Double(mix), b: 1 + (c.b * 1.6 - 1) * Double(mix))
        return LensFilterKit.composite(input, s, ctx, intensity: p.f("intensity", 120) / 100 * 0.8, tint: tint, add: true)
    }
}

// MARK: - Star filter

struct StarFilterEffect: Effect {
    let id = "star"
    let name = "Star Filter"
    let category = EffectCategory.lens
    var symbol: String { "sparkle" }
    var summary: String { "Cross-screen filter: 4, 6 or 8-point stars on every highlight." }
    let params: [ParamDef] = [
        .segmented("points", "Points", [("4", "4"), ("6", "6"), ("8", "8")], "4", section: "Stars"),
        .slider("intensity", "Intensity", 0, 300, 110, step: 1, unit: "%", section: "Stars"),
        .slider("length", "Length", 2, 100, 14, step: 1, unit: "%", curve: 1.4, section: "Stars"),
        .slider("threshold", "Threshold", 0, 100, 86, step: 1, unit: "%", section: "Stars"),
        .slider("angle", "Rotation", 0, 90, 45, step: 0.5, unit: "°", section: "Stars"),
        .slider("thickness", "Thickness", 0, 6, 0.6, step: 0.1, section: "Stars"),
        .slider("smallLights", "Small Lights Only", 0, 100, 85, step: 1, unit: "%", section: "Stars"),
        .toggle("spectral", "Rainbow", false, section: "Color"),
        .color("tint", "Tint", .white, section: "Color"),
    ]
    func apply(_ input: MTLTexture, _ p: Params, _ ctx: RenderContext) -> MTLTexture {
        let f = 4
        var b = LensFilterKit.brights(input, ctx, threshold: p.f("threshold", 86) / 100, knee: 0.08, factor: f)
        b = LensFilterKit.pointsOnly(b, ctx, amount: p.f("smallLights", 85) / 100)
        let th = ctx.px(p.num("thickness", 0.6)) / Double(f)
        if th > 0.3 { b = ctx.blur(b, sigma: th) }
        let n = Int(p.choice("points", "4")) ?? 4
        let L = p.num("length", 14) / 100 * Double(max(b.width, b.height)) * 0.25
        var acc: MTLTexture?
        for i in 0..<(n / 2) {
            let ang = p.num("angle", 45) + Double(i) * 180 / Double(n / 2)
            let s: MTLTexture
            if p.bool("spectral") {
                s = ctx.apply("k_rgbsplit", [LensFilterKit.streak(b, ctx, angle: ang, length: L * 1.2),
                                             LensFilterKit.streak(b, ctx, angle: ang, length: L),
                                             LensFilterKit.streak(b, ctx, angle: ang, length: L * 0.8)])
            } else {
                s = LensFilterKit.streak(b, ctx, angle: ang, length: L)
            }
            acc = acc.map { ctx.apply("k_addtex", [$0, s], [1]) } ?? s
        }
        guard let glow = acc else { return input }
        return LensFilterKit.composite(input, glow, ctx, intensity: p.f("intensity", 110) / 100 * 0.8, tint: p.color("tint", .white), add: true)
    }
}

// MARK: - Prism

struct PrismEffect: Effect {
    let id = "prism"
    let name = "Prism & Kaleidoscope"
    let category = EffectCategory.lens
    var symbol: String { "triangle.lefthalf.filled" }
    var summary: String { "Prism ghosts, a full kaleidoscope or crystal facets — refract, mirror and spin the image." }
    let params: [ParamDef] = [
        .segmented("type", "Type", [("ghosts", "Prism"), ("kaleido", "Kaleidoscope"), ("facets", "Crystal")], "ghosts", section: "Prism"),
        // prism ghosts
        .choice("mode", "Placement", [("side", "One Side"), ("ring", "Around the Edges"), ("full", "Whole Frame")], "side",
                section: "Prism", visible: when("type", "ghosts")),
        .slider("angle", "Direction", -180, 180, 0, step: 1, unit: "°", section: "Prism", visible: when("type", "ghosts")),
        .slider("coverage", "Coverage", 5, 100, 40, step: 1, unit: "%", section: "Prism",
                visible: { $0.choice("type", "ghosts") == "ghosts" && $0.choice("mode", "side") != "full" }),
        .slider("feather", "Feather", 0, 50, 15, step: 1, unit: "%", section: "Prism",
                visible: { $0.choice("type", "ghosts") == "ghosts" && $0.choice("mode", "side") != "full" }),
        .slider("offset", "Offset", 0, 200, 40, step: 1, curve: 1.5, section: "Prism", visible: when("type", "ghosts")),
        .slider("copies", "Copies", 1, 4, 2, step: 1, section: "Prism", visible: when("type", "ghosts")),
        .choice("blend", "Blend", [("screen", "Screen"), ("lighten", "Lighten"), ("normal", "Normal")], "screen", section: "Prism",
                visible: when("type", "ghosts")),
        // kaleidoscope
        .slider("segments", "Segments", 2, 32, 6, step: 1, section: "Kaleidoscope", visible: when("type", "kaleido")),
        .toggle("mirror", "Mirror Segments", true, section: "Kaleidoscope", visible: when("type", "kaleido")),
        .slider("rotation", "Rotation", -180, 180, 0, step: 1, unit: "°", section: "Kaleidoscope", visible: when("type", "kaleido")),
        .slider("zoom", "Zoom", 10, 400, 100, step: 1, unit: "%", curve: 1.5, section: "Kaleidoscope", visible: when("type", "kaleido")),
        .slider("twist", "Twist", -400, 400, 0, step: 1, unit: "°", section: "Kaleidoscope", visible: when("type", "kaleido")),
        .slider("rings", "Rings", 0, 12, 0, step: 1, section: "Kaleidoscope", visible: when("type", "kaleido")),
        .slider("cx", "Center X", 0, 100, 50, step: 1, unit: "%", section: "Kaleidoscope", visible: when("type", "kaleido")),
        .slider("cy", "Center Y", 0, 100, 50, step: 1, unit: "%", section: "Kaleidoscope", visible: when("type", "kaleido")),
        .slider("sx", "Source X", 0, 100, 50, step: 1, unit: "%", section: "Kaleidoscope", visible: when("type", "kaleido")),
        .slider("sy", "Source Y", 0, 100, 50, step: 1, unit: "%", section: "Kaleidoscope", visible: when("type", "kaleido")),
        // facets
        .slider("cell", "Facet Size", 10, 400, 90, step: 1, curve: 1.6, section: "Crystal", visible: when("type", "facets")),
        .slider("refract", "Refraction", 0, 200, 40, step: 1, curve: 1.4, section: "Crystal", visible: when("type", "facets")),
        .slider("edges", "Edge Shine", 0, 100, 35, step: 1, unit: "%", section: "Crystal", visible: when("type", "facets")),
        .slider("stretch", "Stretch", -90, 90, 0, step: 1, unit: "%", section: "Crystal", visible: when("type", "facets")),
        .seed(section: "Crystal"),
        // shared
        .slider("dispersion", "Rainbow Dispersion", 0, 100, 35, step: 1, unit: "%", section: "Color & Mix"),
        .slider("softness", "Softness", 0, 30, 3, step: 0.5, section: "Color & Mix"),
        .slider("intensity", "Amount", 0, 150, 70, step: 1, unit: "%", section: "Color & Mix"),
    ]
    var looks: [EffectLook] {
        [
            EffectLook(id: "side", name: "Rainbow Side", group: "Prism", values: [:]),
            EffectLook(id: "kaleido", name: "Edge Ghosts", group: "Prism", values: ["mode": "ring", "coverage": 45.0, "copies": 3, "offset": 60.0]),
            EffectLook(id: "double", name: "Double Exposure", group: "Prism", values: ["mode": "full", "offset": 120.0, "copies": 1, "dispersion": 15.0,
                                                                                    "intensity": 50.0, "blend": "lighten"]),
            EffectLook(id: "dreamy", name: "Dreamy Refraction", group: "Prism", values: ["softness": 14.0, "dispersion": 70.0, "offset": 25.0, "copies": 3]),
            EffectLook(id: "k6", name: "Classic Kaleidoscope", group: "Kaleidoscope", values: ["type": "kaleido", "segments": 6, "intensity": 100.0,
                                                                                            "dispersion": 10.0]),
            EffectLook(id: "mandala", name: "Mandala", group: "Kaleidoscope", values: ["type": "kaleido", "segments": 12, "rings": 3, "intensity": 100.0,
                                                                                     "zoom": 140.0, "dispersion": 25.0]),
            EffectLook(id: "vortex", name: "Vortex", group: "Kaleidoscope", values: ["type": "kaleido", "segments": 8, "twist": 220.0, "intensity": 100.0,
                                                                                   "dispersion": 40.0]),
            EffectLook(id: "mirror2", name: "Mirror World", group: "Kaleidoscope", values: ["type": "kaleido", "segments": 2, "intensity": 100.0,
                                                                                          "dispersion": 0.0]),
            EffectLook(id: "trip", name: "Psychedelic Trip", group: "Kaleidoscope", values: ["type": "kaleido", "segments": 16, "twist": -160.0,
                                                                                           "rings": 5, "zoom": 70.0, "dispersion": 90.0, "intensity": 100.0]),
            EffectLook(id: "crystal", name: "Crystal Lens", group: "Crystal", values: ["type": "facets", "intensity": 100.0]),
            EffectLook(id: "shards", name: "Glass Shards", group: "Crystal", values: ["type": "facets", "cell": 160.0, "refract": 90.0, "edges": 60.0,
                                                                                    "stretch": 60.0, "intensity": 100.0]),
            EffectLook(id: "mosaic", name: "Fine Mosaic", group: "Crystal", values: ["type": "facets", "cell": 25.0, "refract": 15.0, "edges": 20.0,
                                                                                   "intensity": 100.0, "dispersion": 10.0]),
        ]
    }
    func apply(_ input: MTLTexture, _ p: Params, _ ctx: RenderContext) -> MTLTexture {
        let soft = ctx.blur(input, sigma: ctx.px(p.num("softness", 3)))
        let amt = p.f("intensity", 70) / 100
        let disp = p.f("dispersion", 35) / 100
        switch p.choice("type", "ghosts") {
        case "kaleido":
            return ctx.apply("k_kaleido", [input, soft], [p.f("segments", 6), p.f("rotation") * .pi / 180, p.f("cx", 50) / 100, p.f("cy", 50) / 100,
                                                          p.f("zoom", 100) / 100, p.f("twist") * .pi / 180, p.bool("mirror", true) ? 1 : 0,
                                                          disp, amt, p.f("rings"), p.f("sx", 50) / 100, p.f("sy", 50) / 100])
        case "facets":
            return ctx.apply("k_facets", [input, soft], [Float(ctx.scale), p.f("cell", 90), Float(ctx.px(p.num("refract", 40))),
                                                         disp, p.f("edges", 35) / 100, p.f("seed", 1) * 3.7, amt, p.f("stretch") / 100])
        default:
            let mode: Float = ["side": 0, "ring": 1, "full": 2][p.choice("mode", "side")] ?? 0
            let blend: Float = ["screen": 0, "lighten": 1, "normal": 2][p.choice("blend", "screen")] ?? 0
            return ctx.apply("k_prism", [input, soft], [mode, p.f("angle"), Float(ctx.px(p.num("offset", 40))), disp * 0.6,
                                                        amt, p.f("coverage", 40) / 100, p.f("feather", 15) / 100,
                                                        p.f("copies", 2), blend])
        }
    }
}

// MARK: - Halo

struct HaloEffect: Effect {
    let id = "halo"
    let name = "Halo"
    let category = EffectCategory.lens
    var symbol: String { "circle.circle" }
    var summary: String { "Rings of light around bright sources, optionally with rainbow edges." }
    let params: [ParamDef] = [
        .slider("intensity", "Intensity", 0, 300, 160, step: 1, unit: "%"),
        .slider("radius", "Radius", 5, 300, 60, step: 1, curve: 1.6),
        .slider("width", "Ring Width", 5, 95, 45, step: 1, unit: "%"),
        .slider("threshold", "Threshold", 0, 100, 75, step: 1, unit: "%"),
        .toggle("spectral", "Rainbow", true),
        .color("tint", "Tint", .white),
    ]
    func apply(_ input: MTLTexture, _ p: Params, _ ctx: RenderContext) -> MTLTexture {
        let r = max(2, ctx.px(p.num("radius", 60)))
        let f = max(1, Int(r / 8))
        let b = LensFilterKit.brights(input, ctx, threshold: p.f("threshold", 75) / 100, knee: 0.1, factor: f)
        let sr = r / Double(f) / 2.2
        let inner = 1 - p.num("width", 45) / 100
        func ring(_ k: Double) -> MTLTexture {
            ctx.apply("k_ring", [ctx.blur(b, sigma: sr * k), ctx.blur(b, sigma: sr * k * max(0.05, inner))], [Float(0.95)])
        }
        let glow = p.bool("spectral", true) ? ctx.apply("k_rgbsplit", [ring(1.12), ring(1.0), ring(0.88)]) : ring(1)
        return LensFilterKit.composite(input, glow, ctx, intensity: p.f("intensity", 160) / 100, tint: p.color("tint", .white), add: false)
    }
}

// MARK: - Lens blur / tilt-shift

struct LensBlurEffect: Effect {
    let id = "lensblur"
    let name = "Lens Blur & Tilt-Shift"
    let category = EffectCategory.blur
    var symbol: String { "camera.metering.center.weighted" }
    var summary: String { "Shallow focus: a sharp band (tilt-shift miniature) or a sharp centre with bright bokeh." }
    let params: [ParamDef] = [
        .segmented("mode", "Focus", [("band", "Band"), ("radial", "Radial"), ("full", "Whole")], "band", section: "Focus"),
        .slider("x", "Focus X", 0, 100, 50, step: 1, unit: "%", section: "Focus", visible: when("mode", "radial")),
        .slider("y", "Focus Y", 0, 100, 55, step: 1, unit: "%", section: "Focus", visible: whenNot("mode", "full")),
        .slider("size", "Sharp Area", 0, 100, 20, step: 1, unit: "%", section: "Focus", visible: whenNot("mode", "full")),
        .slider("falloff", "Transition", 1, 100, 25, step: 1, unit: "%", section: "Focus", visible: whenNot("mode", "full")),
        .slider("angle", "Angle", -90, 90, 0, step: 1, unit: "°", section: "Focus", visible: when("mode", "band")),
        .slider("amount", "Blur", 0, 100, 18, step: 0.5, curve: 1.6, section: "Blur"),
        .slider("bokeh", "Bokeh Brightness", 0, 100, 40, step: 1, unit: "%", section: "Blur"),
        .slider("saturation", "Saturation", 0, 200, 100, step: 1, unit: "%", section: "Blur"),
    ]
    var looks: [EffectLook] {
        [
            EffectLook(id: "mini", name: "Miniature (Tilt-Shift)", values: ["mode": "band", "saturation": 140.0, "amount": 14.0]),
            EffectLook(id: "portrait", name: "Portrait f/1.4", values: ["mode": "radial", "size": 30.0, "falloff": 35.0, "amount": 22.0, "y": 45.0]),
            EffectLook(id: "dream", name: "Dreamy Bokeh", values: ["mode": "full", "amount": 30.0, "bokeh": 80.0]),
        ]
    }
    func apply(_ input: MTLTexture, _ p: Params, _ ctx: RenderContext) -> MTLTexture {
        let amt = ctx.px(p.num("amount", 18))
        let boost = p.f("bokeh", 40) / 100
        let pre = ctx.apply("k_hlboost", [input], [boost])
        let b1 = ctx.blur(pre, sigma: max(0.3, amt * 0.35))
        let b2 = ctx.blur(pre, sigma: max(0.3, amt))
        let mode: Float = ["band": 0, "radial": 1, "full": 2][p.choice("mode", "band")] ?? 0
        let mixed = ctx.apply("k_focusmix", [input, b1, b2], [mode, p.f("x", 50) / 100, p.f("y", 55) / 100, p.f("size", 20) / 100,
                                                              p.f("falloff", 25) / 100, p.f("angle"), p.f("saturation", 100) / 100])
        return ctx.apply("k_clamp", [mixed])
    }
}


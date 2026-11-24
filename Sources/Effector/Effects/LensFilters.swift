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

    // Diffusion filter. u: strength, glow, haze, lift, contrast, warmth, glowTint rgb (6..8)
    kernel void k_diffuse(KIN, texture2d<float> base [[texture(1)]], texture2d<float> g1 [[texture(2)]],
                          texture2d<float> g2 [[texture(3)]], texture2d<float> hz [[texture(4)]]) {
        GUARD
        float2 uv = UV;
        float4 s = base.read(gid);
        float3 c = clamp(s.rgb, 0.0, 1.0);
        float k = u[0];
        float w = u[5];
        float3 tint = float3(u[6], u[7], u[8]) * float3(1.0 + 0.12 * w, 1.0 + 0.02 * w, 1.0 - 0.14 * w);
        float3 g = (g1.sample(sLin, uv).rgb * 0.65 + g2.sample(sLin, uv).rgb * 0.5) * tint * u[1] * k;
        c = 1.0 - (1.0 - c) * (1.0 - clamp(g, 0.0, 1.0));
        float3 h = hz.sample(sLin, uv).rgb;
        c = mix(c, max(c, h), clamp(u[2] * k, 0.0, 1.0));
        c = mix(c, 0.5 * (c + h), clamp(u[2] * k * 0.35, 0.0, 1.0));
        float lift = u[3] * k;
        c = lift + c * (1.0 - lift);
        c = (c - 0.5) * (1.0 - u[4] * k * 0.5) + 0.5;
        o.write(float4(clamp(c, 0.0, 1.0), s.a), gid);
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
    // Fisheye / barrel distortion. u: amount, zoom, circle, chroma, center x, y
    kernel void k_fisheye(KIN, texture2d<float> a [[texture(1)]]) {
        GUARD
        float2 size = OSIZE;
        float2 uv = UV;
        float asp = size.x / size.y;
        float2 c = float2(u[4], u[5]);
        float2 d = (uv - c) * float2(asp, 1.0);
        float r2 = dot(d, d);
        float k = u[0];
        float3 col;
        for (int ch = 0; ch < 3; ch++) {
            float kk = k * (1.0 + (float(ch) - 1.0) * u[3]);
            float2 dd = d * (1.0 + kk * r2 + kk * 0.5 * r2 * r2) / max(0.2, u[1]);
            float2 suv = c + dd / float2(asp, 1.0);
            col[ch] = a.sample(sLin, suv)[ch];
            if (u[2] < 0.5 && (any(suv < 0.0) || any(suv > 1.0))) col[ch] = 0.0;
        }
        if (u[2] > 0.5) {
            float lim = 0.5 * min(asp, 1.0);
            col *= 1.0 - smoothstep(lim * 0.99, lim, sqrt(r2));
        }
        o.write(float4(col, a.read(gid).a), gid);
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

// MARK: - Diffusion (Pro-Mist and friends)

struct DiffusionEffect: Effect {
    let id = "diffusion"
    let name = "Diffusion Filter"
    let category = EffectCategory.lens
    var symbol: String { "camera.filters" }
    var summary: String { "Pro-Mist, Black Pro-Mist, Glimmerglass, Soft FX and fog filters: soft highlight bloom and veil." }
    let params: [ParamDef] = [
        .slider("strength", "Strength", 0, 200, 50, step: 1, unit: "%", section: "Filter"),
        .slider("glow", "Highlight Glow", 0, 200, 100, step: 1, unit: "%", section: "Filter"),
        .slider("haze", "Veil / Haze", 0, 100, 30, step: 1, unit: "%", section: "Filter"),
        .slider("threshold", "Glow Threshold", 0, 100, 55, step: 1, unit: "%", section: "Filter"),
        .slider("radius", "Radius", 2, 150, 30, step: 1, curve: 1.8, section: "Filter"),
        .slider("lift", "Black Lift", 0, 30, 5, step: 0.5, unit: "%", section: "Tone"),
        .slider("contrast", "Contrast Reduction", 0, 100, 25, step: 1, unit: "%", section: "Tone"),
        .slider("warmth", "Warmth", -100, 100, 10, step: 1, section: "Tone",
                track: [RGBA(hex: "#3A7BFF"), RGBA(hex: "#9A9A9A"), RGBA(hex: "#FFB020")]),
        .color("tint", "Glow Tint", .white, section: "Tone"),
    ]
    var looks: [EffectLook] {
        [
            EffectLook(id: "pm8", name: "Pro-Mist 1/8", group: "Pro-Mist", values: ["strength": 25.0]),
            EffectLook(id: "pm4", name: "Pro-Mist 1/4", group: "Pro-Mist", values: ["strength": 50.0]),
            EffectLook(id: "pm2", name: "Pro-Mist 1/2", group: "Pro-Mist", values: ["strength": 90.0]),
            EffectLook(id: "pm1", name: "Pro-Mist 1", group: "Pro-Mist", values: ["strength": 150.0, "radius": 45.0]),
            EffectLook(id: "bpm4", name: "Black Pro-Mist 1/4", group: "Black Pro-Mist", values: ["strength": 50.0, "haze": 8.0, "lift": 0.0,
                                                                                               "contrast": 8.0, "threshold": 62.0]),
            EffectLook(id: "bpm2", name: "Black Pro-Mist 1/2", group: "Black Pro-Mist", values: ["strength": 90.0, "haze": 10.0, "lift": 0.0,
                                                                                               "contrast": 10.0, "threshold": 60.0]),
            EffectLook(id: "glimmer", name: "Glimmerglass", group: "Other", values: ["strength": 80.0, "glow": 140.0, "haze": 4.0,
                                                                                    "radius": 10.0, "threshold": 68.0, "lift": 0.0, "contrast": 5.0]),
            EffectLook(id: "softfx", name: "Soft FX", group: "Other", values: ["strength": 70.0, "glow": 40.0, "haze": 70.0, "radius": 12.0,
                                                                              "lift": 2.0, "contrast": 10.0]),
            EffectLook(id: "cinebloom", name: "CineBloom", group: "Other", values: ["strength": 80.0, "glow": 160.0, "haze": 15.0,
                                                                                   "radius": 60.0, "threshold": 50.0]),
            EffectLook(id: "fog", name: "Fog Filter", group: "Other", values: ["strength": 100.0, "glow": 30.0, "haze": 85.0,
                                                                              "radius": 80.0, "lift": 14.0, "contrast": 45.0]),
        ]
    }
    func apply(_ input: MTLTexture, _ p: Params, _ ctx: RenderContext) -> MTLTexture {
        let r = max(1, ctx.px(p.num("radius", 30)))
        let f = max(1, Int(r / 6))
        let hl = brights(input, ctx, p, f)
        let small = ctx.resize(input, max(1, input.width / f), max(1, input.height / f))
        let sr = r / Double(f)
        let g1 = ctx.blur(hl, sigma: sr * 0.5)
        let g2 = ctx.blur(hl, sigma: sr * 2)
        let hz = ctx.blur(small, sigma: sr * 1.5)
        return ctx.apply("k_diffuse", [input, g1, g2, hz], [p.f("strength", 50) / 100, p.f("glow", 100) / 100, p.f("haze", 30) / 100,
                                                            p.f("lift", 5) / 100, p.f("contrast", 25) / 100, p.f("warmth", 10) / 100]
                         + p.color("tint", .white).floats)
    }
    private func brights(_ input: MTLTexture, _ ctx: RenderContext, _ p: Params, _ f: Int) -> MTLTexture {
        LensFilterKit.brights(input, ctx, threshold: p.f("threshold", 55) / 100, knee: 0.25, factor: f)
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
        .slider("glow", "Wide Glow", 0, 100, 25, step: 1, unit: "%", section: "Glow"),
        .slider("points", "Small Lights Only", 0, 100, 60, step: 1, unit: "%", section: "Glow"),
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
        b = LensFilterKit.pointsOnly(b, ctx, amount: p.f("points", 60) / 100)
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
        let gw = p.f("glow", 25) / 100
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
        .slider("points", "Small Lights Only", 0, 100, 85, step: 1, unit: "%", section: "Stars"),
        .toggle("spectral", "Rainbow", false, section: "Color"),
        .color("tint", "Tint", .white, section: "Color"),
    ]
    func apply(_ input: MTLTexture, _ p: Params, _ ctx: RenderContext) -> MTLTexture {
        let f = 4
        var b = LensFilterKit.brights(input, ctx, threshold: p.f("threshold", 86) / 100, knee: 0.08, factor: f)
        b = LensFilterKit.pointsOnly(b, ctx, amount: p.f("points", 85) / 100)
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
    let name = "Prism"
    let category = EffectCategory.lens
    var symbol: String { "triangle.lefthalf.filled" }
    var summary: String { "A prism held in front of the lens: refracted rainbow ghosts on one side or around the frame." }
    let params: [ParamDef] = [
        .choice("mode", "Placement", [("side", "One Side"), ("ring", "Around the Edges"), ("full", "Whole Frame")], "side", section: "Prism"),
        .slider("angle", "Direction", -180, 180, 0, step: 1, unit: "°", section: "Prism"),
        .slider("coverage", "Coverage", 5, 100, 40, step: 1, unit: "%", section: "Prism", visible: whenNot("mode", "full")),
        .slider("feather", "Feather", 0, 50, 15, step: 1, unit: "%", section: "Prism", visible: whenNot("mode", "full")),
        .slider("offset", "Offset", 0, 200, 40, step: 1, curve: 1.5, section: "Refraction"),
        .slider("copies", "Copies", 1, 4, 2, step: 1, section: "Refraction"),
        .slider("dispersion", "Rainbow Dispersion", 0, 100, 35, step: 1, unit: "%", section: "Refraction"),
        .slider("softness", "Softness", 0, 30, 3, step: 0.5, section: "Refraction"),
        .slider("intensity", "Intensity", 0, 150, 70, step: 1, unit: "%", section: "Refraction"),
        .choice("blend", "Blend", [("screen", "Screen"), ("lighten", "Lighten"), ("normal", "Normal")], "screen", section: "Refraction"),
    ]
    var looks: [EffectLook] {
        [
            EffectLook(id: "side", name: "Rainbow Side", values: [:]),
            EffectLook(id: "kaleido", name: "Edge Ghosts", values: ["mode": "ring", "coverage": 45.0, "copies": 3, "offset": 60.0]),
            EffectLook(id: "double", name: "Double Exposure", values: ["mode": "full", "offset": 120.0, "copies": 1, "dispersion": 15.0,
                                                                     "intensity": 50.0, "blend": "lighten"]),
            EffectLook(id: "dreamy", name: "Dreamy Refraction", values: ["softness": 14.0, "dispersion": 70.0, "offset": 25.0, "copies": 3]),
        ]
    }
    func apply(_ input: MTLTexture, _ p: Params, _ ctx: RenderContext) -> MTLTexture {
        let soft = ctx.blur(input, sigma: ctx.px(p.num("softness", 3)))
        let mode: Float = ["side": 0, "ring": 1, "full": 2][p.choice("mode", "side")] ?? 0
        let blend: Float = ["screen": 0, "lighten": 1, "normal": 2][p.choice("blend", "screen")] ?? 0
        return ctx.apply("k_prism", [input, soft], [mode, p.f("angle"), Float(ctx.px(p.num("offset", 40))), p.f("dispersion", 35) / 100 * 0.6,
                                                    p.f("intensity", 70) / 100, p.f("coverage", 40) / 100, p.f("feather", 15) / 100,
                                                    p.f("copies", 2), blend])
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
    let category = EffectCategory.lens
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

// MARK: - Fisheye

struct FisheyeEffect: Effect {
    let id = "fisheye"
    let name = "Fisheye & Distortion"
    let category = EffectCategory.lens
    var symbol: String { "circle.dotted" }
    var summary: String { "Barrel (fisheye) or pincushion distortion, with an optional circular frame — skate-video style." }
    let params: [ParamDef] = [
        .slider("amount", "Distortion", -100, 100, 45, step: 1),
        .slider("zoom", "Zoom", 50, 200, 100, step: 1, unit: "%"),
        .toggle("circle", "Circular Frame", false),
        .slider("fringe", "Color Fringing", 0, 100, 10, step: 1, unit: "%"),
        .slider("cx", "Center X", 0, 100, 50, step: 1, unit: "%"),
        .slider("cy", "Center Y", 0, 100, 50, step: 1, unit: "%"),
    ]
    var looks: [EffectLook] {
        [
            EffectLook(id: "vx", name: "VX1000 Fisheye", values: ["amount": 70.0, "fringe": 15.0]),
            EffectLook(id: "circle", name: "Full Circle", values: ["amount": 90.0, "circle": true]),
            EffectLook(id: "mild", name: "Mild Barrel", values: ["amount": 20.0]),
            EffectLook(id: "pincushion", name: "Pincushion", values: ["amount": -40.0, "zoom": 95.0]),
        ]
    }
    func apply(_ input: MTLTexture, _ p: Params, _ ctx: RenderContext) -> MTLTexture {
        let k = Double(p.f("amount", 45) / 100 * 0.9)
        let asp = Double(input.width) / Double(input.height)
        func f(_ r: Double) -> Double { 1 + k * r * r + k * 0.5 * r * r * r * r }
        var auto = 1.0
        if p.bool("circle") {
            // the circle's edge lands exactly on the image's short side
            let lim = 0.5 * min(asp, 1)
            auto = lim * f(lim) / (0.5 * min(asp, 1))
        } else if k > 0 {
            // barrel distortion pulls the corners inwards: zoom so the frame stays filled
            let rc = (asp * asp / 4 + 0.25).squareRoot()
            auto = f(rc)
        }
        let zoom = Float(p.num("zoom", 100) / 100 * auto)
        return ctx.apply("k_fisheye", [input], [Float(k), zoom,
                                                p.bool("circle") ? 1 : 0, p.f("fringe", 10) / 100 * 0.08,
                                                p.f("cx", 50) / 100, p.f("cy", 50) / 100])
    }
}

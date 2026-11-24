import Foundation
import Metal

enum LensShaders {
    static let source = """
    // Chromatic aberration. u: mode (0 radial, 1 linear), amount(px), angle, falloff, center x, y
    kernel void k_chromab(KIN, texture2d<float> a [[texture(1)]]) {
        GUARD
        float2 size = OSIZE;
        float2 uv = UV;
        float2 off;
        if (u[0] < 0.5) {
            float2 d = (uv - float2(u[4], u[5])) * float2(size.x / max(size.x, size.y), size.y / max(size.x, size.y));
            float r = length(d) * 1.414;
            off = normalize(d + 1e-6) * pow(r, max(0.2, u[3])) * u[1];
        } else {
            float an = u[2] * 0.0174533;
            off = float2(cos(an), -sin(an)) * u[1];
        }
        off /= size;
        float4 c = a.read(gid);
        float r = a.sample(sLin, uv + off).r;
        float b = a.sample(sLin, uv - off).b;
        o.write(float4(r, c.g, b, c.a), gid);
    }

    // Vignette. u: amount (-1..1), midpoint, roundness, feather, color rgb (4..6), highlight protect
    kernel void k_vignette(KIN, texture2d<float> a [[texture(1)]]) {
        GUARD
        float2 size = OSIZE;
        float2 uv = UV - 0.5;
        float asp = size.x / size.y;
        float2 q = uv * mix(float2(2.0), float2(asp, 1.0) / max(asp, 1.0) * 2.0, u[2]);
        float d = length(q);
        float mid = mix(0.3, 1.3, u[1]);
        float f = smoothstep(mid - u[3], mid + u[3] * 0.6, d);
        float4 s = a.read(gid);
        float3 c = s.rgb;
        float amt = u[0];
        if (amt >= 0.0) {
            float protect = 1.0 - u[7] * smoothstep(0.7, 1.0, luma(c));
            c = mix(c, c * float3(u[4], u[5], u[6]), f * amt * protect);
        } else {
            c = mix(c, 1.0 - (1.0 - c) * 0.25, f * -amt);
        }
        o.write(float4(clamp(c, 0.0, 1.0), s.a), gid);
    }

    // Unsharp mask. u: amount, threshold, luminance only
    kernel void k_unsharp(KIN, texture2d<float> a [[texture(1)]], texture2d<float> b [[texture(2)]]) {
        GUARD
        float4 s = a.read(gid);
        float3 bl = b.read(gid).rgb;
        float3 d = s.rgb - bl;
        if (u[2] > 0.5) d = float3(luma(d));
        float m = smoothstep(u[1], u[1] * 2.0 + 1e-4, abs(luma(s.rgb - bl)));
        o.write(float4(clamp(s.rgb + d * u[0] * m, 0.0, 1.0), s.a), gid);
    }

    // CRT. u: curvature, mask type (0 none, 1 aperture, 2 shadow, 3 slot), mask size(px), mask strength,
    //      scanlines, scan size(px), glow, brightness, vignette, corner, flicker/noise
    kernel void k_crt(KIN, texture2d<float> a [[texture(1)]], texture2d<float> glow [[texture(2)]]) {
        GUARD
        float2 size = OSIZE;
        float2 uv = UV;
        float2 cc = uv * 2.0 - 1.0;
        float k = u[0];
        float2 dc = cc * (1.0 + k * float2(cc.y * cc.y, cc.x * cc.x) * 0.25);
        float2 suv = dc * 0.5 + 0.5;
        float inside = step(0.0, suv.x) * step(suv.x, 1.0) * step(0.0, suv.y) * step(suv.y, 1.0);
        if (u[11] < 0.5) { inside = 1.0; }
        // rounded screen corners
        float2 e = abs(dc);
        float cr = u[9];
        float2 qq = max(e - (1.0 - cr), 0.0);
        float corner = u[11] < 0.5 ? 1.0 : 1.0 - smoothstep(cr - 0.01, cr + 0.005, length(qq));
        float3 c = a.sample(sLin, suv).rgb;
        c += glow.sample(sLin, suv).rgb * u[6];
        float2 px = float2(gid);
        float ms = max(1.0, u[2]);
        int mt = int(u[1]);
        float3 mask = float3(1.0);
        if (mt == 1) {
            int i = int(floor(px.x / ms)) % 3;
            mask = i == 0 ? float3(1, 0.25, 0.25) : (i == 1 ? float3(0.25, 1, 0.25) : float3(0.25, 0.25, 1));
        } else if (mt == 2) {
            float row = floor(px.y / ms);
            int i = int(floor(px.x / ms + fmod(row, 2.0) * 1.5)) % 3;
            mask = i == 0 ? float3(1, 0.3, 0.3) : (i == 1 ? float3(0.3, 1, 0.3) : float3(0.3, 0.3, 1));
            mask *= 0.75 + 0.25 * step(0.25, fract(px.y / ms));
        } else if (mt == 3) {
            float col = floor(px.x / ms);
            int i = int(col) % 3;
            mask = i == 0 ? float3(1, 0.3, 0.3) : (i == 1 ? float3(0.3, 1, 0.3) : float3(0.3, 0.3, 1));
            float yy = fract((px.y + floor(col / 3.0) * ms * 1.5) / (ms * 3.0));
            mask *= 0.7 + 0.3 * step(0.12, yy);
        }
        c *= mix(float3(1.0), mask * 1.6, u[3]);
        if (u[4] > 0.0) {
            float sp = max(2.0, u[5]);
            float ph = fract(suv.y * size.y / sp);
            float l = luma(c);
            float w = mix(0.5, 0.15, clamp(l, 0.0, 1.0));       // bright lines bloom wider
            float sl = smoothstep(0.0, w, ph) * smoothstep(1.0, 1.0 - w, ph);
            c *= mix(1.0, 0.35 + 0.9 * sl, u[4]);
        }
        c *= u[7];
        c *= 1.0 - u[8] * smoothstep(0.4, 1.4, length(cc));
        c += (hash12(px + u[10] * 13.0) - 0.5) * u[10] * 0.06;
        c = mix(float3(0.02), c, inside * corner);
        o.write(float4(clamp(c, 0.0, 1.0), a.read(gid).a), gid);
    }
    """
}

struct ChromaticAberrationEffect: Effect {
    let id = "chromab"
    let name = "Chromatic Aberration"
    let category = EffectCategory.lens
    var symbol: String { "camera.aperture" }
    var summary: String { "Red/blue colour fringing from a cheap lens, radial or linear." }
    let params: [ParamDef] = [
        .segmented("mode", "Mode", [("radial", "Radial"), ("linear", "Linear")], "radial"),
        .slider("amount", "Amount", 0, 60, 6, step: 0.1, curve: 1.8),
        .slider("angle", "Angle", -180, 180, 0, step: 1, unit: "°", visible: when("mode", "linear")),
        .slider("falloff", "Falloff", 0.2, 4, 1.6, step: 0.05, visible: whenNot("mode", "linear")),
        .slider("cx", "Center X", 0, 100, 50, step: 1, unit: "%", visible: whenNot("mode", "linear")),
        .slider("cy", "Center Y", 0, 100, 50, step: 1, unit: "%", visible: whenNot("mode", "linear")),
    ]
    func apply(_ input: MTLTexture, _ p: Params, _ ctx: RenderContext) -> MTLTexture {
        ctx.apply("k_chromab", [input], [p.choice("mode", "radial") == "linear" ? 1 : 0, Float(ctx.px(p.num("amount", 6))),
                                         p.f("angle"), p.f("falloff", 1.6), p.f("cx", 50) / 100, p.f("cy", 50) / 100])
    }
}

struct VignetteEffect: Effect {
    let id = "vignette"
    let name = "Vignette"
    let category = EffectCategory.lens
    var symbol: String { "circle.dashed" }
    var summary: String { "Darken (or lighten) the edges of the frame." }
    let params: [ParamDef] = [
        .slider("amount", "Amount", -100, 100, 45, step: 1, unit: "%"),
        .slider("midpoint", "Midpoint", 0, 100, 50, step: 1, unit: "%"),
        .slider("roundness", "Roundness", 0, 100, 50, step: 1, unit: "%"),
        .slider("feather", "Feather", 5, 100, 55, step: 1, unit: "%"),
        .slider("protect", "Protect Highlights", 0, 100, 0, step: 1, unit: "%"),
        .color("color", "Color", RGBA(hex: "#000000")),
        .sectionSwitch("blurOn", section: "Edge Blur", false),
        .slider("blur", "Radius", 1, 80, 12, step: 0.5, curve: 1.6, section: "Edge Blur"),
        .slider("blurAmount", "Amount", 0, 100, 80, step: 1, unit: "%", section: "Edge Blur"),
    ]
    func apply(_ input: MTLTexture, _ p: Params, _ ctx: RenderContext) -> MTLTexture {
        var img = input
        if p.bool("blurOn", false) {
            let b = ctx.blur(input, sigma: ctx.px(p.num("blur", 12)))
            img = ctx.apply("k_maskmix", [input, b], [1, p.f("midpoint", 50) / 100 * 0.8, p.f("feather", 55) / 100,
                                                      0, 0.5, 0.5, p.f("blurAmount", 80) / 100])
        }
        return ctx.apply("k_vignette", [img], [p.f("amount", 45) / 100, p.f("midpoint", 50) / 100, p.f("roundness", 50) / 100,
                                               p.f("feather", 55) / 100] + p.color("color", .black).floats + [p.f("protect") / 100])
    }
}

struct SharpenEffect: Effect {
    let id = "sharpen"
    let name = "Sharpen"
    let category = EffectCategory.adjust
    var symbol: String { "triangle" }
    var summary: String { "Unsharp mask. Push it for the crunchy over-sharpened digicam look." }
    let params: [ParamDef] = [
        .slider("amount", "Amount", 0, 500, 80, step: 1, unit: "%", curve: 1.5),
        .slider("radius", "Radius", 0.3, 20, 1.5, step: 0.05, curve: 2),
        .slider("threshold", "Threshold", 0, 50, 2, step: 0.5, unit: "%"),
        .toggle("luma", "Luminance Only", true),
    ]
    func apply(_ input: MTLTexture, _ p: Params, _ ctx: RenderContext) -> MTLTexture {
        let b = ctx.blur(input, sigma: max(0.35, ctx.px(p.num("radius", 1.5))))
        return ctx.apply("k_unsharp", [input, b], [p.f("amount", 80) / 100, p.f("threshold", 2) / 100, p.bool("luma", true) ? 1 : 0])
    }
}

struct CRTEffect: Effect {
    let id = "crt"
    let name = "CRT Screen"
    let category = EffectCategory.glitch
    var symbol: String { "tv" }
    var summary: String { "Old tube monitor: phosphor mask, scanlines, glow and curved glass." }
    let params: [ParamDef] = [
        .slider("curvature", "Curvature", 0, 100, 30, step: 1, unit: "%", section: "Screen"),
        .toggle("bezel", "Black Border", true, section: "Screen"),
        .slider("corner", "Corner Radius", 0, 30, 8, step: 0.5, unit: "%", section: "Screen", visible: whenOn("bezel")),
        .slider("brightness", "Brightness", 50, 200, 125, step: 1, unit: "%", section: "Screen"),
        .sectionSwitch("maskOn", section: "Phosphors"),
        .choice("mask", "Phosphor Mask", [("aperture", "Aperture Grille"), ("shadow", "Shadow Mask"), ("slot", "Slot Mask")],
                "aperture", section: "Phosphors"),
        .slider("maskSize", "Mask Size", 1, 8, 1.5, step: 0.25, section: "Phosphors"),
        .slider("maskAmt", "Mask Strength", 0, 100, 55, step: 1, unit: "%", section: "Phosphors"),
        .sectionSwitch("scanOn", section: "Scanlines"),
        .slider("scan", "Intensity", 0, 100, 55, step: 1, unit: "%", section: "Scanlines"),
        .slider("scanSize", "Line Spacing", 2, 16, 4, step: 0.5, section: "Scanlines"),
        .sectionSwitch("glowOn", section: "Glow"),
        .slider("glow", "Glow", 0, 100, 35, step: 1, unit: "%", section: "Glow"),
        .slider("glowRadius", "Glow Radius", 1, 40, 8, step: 0.5, curve: 1.6, section: "Glow"),
        .sectionSwitch("vigOn", section: "Vignette"),
        .slider("vignette", "Amount", 0, 100, 35, step: 1, unit: "%", section: "Vignette"),
        .sectionSwitch("noiseOn", section: "Noise"),
        .slider("noise", "Amount", 0, 100, 10, step: 1, unit: "%", section: "Noise"),
    ]
    var looks: [EffectLook] {
        [
            EffectLook(id: "pc", name: "PC Monitor '99", values: ["mask": "shadow", "maskSize": 1.25, "maskAmt": 45.0, "scan": 25.0, "curvature": 15.0]),
            EffectLook(id: "tv", name: "Living Room TV", values: ["mask": "slot", "maskSize": 2.0, "scan": 50.0, "curvature": 45.0, "glow": 50.0]),
            EffectLook(id: "arcade", name: "Arcade Cabinet", values: ["mask": "aperture", "maskSize": 2.0, "maskAmt": 70.0, "scan": 75.0,
                                                                    "scanSize": 5.0, "curvature": 25.0, "glow": 60.0, "brightness": 140.0]),
            EffectLook(id: "pvm", name: "Studio PVM", values: ["mask": "aperture", "maskSize": 1.25, "maskAmt": 40.0, "scan": 80.0,
                                                             "scanSize": 3.0, "curvature": 5.0, "corner": 2.0, "vignette": 15.0]),
        ]
    }
    func apply(_ input: MTLTexture, _ p: Params, _ ctx: RenderContext) -> MTLTexture {
        func on(_ k: String) -> Float { p.bool(k, true) ? 1 : 0 }
        let maskOn = p.bool("maskOn", true) && p.choice("mask", "aperture") != "none"
        let mask: Float = maskOn ? (["aperture": 1, "shadow": 2, "slot": 3][p.choice("mask", "aperture")] ?? 1) : 0
        let glowOn = p.bool("glowOn", true)
        let glow = glowOn ? ctx.blur(input, sigma: ctx.px(p.num("glowRadius", 8))) : input
        return ctx.apply("k_crt", [input, glow], [
            p.f("curvature", 30) / 100, mask, Float(max(1, ctx.px(p.num("maskSize", 1.5)).rounded())), p.f("maskAmt", 55) / 100,
            p.f("scan", 55) / 100 * on("scanOn"), Float(max(2, ctx.px(p.num("scanSize", 4)).rounded())), glowOn ? p.f("glow", 35) / 100 * 0.8 : 0,
            p.f("brightness", 125) / 100, p.f("vignette", 35) / 100 * on("vigOn"), p.f("corner", 8) / 100,
            p.f("noise", 10) / 100 * on("noiseOn"), on("bezel"),
        ])
    }
}

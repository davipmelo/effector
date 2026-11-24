import Foundation
import Metal

enum BlurShaders {
    static let source = """
    // 0 at the protected centre, 1 towards the borders. shape: 0 round, 1 frame
    inline float edgeMask(float2 uv, float asp, float2 c, float protect, float soft, float shape) {
        float2 q = (uv - c) * float2(asp, 1.0);
        float d = shape < 0.5 ? length(q) / length(float2(asp, 1.0) * 0.5) : max(abs(q.x) / (asp * 0.5), abs(q.y) / 0.5);
        return smoothstep(protect, min(1.2, protect + max(0.02, soft)), d);
    }

    // Mix a blurred version in, everywhere / on the edges / in the centre.
    // u: where (0 all,1 edges,2 centre), protect, soft, shape, cx, cy, amount
    kernel void k_maskmix(KIN, texture2d<float> base [[texture(1)]], texture2d<float> bl [[texture(2)]]) {
        GUARD
        float2 size = OSIZE;
        float2 uv = UV;
        float m = 1.0;
        int w = int(u[0]);
        if (w > 0) {
            m = edgeMask(uv, size.x / size.y, float2(u[4], u[5]), u[1], u[2], u[3]);
            if (w == 2) m = 1.0 - m;
        }
        float4 b = base.read(gid);
        o.write(float4(mix(b.rgb, bl.sample(sLin, uv).rgb, clamp(m * u[6], 0.0, 1.0)), b.a), gid);
    }

    // Radial blur (zoom / spin), one pass. u: mode(0 zoom,1 spin,2 both), amount, cx, cy, protect, soft, shape, passScale
    kernel void k_radialblur(KIN, texture2d<float> a [[texture(1)]]) {
        GUARD
        float2 size = OSIZE;
        float asp = size.x / size.y;
        float2 uv = UV;
        float2 c = float2(u[2], u[3]);
        float m = edgeMask(uv, asp, c, u[4], u[5], u[6]);
        float2 q = (uv - c) * float2(asp, 1.0);
        float amt = u[1] * m * u[7];
        const int N = 28;
        float3 acc = float3(0.0);
        float jit = hash12(float2(gid) + u[7] * 7.0);
        int mode = int(u[0]);
        for (int i = 0; i < N; i++) {
            float t = (float(i) + jit) / float(N) - 0.5;      // centred on the pixel
            float2 sq = q;
            if (mode == 0 || mode == 2) sq *= 1.0 + t * amt * 0.5;
            if (mode == 1 || mode == 2) {
                float an = t * amt * 0.6;
                float cs = cos(an), sn = sin(an);
                sq = float2(cs * sq.x - sn * sq.y, sn * sq.x + cs * sq.y);
            }
            acc += a.sample(sLin, c + sq / float2(asp, 1.0)).rgb;
        }
        o.write(float4(acc / float(N), a.read(gid).a), gid);
    }

    // Directional (motion) blur, one pass. u: dx, dy (px), centred, shutter (0 box,1 fade)
    kernel void k_dirblur(KIN, texture2d<float> a [[texture(1)]]) {
        GUARD
        float2 size = OSIZE;
        float2 uv = UV;
        const int N = 32;
        float3 acc = float3(0.0); float ws = 0.0;
        float jit = hash12(float2(gid) + u[0]);
        for (int i = 0; i < N; i++) {
            float t = (float(i) + jit) / float(N);
            if (u[2] > 0.5) t -= 0.5;
            float w = u[3] > 0.5 ? (1.0 - abs(t) * (u[2] > 0.5 ? 2.0 : 1.0)) : 1.0;
            acc += a.sample(sLin, uv + float2(u[0], u[1]) * t / size).rgb * w;
            ws += w;
        }
        o.write(float4(acc / max(ws, 1e-4), a.read(gid).a), gid);
    }

    // Edge-preserving (surface) blur. u: radius(px), threshold
    kernel void k_surface(KIN, texture2d<float> a [[texture(1)]]) {
        GUARD
        float2 size = OSIZE;
        float2 uv = UV;
        float3 c0 = a.read(gid).rgb;
        float3 acc = float3(0.0); float ws = 0.0;
        float th = max(0.01, u[1]);
        for (int y = -4; y <= 4; y++) {
            for (int x = -4; x <= 4; x++) {
                float2 o2 = float2(x, y) / 4.0 * u[0];
                float3 s = a.sample(sLin, uv + o2 / size).rgb;
                float ds = length(s - c0);
                float w = exp(-dot(float2(x, y), float2(x, y)) / 18.0) * exp(-ds * ds / (th * th));
                acc += s * w; ws += w;
            }
        }
        o.write(float4(acc / ws, a.read(gid).a), gid);
    }
    """
}

/// Shared "where" controls for blurs: everywhere, only the edges or only the centre.
enum BlurArea {
    static func params(_ section: String) -> [ParamDef] {
        [
            .segmented("area", "Where", [("all", "Everywhere"), ("edges", "Edges"), ("center", "Center")], "all", section: section),
            .segmented("shape", "Shape", [("round", "Round"), ("frame", "Frame")], "round", section: section, visible: whenNot("area", "all")),
            .slider("protect", "Center Size", 0, 90, 35, step: 1, unit: "%", section: section, visible: whenNot("area", "all")),
            .slider("softness", "Transition", 2, 100, 45, step: 1, unit: "%", section: section, visible: whenNot("area", "all")),
            .slider("cx", "Center X", 0, 100, 50, step: 1, unit: "%", section: section, visible: whenNot("area", "all")),
            .slider("cy", "Center Y", 0, 100, 50, step: 1, unit: "%", section: section, visible: whenNot("area", "all")),
        ]
    }
    static func mix(_ input: MTLTexture, _ blurred: MTLTexture, _ p: Params, _ ctx: RenderContext, amount: Float = 1) -> MTLTexture {
        let area: Float = ["all": 0, "edges": 1, "center": 2][p.choice("area", "all")] ?? 0
        if area == 0 && amount >= 0.999 { return blurred }
        return ctx.apply("k_maskmix", [input, blurred], [area, p.f("protect", 35) / 100, p.f("softness", 45) / 100,
                                                         p.choice("shape", "round") == "frame" ? 1 : 0,
                                                         p.f("cx", 50) / 100, p.f("cy", 50) / 100, amount])
    }
}

struct BlurEffect: Effect {
    let id = "blur"
    let name = "Blur"
    let category = EffectCategory.blur
    var symbol: String { "aqi.medium" }
    var summary: String { "Gaussian or edge-preserving (surface) blur, everywhere, only on the edges or only in the centre." }
    var params: [ParamDef] {
        [
            .segmented("type", "Type", [("gaussian", "Gaussian"), ("surface", "Surface (Smart)")], "gaussian", section: "Blur"),
            .slider("radius", "Radius", 0, 200, 8, step: 0.5, curve: 2.2, section: "Blur"),
            .slider("threshold", "Edge Threshold", 1, 100, 20, step: 1, unit: "%", section: "Blur", visible: when("type", "surface")),
            .slider("amount", "Amount", 0, 100, 100, step: 1, unit: "%", section: "Blur"),
        ] + BlurArea.params("Area")
    }
    var looks: [EffectLook] {
        [
            EffectLook(id: "soft", name: "Soft Focus", values: ["radius": 6.0, "amount": 50.0]),
            EffectLook(id: "edges", name: "Blurry Edges", values: ["radius": 20.0, "area": "edges", "protect": 30.0]),
            EffectLook(id: "skin", name: "Smooth Skin", values: ["type": "surface", "radius": 10.0, "threshold": 12.0]),
            EffectLook(id: "dream", name: "Dreamy Center", values: ["radius": 14.0, "area": "center", "amount": 60.0]),
        ]
    }
    func apply(_ input: MTLTexture, _ p: Params, _ ctx: RenderContext) -> MTLTexture {
        let r = ctx.px(p.num("radius", 8))
        let blurred = p.choice("type", "gaussian") == "surface"
            ? ctx.apply("k_surface", [input], [Float(max(1, r)), p.f("threshold", 20) / 100])
            : ctx.blur(input, sigma: r / 2)
        return BlurArea.mix(input, blurred, p, ctx, amount: p.f("amount", 100) / 100)
    }
}

struct RadialBlurEffect: Effect {
    let id = "radialblur"
    let name = "Radial Blur"
    let category = EffectCategory.blur
    var symbol: String { "smallcircle.filled.circle" }
    var summary: String { "Zoom or spin blur around a point, with a sharp protected centre — like Resolve's radial blur." }
    let params: [ParamDef] = [
        .segmented("mode", "Mode", [("zoom", "Zoom"), ("spin", "Spin"), ("both", "Both")], "zoom", section: "Blur"),
        .slider("amount", "Amount", 0, 200, 40, step: 1, unit: "%", curve: 1.3, section: "Blur"),
        .slider("cx", "Center X", 0, 100, 50, step: 1, unit: "%", section: "Blur"),
        .slider("cy", "Center Y", 0, 100, 50, step: 1, unit: "%", section: "Blur"),
        .segmented("shape", "Shape", [("round", "Round"), ("frame", "Frame")], "round", section: "Sharp Center"),
        .slider("protect", "Center Size", 0, 90, 15, step: 1, unit: "%", section: "Sharp Center"),
        .slider("softness", "Transition", 2, 100, 60, step: 1, unit: "%", section: "Sharp Center"),
        .segmented("quality", "Quality", [("fast", "Fast"), ("high", "High")], "high", section: "Sharp Center"),
    ]
    var looks: [EffectLook] {
        [
            EffectLook(id: "zoom", name: "Zoom Burst", values: ["mode": "zoom", "amount": 60.0]),
            EffectLook(id: "spin", name: "Spin", values: ["mode": "spin", "amount": 50.0]),
            EffectLook(id: "vortex", name: "Vortex", values: ["mode": "both", "amount": 80.0, "protect": 10.0]),
            EffectLook(id: "subtle", name: "Subtle Focus Pull", values: ["mode": "zoom", "amount": 20.0, "protect": 35.0]),
        ]
    }
    func apply(_ input: MTLTexture, _ p: Params, _ ctx: RenderContext) -> MTLTexture {
        let mode: Float = ["zoom": 0, "spin": 1, "both": 2][p.choice("mode", "zoom")] ?? 0
        let base: [Float] = [mode, p.f("amount", 40) / 100, p.f("cx", 50) / 100, p.f("cy", 50) / 100, p.f("protect", 15) / 100,
                             p.f("softness", 60) / 100, p.choice("shape", "round") == "frame" ? 1 : 0]
        var out = ctx.apply("k_radialblur", [input], base + [1])
        if p.choice("quality", "high") == "high" {
            // second, finer pass fills the gaps between samples
            out = ctx.apply("k_radialblur", [out], base + [0.12])
        }
        return out
    }
}

struct MotionBlurEffect: Effect {
    let id = "motionblur"
    let name = "Motion Blur"
    let category = EffectCategory.blur
    var symbol: String { "arrow.right.and.line.vertical.and.arrow.left" }
    var summary: String { "Straight-line motion blur at any angle, centred or trailing, optionally only on the edges." }
    var params: [ParamDef] {
        [
            .slider("angle", "Angle", -180, 180, 0, step: 1, unit: "°", section: "Blur"),
            .slider("distance", "Distance", 0, 600, 60, step: 1, curve: 1.8, section: "Blur"),
            .toggle("centered", "Centered", true, section: "Blur"),
            .segmented("shutter", "Shutter", [("box", "Even"), ("fade", "Fade")], "fade", section: "Blur"),
            .slider("amount", "Amount", 0, 100, 100, step: 1, unit: "%", section: "Blur"),
        ] + BlurArea.params("Area")
    }
    func apply(_ input: MTLTexture, _ p: Params, _ ctx: RenderContext) -> MTLTexture {
        let a = p.num("angle") * .pi / 180
        let d = ctx.px(p.num("distance", 60))
        let dx = Float(cos(a) * d), dy = Float(-sin(a) * d)
        let shutter: Float = p.choice("shutter", "fade") == "fade" ? 1 : 0
        let centered: Float = p.bool("centered", true) ? 1 : 0
        var b = ctx.apply("k_dirblur", [input], [dx, dy, centered, shutter])
        if d > 30 { b = ctx.apply("k_dirblur", [b], [dx / 12, dy / 12, 1, 0]) }
        return BlurArea.mix(input, b, p, ctx, amount: p.f("amount", 100) / 100)
    }
}

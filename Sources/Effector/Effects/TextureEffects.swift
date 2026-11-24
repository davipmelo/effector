import Foundation
import Metal
import CoreGraphics

enum TextureShaders {
    static let source = """
    inline float fbm(float2 p, float seed, int oct) {
        float s = 0.0, a = 0.5;
        for (int i = 0; i < oct; i++) {
            s += a * vnoise(p, seed + float(i) * 7.1);
            p = p * 2.03 + 17.1;
            a *= 0.5;
        }
        return s / (1.0 - pow(0.5, float(oct)));
    }
    inline float ridge(float2 p, float seed) { return 1.0 - abs(vnoise(p, seed) * 2.0 - 1.0); }
    // Voronoi: (F1, F2, cell id hash, cell id hash 2)
    inline float4 voro(float2 x, float seed) {
        float2 n = floor(x), f = fract(x);
        float f1 = 8.0, f2 = 8.0, id = 0.0, id2 = 0.0;
        for (int j = -1; j <= 1; j++) {
            for (int i = -1; i <= 1; i++) {
                float2 g = float2(i, j);
                float2 o = float2(hash13(float3(n + g, seed)), hash13(float3(n + g, seed + 1.7)));
                float2 r = g + o - f;
                float d = dot(r, r);
                if (d < f1) { f2 = f1; f1 = d; id = hash13(float3(n + g, seed + 3.3)); id2 = hash13(float3(n + g, seed + 5.9)); }
                else if (d < f2) { f2 = d; }
            }
        }
        return float4(sqrt(f1), sqrt(f2), id, id2);
    }
    // Composite a premultiplied overlay with opacity. u[0] = opacity
    kernel void k_over_pm_op(KIN, texture2d<float> base [[texture(1)]], texture2d<float> ov [[texture(2)]]) {
        GUARD
        float4 b = base.read(gid);
        float4 t = ov.sample(sLin, UV) * u[0];
        o.write(float4(b.rgb * (1.0 - t.a) + t.rgb, b.a), gid);
    }

    // Light leak. u: 0 seed, 1 unit, 2 edge, 3 size, 4 intensity, 5..7 colA, 8..10 colB, 11 warp, 12 blend
    kernel void k_leak(KIN, texture2d<float> a [[texture(1)]]) {
        GUARD
        float2 size = OSIZE;
        float2 uv = UV;
        float asp = size.x / size.y;
        float seed = u[0];
        int edge = int(u[2]);
        float d;
        if (edge == 0) d = uv.x * asp;
        else if (edge == 1) d = (1.0 - uv.x) * asp;
        else if (edge == 2) d = uv.y;
        else if (edge == 3) d = 1.0 - uv.y;
        else if (edge == 4) d = length(uv * float2(asp, 1.0)) * 0.75;
        else if (edge == 5) d = length((1.0 - uv) * float2(asp, 1.0)) * 0.75;
        else {
            float2 c = float2(hash12(float2(seed, 4.0)), hash12(float2(seed, 5.0)));
            d = length((uv - c) * float2(asp, 1.0)) * 0.9;
        }
        float2 nq = uv * float2(asp, 1.0) * 2.2;
        d += (fbm(nq, seed, 4) - 0.5) * u[11];
        float w = exp(-max(d, 0.0) / max(0.02, u[3]) * 2.5);
        float t = smoothstep(0.25, 0.75, fbm(nq * 0.8 + 7.0, seed + 3.0, 3));
        float3 col = mix(float3(u[5], u[6], u[7]), float3(u[8], u[9], u[10]), t) * w;
        col += float3(1.0, 0.96, 0.88) * pow(w, 3.0) * 0.45;
        col *= u[4];
        float4 s = a.read(gid);
        float3 b = clamp(s.rgb, 0.0, 1.0);
        float3 r;
        int m = int(u[12]);
        if (m == 1) r = b + col;
        else if (m == 2) r = max(b, col);
        else r = 1.0 - (1.0 - b) * (1.0 - clamp(col, 0.0, 1.0));
        o.write(float4(clamp(r, 0.0, 1.0), s.a), gid);
    }
    """
}

// MARK: - Dust & scratches

struct DustEffect: Effect {
    let id = "dust"
    let name = "Dust & Scratches"
    let category = EffectCategory.overlay
    var symbol: String { "aqi.low" }
    var summary: String { "Specks, hairs and film scratches, like a dusty scan or an old print." }
    let params: [ParamDef] = [
        .segmented("style", "Style", [("dark", "Dark"), ("light", "Light"), ("mixed", "Mixed")], "light", section: "Look"),
        .sectionSwitch("dustOn", section: "Dust"),
        .slider("amount", "Density", 0, 100, 35, step: 1, unit: "%", curve: 1.5, section: "Dust"),
        .slider("size", "Size", 0.2, 5, 1, step: 0.05, curve: 1.8, section: "Dust"),
        .sectionSwitch("hairsOn", section: "Hairs"),
        .slider("hairs", "Hairs", 0, 40, 5, step: 1, section: "Hairs"),
        .sectionSwitch("scratchesOn", section: "Scratches"),
        .slider("scratches", "Scratches", 0, 40, 3, step: 1, section: "Scratches"),
        .slider("scratchLen", "Scratch Length", 5, 100, 55, step: 1, unit: "%", section: "Scratches"),
        .slider("opacity", "Opacity", 0, 100, 80, step: 1, unit: "%", section: "Look"),
        .slider("softness", "Softness", 0, 3, 0.5, step: 0.05, section: "Look"),
        .seed(section: "Look"),
    ]
    var looks: [EffectLook] {
        [
            EffectLook(id: "scan", name: "Dusty Negative Scan", values: ["style": "light", "amount": 45.0, "hairs": 8, "scratches": 2]),
            EffectLook(id: "print", name: "Old Print", values: ["style": "dark", "amount": 30.0, "hairs": 4, "scratches": 5, "opacity": 65.0]),
            EffectLook(id: "cinema", name: "Projected Film Reel", values: ["style": "mixed", "amount": 25.0, "hairs": 3, "scratches": 14,
                                                                     "scratchLen": 100.0, "size": 1.4]),
            EffectLook(id: "subtle", name: "Subtle Specks", values: ["style": "light", "amount": 15.0, "hairs": 1, "scratches": 0, "opacity": 60.0]),
        ]
    }

    func apply(_ input: MTLTexture, _ p: Params, _ ctx: RenderContext) -> MTLTexture {
        let W = CGFloat(input.width), H = CGFloat(input.height)
        let s = CGFloat(ctx.scale)
        var rng = SplitMix(seed: UInt64(max(1, p.num("seed", 1))) &* 2654435761)
        func r() -> CGFloat { CGFloat(rng.unit()) }
        let style = p.choice("style", "light")
        let size = CGFloat(p.num("size", 1))
        let ov = OverlayCanvas(input.width, input.height)
        let g = ov.ctx
        func color() -> CGColor {
            let dark = style == "dark" || (style == "mixed" && r() < 0.5)
            let a = 0.55 + r() * 0.45
            return dark ? CGColor(srgbRed: 0.06, green: 0.06, blue: 0.06, alpha: a) : CGColor(srgbRed: 0.96, green: 0.96, blue: 0.96, alpha: a)
        }
        // specks: many tiny, a few larger clumps
        let refArea = Double(W / s) * Double(H / s)
        let n = p.bool("dustOn", true) ? Int(p.num("amount", 35) / 100 * refArea * 0.0012) : 0
        for _ in 0..<min(n, 40000) {
            let x = r() * W, y = r() * H
            let rad = (0.35 + pow(r(), 5) * 5.5) * size * s
            g.setFillColor(color())
            let parts = rad > 2 * s ? 3 : 1
            for _ in 0..<parts {
                let ox = (r() - 0.5) * rad, oy = (r() - 0.5) * rad
                let rw = rad * (0.5 + r() * 0.8), rh = rad * (0.5 + r() * 0.8)
                g.fillEllipse(in: CGRect(x: x + ox - rw / 2, y: y + oy - rh / 2, width: rw, height: rh))
            }
        }
        // hairs / fibres
        g.setLineCap(.round)
        for _ in 0..<(p.bool("hairsOn", true) ? Int(p.num("hairs", 5)) : 0) {
            g.setStrokeColor(color())
            g.setLineWidth((0.5 + r() * 0.8) * s * size)
            var pt = CGPoint(x: r() * W, y: r() * H)
            g.move(to: pt)
            var ang = r() * .pi * 2
            let segs = 3 + Int(r() * 4)
            let len = (15 + r() * 70) * s * size
            for _ in 0..<segs {
                ang += (r() - 0.5) * 1.6
                let to = CGPoint(x: pt.x + cos(ang) * len / CGFloat(segs), y: pt.y + sin(ang) * len / CGFloat(segs))
                let c = CGPoint(x: (pt.x + to.x) / 2 + (r() - 0.5) * len * 0.3, y: (pt.y + to.y) / 2 + (r() - 0.5) * len * 0.3)
                g.addQuadCurve(to: to, control: c)
                pt = to
            }
            g.strokePath()
        }
        // scratches run along the film (vertical)
        let lenF = CGFloat(p.num("scratchLen", 55) / 100)
        for _ in 0..<(p.bool("scratchesOn", true) ? Int(p.num("scratches", 3)) : 0) {
            let light = style != "dark" && (style == "light" || r() < 0.6)
            g.setStrokeColor(light ? CGColor(srgbRed: 0.95, green: 0.95, blue: 0.95, alpha: 0.25 + r() * 0.5)
                                   : CGColor(srgbRed: 0.05, green: 0.05, blue: 0.05, alpha: 0.25 + r() * 0.5))
            g.setLineWidth((0.35 + r() * 0.9) * s)
            let x = r() * W
            let len = H * lenF * (0.5 + r() * 0.5)
            let y0 = r() * max(1, H - len)
            g.move(to: CGPoint(x: x, y: y0))
            let steps = 8
            for i in 1...steps {
                g.addLine(to: CGPoint(x: x + (r() - 0.5) * 1.5 * s, y: y0 + len * CGFloat(i) / CGFloat(steps)))
            }
            g.strokePath()
        }
        var tex = ctx.apply("k_copy", [ov.texture()])
        let soft = p.num("softness", 0.5)
        if soft > 0.01 { tex = ctx.blur(tex, sigma: ctx.px(soft)) }
        return ctx.apply("k_over_pm_op", [input, tex], [p.f("opacity", 80) / 100])
    }
}

// MARK: - Light leak

struct LightLeakEffect: Effect {
    let id = "leak"
    let name = "Light Leak"
    let category = EffectCategory.film
    var symbol: String { "sun.haze" }
    var summary: String { "Warm film-camera light leaks bleeding in from an edge." }
    let params: [ParamDef] = [
        .choice("edge", "From", [("left", "Left Edge"), ("right", "Right Edge"), ("top", "Top Edge"), ("bottom", "Bottom Edge"),
                                 ("tl", "Top-Left Corner"), ("br", "Bottom-Right Corner"), ("random", "Random Spot")], "right"),
        .slider("size", "Size", 5, 100, 45, step: 1, unit: "%"),
        .slider("intensity", "Intensity", 0, 200, 90, step: 1, unit: "%"),
        .slider("warp", "Irregularity", 0, 100, 45, step: 1, unit: "%"),
        .color("colorA", "Color A", RGBA(hex: "#FF4A1C")),
        .color("colorB", "Color B", RGBA(hex: "#FFC23A")),
        .choice("blend", "Blend", [("screen", "Screen"), ("add", "Add"), ("lighten", "Lighten")], "screen"),
        .seed(),
    ]
    var looks: [EffectLook] {
        [
            EffectLook(id: "amber", name: "Amber Leak", values: ["colorA": RGBA(hex: "#FF4A1C"), "colorB": RGBA(hex: "#FFC23A")]),
            EffectLook(id: "magenta", name: "Magenta Burn", values: ["colorA": RGBA(hex: "#FF1E6E"), "colorB": RGBA(hex: "#FF8A3D"), "edge": "left"]),
            EffectLook(id: "red", name: "Red Flare", values: ["colorA": RGBA(hex: "#E0100F"), "colorB": RGBA(hex: "#FF5A1A"), "edge": "tl", "size": 45.0]),
            EffectLook(id: "cool", name: "Cool Haze", values: ["colorA": RGBA(hex: "#3AA0FF"), "colorB": RGBA(hex: "#C6E8FF"), "edge": "top", "intensity": 70.0]),
            EffectLook(id: "end", name: "End of Roll", values: ["colorA": RGBA(hex: "#FF3A12"), "colorB": RGBA(hex: "#FFE27A"), "edge": "right",
                                                              "size": 70.0, "intensity": 160.0, "warp": 70.0]),
        ]
    }
    func apply(_ input: MTLTexture, _ p: Params, _ ctx: RenderContext) -> MTLTexture {
        let edge: Float = ["left": 0, "right": 1, "top": 2, "bottom": 3, "tl": 4, "br": 5, "random": 6][p.choice("edge", "right")] ?? 1
        let blend: Float = ["screen": 0, "add": 1, "lighten": 2][p.choice("blend", "screen")] ?? 0
        return ctx.apply("k_leak", [input], [p.f("seed", 1) * 2.7, Float(ctx.scale), edge, p.f("size", 45) / 100,
                                             p.f("intensity", 90) / 100]
                         + p.color("colorA", RGBA(hex: "#FF4A1C")).floats + p.color("colorB", RGBA(hex: "#FFC23A")).floats
                         + [p.f("warp", 45) / 100 * 0.6, blend])
    }
}

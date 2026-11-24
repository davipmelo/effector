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
    /// Crumpled sheet: flat facets with random tilt and sharp creases. Returns a shading offset.
    inline float crumpleShade(float2 q, float cs, float seed) {
        float s = 0.0, amp = 1.0;
        float2 w = float2(fbm(q / (cs * 1.7), seed + 8.0, 2), fbm(q / (cs * 1.7) + 4.3, seed + 9.0, 2)) - 0.5;
        float2 x = q / cs + w * 0.6;
        for (int o = 0; o < 3; o++) {
            float4 v = voro(x, seed + float(o) * 13.0);
            float edge = v.y - v.x;
            // each facet is tilted: brightness ramps across it towards the light
            float2 tilt = float2(v.z - 0.5, v.w - 0.5);
            float facet = (v.z - 0.5) * 0.3 + dot(tilt, float2(-0.6, -0.8)) * 0.15;
            float crease = 1.0 - smoothstep(0.0, 0.1, edge);
            float rim = (1.0 - smoothstep(0.02, 0.22, edge)) * (v.w - 0.5);
            s += (facet - crease * 0.1 + rim * 0.3) * amp;
            x = x * 2.13 + 7.7;
            amp *= 0.45;
        }
        return s;
    }

    // Composite a premultiplied overlay with opacity. u[0] = opacity
    kernel void k_over_pm_op(KIN, texture2d<float> base [[texture(1)]], texture2d<float> ov [[texture(2)]]) {
        GUARD
        float4 b = base.read(gid);
        float4 t = ov.sample(sLin, UV) * u[0];
        o.write(float4(b.rgb * (1.0 - t.a) + t.rgb, b.a), gid);
    }

    // Paper. u: 0 seed, 1 unit, 2..4 paper rgb, 5 grain, 6 fibers, 7 grain scale, 8 crumple, 9 crumple scale,
    //    10 folds, 11 fold strength, 12 stains, 13 mottle, 14 amount, 15 fade, 16 edge burn
    inline float paperHeight(float2 q, float cs, float seed) {
        return ridge(q / cs, seed) * 0.6 + ridge(q / (cs * 0.47) + 3.1, seed + 1.0) * 0.3 + ridge(q / (cs * 0.21) + 7.3, seed + 2.0) * 0.1;
    }
    kernel void k_paper(KIN, texture2d<float> a [[texture(1)]]) {
        GUARD
        float2 size = OSIZE;
        float2 p = float2(gid) + 0.5;
        float unit = max(u[1], 1e-3);
        float seed = u[0];
        float2 q = p / unit;          // reference pixels
        float2 uv = p / size;
        float3 paper = float3(u[2], u[3], u[4]);
        float gs = max(0.4, u[7]);
        float g = fbm(q / gs, seed, 3) - 0.5;
        float2 fq = float2(q.x * 0.92 + q.y * 0.38, q.y * 0.92 - q.x * 0.38);
        float fib = (vnoise(fq * float2(0.06, 1.0) / gs, seed + 5.0) - 0.5)
                  + (vnoise(fq.yx * float2(0.05, 0.8) / gs + 9.0, seed + 9.0) - 0.5) * 0.7;
        float tex = g * u[5] * 0.13 + fib * u[6] * 0.11;
        float mot = (fbm(q / 260.0, seed + 3.0, 4) - 0.5) * u[13] * 0.2;
        float st = smoothstep(0.6, 0.8, fbm(q / 170.0, seed + 11.0, 4));
        float ring = smoothstep(0.58, 0.6, fbm(q / 170.0, seed + 11.0, 4)) - st;
        float3 col = paper * (1.0 + tex + mot);
        col = mix(col, col * float3(0.88, 0.78, 0.6), (st * 0.55 + max(ring, 0.0) * 0.6) * u[12]);
        // crumple: shaded height field
        float shade = 0.0;
        if (u[8] > 0.001) {
            shade += crumpleShade(q, max(8.0, u[9]), seed) * u[8] * 0.8;
        }
        // folds
        int folds = int(u[10]);
        if (folds > 0 && u[11] > 0.001) {
            float k = u[11];
            float dx = (uv.x - 0.5) * size.x / unit;
            shade += (-exp(-abs(dx) / 1.6) * 0.22 + sign(dx) * 0.05 * exp(-abs(dx) / 90.0)) * k;
            if (folds >= 2) {
                float dy = (uv.y - 0.5) * size.y / unit;
                shade += (-exp(-abs(dy) / 1.6) * 0.22 - sign(dy) * 0.05 * exp(-abs(dy) / 90.0)) * k;
            }
            if (folds == 3) {
                float2 c0 = float2(hash12(float2(seed, 1.0)), hash12(float2(seed, 2.0)));
                float an = hash12(float2(seed, 3.0)) * 3.14159;
                float2 nn = float2(cos(an), sin(an));
                float dd = dot((uv - c0) * size / unit, nn);
                shade += (-exp(-abs(dd) / 1.4) * 0.2 + sign(dd) * 0.05 * exp(-abs(dd) / 70.0)) * k;
            }
        }
        // edge burn / handling wear
        if (u[16] > 0.001) {
            float2 e2 = min(uv, 1.0 - uv) * size / unit;
            float ed = min(e2.x, e2.y) + (fbm(q / 40.0, seed + 21.0, 3) - 0.5) * 30.0;
            shade -= (1.0 - smoothstep(0.0, 60.0, ed)) * 0.35 * u[16];
        }
        col *= 1.0 + shade;
        float4 s = a.read(gid);
        float3 img = mix(clamp(s.rgb, 0.0, 1.0), float3(1.0), u[15]);
        float3 printed = img * col;
        o.write(float4(clamp(mix(s.rgb, printed, u[14]), 0.0, 1.0), s.a), gid);
    }

    // Plastic wrap: long stretched creases catch the light. u: 0 seed, 1 unit, 2 intensity, 3 scale, 4 sharpness,
    //    5 angle, 6 refraction, 7..9 tint, 10 haze
    kernel void k_plastic(KIN, texture2d<float> a [[texture(1)]]) {
        GUARD
        float2 size = OSIZE;
        float2 p = float2(gid) + 0.5;
        float unit = max(u[1], 1e-3);
        float2 q = p / unit;
        float an = u[5] * 0.0174533;
        float2 rq = float2(cos(an) * q.x + sin(an) * q.y, -sin(an) * q.x + cos(an) * q.y);
        float sc = max(20.0, u[3]);
        float2 w = float2(fbm(rq / (sc * 1.5), u[0] + 2.0, 3), fbm(rq / (sc * 1.5) + 5.2, u[0] + 3.0, 3)) - 0.5;
        float2 x = rq / sc * float2(0.22, 1.0) + w * 0.7;
        float4 v = voro(x, u[0]);
        float4 v2 = voro(x * 2.1 + 3.3, u[0] + 7.0);
        float wid = mix(0.28, 0.06, u[4]);
        float line = exp(-(v.y - v.x) / wid) * (0.15 + 0.6 * pow(v.z, 2.0)) + exp(-(v2.y - v2.x) / wid) * 0.3 * pow(v2.z, 2.0);
        float facet = pow(v.z, 7.0) * 0.55 + pow(v2.w, 9.0) * 0.3;
        float sheen = smoothstep(0.55, 0.95, fbm(rq / (sc * 2.5), u[0] + 11.0, 3)) * 0.35;
        float hl = (line * 0.9 + facet + sheen) * u[2];
        float2 dir = float2(v.z - 0.5, v.w - 0.5);
        float2 uv = p / size + dir * u[6] * 0.012;
        float3 img = a.sample(sLin, uv).rgb;
        float3 tint = float3(u[7], u[8], u[9]);
        img = mix(img, tint, u[10] * 0.3);
        float3 c = 1.0 - (1.0 - img) * (1.0 - clamp(hl * tint, 0.0, 1.0));
        o.write(float4(clamp(c, 0.0, 1.0), a.read(gid).a), gid);
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
    let category = EffectCategory.texture
    var symbol: String { "aqi.low" }
    var summary: String { "Specks, hairs and film scratches, like a dusty scan or an old print." }
    let params: [ParamDef] = [
        .segmented("style", "Style", [("dark", "Dark"), ("light", "Light"), ("mixed", "Mixed")], "light", section: "Dust"),
        .slider("amount", "Density", 0, 100, 35, step: 1, unit: "%", curve: 1.5, section: "Dust"),
        .slider("size", "Size", 0.2, 5, 1, step: 0.05, curve: 1.8, section: "Dust"),
        .slider("hairs", "Hairs", 0, 40, 5, step: 1, section: "Dust"),
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
        let n = Int(p.num("amount", 35) / 100 * refArea * 0.0012)
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
        for _ in 0..<Int(p.num("hairs", 5)) {
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
        for _ in 0..<Int(p.num("scratches", 3)) {
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

// MARK: - Paper

struct PaperEffect: Effect {
    let id = "paper"
    let name = "Paper"
    let category = EffectCategory.texture
    var symbol: String { "doc.richtext" }
    var summary: String { "Prints the image on paper: fibres, grain, crumples, folds, stains and worn edges." }
    let params: [ParamDef] = [
        .color("paper", "Paper Color", RGBA(hex: "#F4F1EA"), section: "Paper"),
        .slider("amount", "Amount", 0, 100, 100, step: 1, unit: "%", section: "Paper"),
        .slider("fade", "Ink Fade", 0, 100, 8, step: 1, unit: "%", section: "Paper"),
        .slider("grain", "Grain", 0, 100, 45, step: 1, unit: "%", section: "Texture"),
        .slider("fibers", "Fibers", 0, 100, 35, step: 1, unit: "%", section: "Texture"),
        .slider("scale", "Texture Scale", 0.4, 6, 1.2, step: 0.05, curve: 1.6, section: "Texture"),
        .slider("mottle", "Mottling", 0, 100, 30, step: 1, unit: "%", section: "Texture"),
        .slider("crumple", "Crumple", 0, 100, 0, step: 1, unit: "%", section: "Crumple & Folds"),
        .slider("crumpleScale", "Crumple Scale", 20, 400, 120, step: 1, curve: 1.5, section: "Crumple & Folds"),
        .choice("folds", "Folds", [("none", "None"), ("half", "Folded in Half"), ("quarters", "Folded in Quarters"),
                                   ("random", "Random Crease")], "none", section: "Crumple & Folds"),
        .slider("foldStrength", "Fold Strength", 0, 100, 60, step: 1, unit: "%", section: "Crumple & Folds",
                visible: whenNot("folds", "none")),
        .slider("stains", "Stains", 0, 100, 0, step: 1, unit: "%", section: "Aging"),
        .slider("edges", "Edge Wear", 0, 100, 0, step: 1, unit: "%", section: "Aging"),
        .seed(section: "Aging"),
    ]
    var looks: [EffectLook] {
        [
            EffectLook(id: "fine", name: "Fine Art Paper", values: ["paper": RGBA(hex: "#F6F3EC"), "grain": 40.0, "fibers": 25.0]),
            EffectLook(id: "newsprint", name: "Newsprint", values: ["paper": RGBA(hex: "#E9E4D6"), "grain": 65.0, "fibers": 55.0,
                                                                   "mottle": 45.0, "fade": 18.0, "scale": 0.9]),
            EffectLook(id: "kraft", name: "Kraft Paper", values: ["paper": RGBA(hex: "#C9A97C"), "grain": 70.0, "fibers": 80.0,
                                                                 "mottle": 40.0, "fade": 10.0, "scale": 1.6]),
            EffectLook(id: "receipt", name: "Thermal Receipt", values: ["paper": RGBA(hex: "#F3F3F1"), "grain": 20.0, "fibers": 10.0,
                                                                       "mottle": 15.0, "crumple": 35.0, "crumpleScale": 160.0,
                                                                       "folds": "random", "foldStrength": 50.0]),
            EffectLook(id: "crumpled", name: "Crumpled Paper", values: ["crumple": 85.0, "crumpleScale": 110.0, "grain": 40.0,
                                                                       "mottle": 35.0, "edges": 20.0]),
            EffectLook(id: "letter", name: "Folded Letter", values: ["folds": "quarters", "foldStrength": 70.0, "crumple": 15.0,
                                                                    "grain": 35.0, "edges": 15.0]),
            EffectLook(id: "aged", name: "Aged & Stained", values: ["paper": RGBA(hex: "#EADFC8"), "stains": 70.0, "edges": 60.0,
                                                                   "mottle": 60.0, "fade": 15.0, "crumple": 20.0]),
            EffectLook(id: "watercolor", name: "Watercolor Paper", values: ["grain": 90.0, "fibers": 20.0, "scale": 3.0,
                                                                           "mottle": 25.0, "paper": RGBA(hex: "#F7F5EF")]),
        ]
    }
    func apply(_ input: MTLTexture, _ p: Params, _ ctx: RenderContext) -> MTLTexture {
        let folds: Float = ["none": 0, "half": 1, "quarters": 2, "random": 3][p.choice("folds", "none")] ?? 0
        let u: [Float] = [p.f("seed", 1) * 3.17, Float(ctx.scale)] + p.color("paper", RGBA(hex: "#F4F1EA")).floats + [
            p.f("grain", 45) / 100, p.f("fibers", 35) / 100, p.f("scale", 1.2), p.f("crumple") / 100, p.f("crumpleScale", 120),
            folds, p.f("foldStrength", 60) / 100, p.f("stains") / 100, p.f("mottle", 30) / 100,
            p.f("amount", 100) / 100, p.f("fade", 8) / 100, p.f("edges") / 100,
        ]
        return ctx.apply("k_paper", [input], u)
    }
}

// MARK: - Plastic wrap

struct PlasticEffect: Effect {
    let id = "plastic"
    let name = "Plastic Wrap"
    let category = EffectCategory.texture
    var symbol: String { "bubble.left.and.text.bubble.right" }
    var summary: String { "Wrinkled cling-film / sleeve highlights with a little refraction." }
    let params: [ParamDef] = [
        .slider("intensity", "Intensity", 0, 150, 55, step: 1, unit: "%"),
        .slider("scale", "Wrinkle Scale", 20, 600, 160, step: 1, curve: 1.6),
        .slider("sharpness", "Sharpness", 0, 100, 35, step: 1, unit: "%"),
        .slider("angle", "Direction", -90, 90, 20, step: 1, unit: "°"),
        .slider("refraction", "Refraction", 0, 100, 25, step: 1, unit: "%"),
        .slider("haze", "Haze", 0, 100, 15, step: 1, unit: "%"),
        .color("tint", "Tint", RGBA(hex: "#F4F8FF")),
        .seed(),
    ]
    func apply(_ input: MTLTexture, _ p: Params, _ ctx: RenderContext) -> MTLTexture {
        ctx.apply("k_plastic", [input], [p.f("seed", 1) * 5.3, Float(ctx.scale), p.f("intensity", 55) / 100, p.f("scale", 160),
                                         p.f("sharpness", 35) / 100, p.f("angle", 20), p.f("refraction", 25) / 100]
                  + p.color("tint", RGBA(hex: "#F4F8FF")).floats + [p.f("haze", 15) / 100])
    }
}

// MARK: - Light leak

struct LightLeakEffect: Effect {
    let id = "leak"
    let name = "Light Leak"
    let category = EffectCategory.light
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

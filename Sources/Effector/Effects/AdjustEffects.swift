import Foundation
import Metal

// MARK: - Curve math (monotone cubic / Fritsch–Carlson)

enum CurveMath {
    static func evaluate(_ pts: [CurvePoint], _ x: Double) -> Double {
        let p = pts.sorted { $0.x < $1.x }
        guard p.count >= 2 else { return x }
        if x <= p[0].x { return p[0].y }
        if x >= p[p.count - 1].x { return p[p.count - 1].y }
        let n = p.count
        var d = [Double](repeating: 0, count: n - 1)
        for i in 0..<(n - 1) {
            let dx = max(1e-6, p[i + 1].x - p[i].x)
            d[i] = (p[i + 1].y - p[i].y) / dx
        }
        var m = [Double](repeating: 0, count: n)
        m[0] = d[0]; m[n - 1] = d[n - 2]
        if n > 2 {
            for i in 1..<(n - 1) {
                m[i] = d[i - 1] * d[i] <= 0 ? 0 : (d[i - 1] + d[i]) / 2
            }
        }
        for i in 0..<(n - 1) where abs(d[i]) < 1e-9 {
            m[i] = 0; m[i + 1] = 0
        }
        for i in 0..<(n - 1) where abs(d[i]) >= 1e-9 {
            let a = m[i] / d[i], b = m[i + 1] / d[i]
            let s = a * a + b * b
            if s > 9 {
                let t = 3 / s.squareRoot()
                m[i] = t * a * d[i]; m[i + 1] = t * b * d[i]
            }
        }
        var k = 0
        while k < n - 2 && x > p[k + 1].x { k += 1 }
        let h = p[k + 1].x - p[k].x
        let t = (x - p[k].x) / h
        let t2 = t * t, t3 = t2 * t
        let y = (2 * t3 - 3 * t2 + 1) * p[k].y + (t3 - 2 * t2 + t) * h * m[k]
            + (-2 * t3 + 3 * t2) * p[k + 1].y + (t3 - t2) * h * m[k + 1]
        return min(1, max(0, y))
    }

    static func lut(_ c: CurveSet, count: Int = 256) -> [SIMD4<Float>] {
        (0..<count).map { i in
            let x = Double(i) / Double(count - 1)
            let r = evaluate(c.master, evaluate(c.red, x))
            let g = evaluate(c.master, evaluate(c.green, x))
            let b = evaluate(c.master, evaluate(c.blue, x))
            return SIMD4(Float(r), Float(g), Float(b), 1)
        }
    }
}

// MARK: - Shaders

enum AdjustShaders {
    static let source = """
    // u: exposure, contrast, highlights, shadows, whites, blacks, temp, tint, vibrance, saturation
    kernel void k_basic(KIN, texture2d<float> a [[texture(1)]]) {
        GUARD
        float4 src = a.read(gid);
        float3 lin = toLinear(src.rgb) * exp2(u[0]);
        float t = u[6] / 100.0, ti = u[7] / 100.0;
        lin *= float3(1.0 + 0.28 * t, 1.0, 1.0 - 0.28 * t);
        lin *= float3(1.0 + 0.12 * ti, 1.0 - 0.2 * ti, 1.0 + 0.12 * ti);
        float3 c = toSRGB(lin);
        float l = clamp(luma(c), 0.0, 1.0);
        float ws = 6.75 * l * (1.0 - l) * (1.0 - l);
        float wh = 6.75 * l * l * (1.0 - l);
        float dl = u[3] / 100.0 * 0.35 * ws + u[2] / 100.0 * 0.35 * wh
                 + u[4] / 100.0 * 0.25 * l * l + u[5] / 100.0 * 0.25 * (1.0 - l) * (1.0 - l);
        c += dl;
        float k = u[1] / 100.0;
        float cf = k >= 0.0 ? 1.0 + k * 1.5 : 1.0 + k * 0.9;
        c = (c - 0.5) * cf + 0.5;
        float g = luma(c);
        float sat = (max(c.r, max(c.g, c.b)) - min(c.r, min(c.g, c.b)));
        float vib = u[8] / 100.0 * (1.0 - clamp(sat, 0.0, 1.0));
        c = mix(float3(g), c, 1.0 + vib);
        c = mix(float3(luma(c)), c, 1.0 + u[9] / 100.0);
        o.write(float4(clamp(c, 0.0, 1.0), src.a), gid);
    }

    kernel void k_curves(KIN, texture2d<float> a [[texture(1)]], texture2d<float> lut [[texture(2)]]) {
        GUARD
        float4 c = clamp(a.read(gid), 0.0, 1.0);
        float n = float(lut.get_width());
        float3 r;
        r.r = lut.sample(sLin, float2((c.r * (n - 1.0) + 0.5) / n, 0.5)).r;
        r.g = lut.sample(sLin, float2((c.g * (n - 1.0) + 0.5) / n, 0.5)).g;
        r.b = lut.sample(sLin, float2((c.b * (n - 1.0) + 0.5) / n, 0.5)).b;
        o.write(float4(r, c.a), gid);
    }

    // u: 8 bands x (hue, sat, lum) in -100..100
    kernel void k_hsl(KIN, texture2d<float> a [[texture(1)]]) {
        GUARD
        float4 src = a.read(gid);
        float3 h = rgb2hsl(clamp(src.rgb, 0.0, 1.0));
        const float centers[8] = {0.0, 30.0, 60.0, 120.0, 180.0, 240.0, 280.0, 320.0};
        float deg = h.x * 360.0;
        float dh = 0, ds = 0, dl = 0;
        for (int i = 0; i < 8; i++) {
            float c0 = centers[i];
            float cPrev = centers[(i + 7) % 8]; float cNext = centers[(i + 1) % 8];
            float dPrev = fmod(c0 - cPrev + 360.0, 360.0);
            float dNext = fmod(cNext - c0 + 360.0, 360.0);
            float d = deg - c0;
            d = fmod(d + 540.0, 360.0) - 180.0;
            float w = d >= 0.0 ? 1.0 - d / dNext : 1.0 + d / dPrev;
            w = clamp(w, 0.0, 1.0);
            dh += w * u[i * 3 + 0];
            ds += w * u[i * 3 + 1];
            dl += w * u[i * 3 + 2];
        }
        float satW = smoothstep(0.0, 0.15, h.y);
        h.x = fract(h.x + dh / 100.0 * 30.0 / 360.0 * satW);
        h.y = clamp(h.y * (1.0 + ds / 100.0), 0.0, 1.0);
        float3 c = hsl2rgb(h);
        c = clamp(c + dl / 100.0 * 0.4 * satW * h.y, 0.0, 1.0);
        o.write(float4(c, src.a), gid);
    }

    // u: hue(deg), sat, light, colorize, colorHue(deg), colorSat
    kernel void k_huesat(KIN, texture2d<float> a [[texture(1)]]) {
        GUARD
        float4 src = a.read(gid);
        float3 h = rgb2hsl(clamp(src.rgb, 0.0, 1.0));
        if (u[3] > 0.5) {
            h.x = u[4] / 360.0;
            h.y = u[5] / 100.0;
        } else {
            h.x = fract(h.x + u[0] / 360.0);
            h.y = clamp(h.y * (1.0 + u[1] / 100.0), 0.0, 1.0);
        }
        float3 c = hsl2rgb(h);
        float L = u[2] / 100.0;
        c = L >= 0.0 ? mix(c, float3(1.0), L) : c * (1.0 + L);
        o.write(float4(clamp(c, 0.0, 1.0), src.a), gid);
    }

    // u: reds, yellows, greens, cyans, blues, magentas (percent), contrast
    kernel void k_bw(KIN, texture2d<float> a [[texture(1)]]) {
        GUARD
        float4 src = a.read(gid);
        float3 c = clamp(src.rgb, 0.0, 1.0);
        float mn = min(c.r, min(c.g, c.b));
        float mx = max(c.r, max(c.g, c.b));
        float md = c.r + c.g + c.b - mn - mx;
        float wR = u[0] / 100.0, wY = u[1] / 100.0, wG = u[2] / 100.0;
        float wC = u[3] / 100.0, wB = u[4] / 100.0, wM = u[5] / 100.0;
        float wp, ws;
        if (c.r >= c.g && c.r >= c.b) { wp = wR; ws = c.g >= c.b ? wY : wM; }
        else if (c.g >= c.r && c.g >= c.b) { wp = wG; ws = c.r >= c.b ? wY : wC; }
        else { wp = wB; ws = c.r >= c.g ? wM : wC; }
        float g = mn + (md - mn) * ws + (mx - md) * wp;
        float k = u[6] / 100.0;
        g = (g - 0.5) * (1.0 + k) + 0.5;
        g = clamp(g, 0.0, 1.0);
        o.write(float4(g, g, g, src.a), gid);
    }
    """
}

// MARK: - Effects

struct BasicAdjustEffect: Effect {
    let id = "basic"
    let name = "Basic Adjustments"
    let category = EffectCategory.adjust
    var symbol: String { "slider.horizontal.3" }
    let params: [ParamDef] = [
        .slider("exposure", "Exposure", -4, 4, 0, unit: " EV", section: "Light"),
        .slider("contrast", "Contrast", -100, 100, 0, section: "Light"),
        .slider("highlights", "Highlights", -100, 100, 0, section: "Light"),
        .slider("shadows", "Shadows", -100, 100, 0, section: "Light"),
        .slider("whites", "Whites", -100, 100, 0, section: "Light"),
        .slider("blacks", "Blacks", -100, 100, 0, section: "Light"),
        .slider("temperature", "Temperature", -100, 100, 0, section: "Color"),
        .slider("tint", "Tint", -100, 100, 0, section: "Color"),
        .slider("vibrance", "Vibrance", -100, 100, 0, section: "Color"),
        .slider("saturation", "Saturation", -100, 100, 0, section: "Color"),
    ]
    func apply(_ input: MTLTexture, _ p: Params, _ ctx: RenderContext) -> MTLTexture {
        ctx.apply("k_basic", [input], ["exposure", "contrast", "highlights", "shadows", "whites", "blacks",
                                         "temperature", "tint", "vibrance", "saturation"].map { p.f($0) })
    }
}

struct CurvesEffect: Effect {
    let id = "curves"
    let name = "Curves"
    let category = EffectCategory.adjust
    var symbol: String { "point.topleft.down.to.point.bottomright.curvepath" }
    let params: [ParamDef] = [ParamDef(key: "curves", label: "Curves", kind: .curves)]
    func apply(_ input: MTLTexture, _ p: Params, _ ctx: RenderContext) -> MTLTexture {
        let lut = ctx.gpu.makeLUT1D(CurveMath.lut(p.curves("curves")))
        return ctx.apply("k_curves", [input, lut])
    }
}

struct HSLEffect: Effect {
    static let bands = ["Red", "Orange", "Yellow", "Green", "Aqua", "Blue", "Purple", "Magenta"]
    let id = "hsl"
    let name = "HSL"
    let category = EffectCategory.adjust
    var symbol: String { "circle.lefthalf.filled.righthalf.striped.horizontal" }
    var params: [ParamDef] {
        HSLEffect.bands.flatMap { b -> [ParamDef] in
            let k = b.lowercased()
            return [
                .slider("\(k)_h", "Hue", -100, 100, 0, section: b),
                .slider("\(k)_s", "Saturation", -100, 100, 0, section: b),
                .slider("\(k)_l", "Luminance", -100, 100, 0, section: b),
            ]
        }
    }
    func apply(_ input: MTLTexture, _ p: Params, _ ctx: RenderContext) -> MTLTexture {
        var u: [Float] = []
        for b in HSLEffect.bands {
            let k = b.lowercased()
            u += [p.f("\(k)_h"), p.f("\(k)_s"), p.f("\(k)_l")]
        }
        return ctx.apply("k_hsl", [input], u)
    }
}

struct HueSatEffect: Effect {
    let id = "huesat"
    let name = "Hue / Saturation"
    let category = EffectCategory.adjust
    var symbol: String { "drop.halffull" }
    let params: [ParamDef] = [
        .slider("hue", "Hue", -180, 180, 0, unit: "°"),
        .slider("saturation", "Saturation", -100, 100, 0),
        .slider("lightness", "Lightness", -100, 100, 0),
        .toggle("colorize", "Colorize", false),
        .slider("colorHue", "Color Hue", 0, 360, 30, unit: "°", visible: whenOn("colorize")),
        .slider("colorSat", "Color Saturation", 0, 100, 40, visible: whenOn("colorize")),
    ]
    func apply(_ input: MTLTexture, _ p: Params, _ ctx: RenderContext) -> MTLTexture {
        ctx.apply("k_huesat", [input], [p.f("hue"), p.f("saturation"), p.f("lightness"),
                                          p.bool("colorize") ? 1 : 0, p.f("colorHue"), p.f("colorSat")])
    }
}

struct BlackWhiteEffect: Effect {
    let id = "bw"
    let name = "Black & White"
    let category = EffectCategory.adjust
    var symbol: String { "circle.lefthalf.filled" }
    let params: [ParamDef] = [
        .slider("reds", "Reds", -200, 300, 40, unit: "%"),
        .slider("yellows", "Yellows", -200, 300, 60, unit: "%"),
        .slider("greens", "Greens", -200, 300, 40, unit: "%"),
        .slider("cyans", "Cyans", -200, 300, 60, unit: "%"),
        .slider("blues", "Blues", -200, 300, 20, unit: "%"),
        .slider("magentas", "Magentas", -200, 300, 80, unit: "%"),
        .slider("contrast", "Contrast", -100, 100, 0),
    ]
    func apply(_ input: MTLTexture, _ p: Params, _ ctx: RenderContext) -> MTLTexture {
        ctx.apply("k_bw", [input], ["reds", "yellows", "greens", "cyans", "blues", "magentas", "contrast"].map { p.f($0) })
    }
}

struct InvertEffect: Effect {
    let id = "invert"
    let name = "Invert"
    let category = EffectCategory.adjust
    var symbol: String { "circle.righthalf.filled.inverse" }
    let params: [ParamDef] = []
    func apply(_ input: MTLTexture, _ p: Params, _ ctx: RenderContext) -> MTLTexture {
        ctx.apply("k_invert", [input])
    }
}

struct PosterizeEffect: Effect {
    let id = "posterize"
    let name = "Posterize"
    let category = EffectCategory.adjust
    var symbol: String { "square.stack.3d.down.forward" }
    let params: [ParamDef] = [.slider("levels", "Levels", 2, 16, 4, step: 1)]
    func apply(_ input: MTLTexture, _ p: Params, _ ctx: RenderContext) -> MTLTexture {
        ctx.apply("k_posterize", [input], [p.f("levels", 4)])
    }
}

struct ThresholdEffect: Effect {
    let id = "threshold"
    let name = "Threshold"
    let category = EffectCategory.adjust
    var symbol: String { "circle.and.line.horizontal" }
    let params: [ParamDef] = [
        .slider("level", "Level", 0, 100, 50, unit: "%"),
        .slider("softness", "Softness", 0, 50, 0, unit: "%"),
        .color("dark", "Dark", .black),
        .color("light", "Light", .white),
    ]
    func apply(_ input: MTLTexture, _ p: Params, _ ctx: RenderContext) -> MTLTexture {
        ctx.apply("k_threshold", [input], [p.f("level", 50) / 100, p.f("softness") / 100]
                  + p.color("dark").floats + p.color("light", .white).floats)
    }
}

struct BlurEffect: Effect {
    let id = "blur"
    let name = "Blur"
    let category = EffectCategory.light
    var symbol: String { "aqi.medium" }
    let params: [ParamDef] = [.slider("radius", "Radius", 0, 200, 8, unit: " px")]
    func apply(_ input: MTLTexture, _ p: Params, _ ctx: RenderContext) -> MTLTexture {
        ctx.blur(input, sigma: ctx.px(p.num("radius", 8)) / 2)
    }
}

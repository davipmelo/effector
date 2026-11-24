import Foundation
import Metal

enum DitherShaders {
    static let source = """
    // Pre-pass: brightness, contrast, gamma; mono → neutral grey. u: brightness, contrast, mono, gamma
    kernel void k_dither_pre(KIN, texture2d<float> a [[texture(1)]]) {
        GUARD
        float4 s = a.read(gid);
        float3 c = clamp(s.rgb, 0.0, 1.0);
        c = pow(c, float3(1.0 / max(0.05, u[3])));
        c = c + u[0] / 100.0;
        float k = u[1] / 100.0;
        c = (c - 0.5) * (k >= 0.0 ? 1.0 + k * 2.0 : 1.0 + k) + 0.5;
        c = clamp(c, 0.0, 1.0);
        if (u[2] > 0.5) c = float3(neutralGray(c));
        o.write(float4(c, s.a), gid);
    }

    constant float bayer2[4] = {0, 2, 3, 1};
    constant float bayer4[16] = {0, 8, 2, 10, 12, 4, 14, 6, 3, 11, 1, 9, 15, 7, 13, 5};
    inline float bayer8v(uint2 p) {
        uint x = p.x % 8, y = p.y % 8;
        uint v = 0;
        uint xy = x ^ y;
        for (uint b = 0; b < 3; b++) {
            v = (v << 2) | (((xy >> b) & 1u) << 1) | ((x >> b) & 1u);
        }
        return float(v);
    }
    inline float ditherThreshold(uint2 p, int kind, float seed) {
        switch (kind) {
            case 0: return (bayer2[(p.y % 2) * 2 + (p.x % 2)] + 0.5) / 4.0;
            case 1: return (bayer4[(p.y % 4) * 4 + (p.x % 4)] + 0.5) / 16.0;
            case 2: return (bayer8v(p) + 0.5) / 64.0;
            case 3: return 0.5;
            case 4: return hash13(float3(float2(p), seed));
            case 5: { // interleaved gradient noise (blue-noise like)
                float2 q = float2(p) + seed * 5.588238;
                return fract(52.9829189 * fract(0.06711056 * q.x + 0.00583715 * q.y));
            }
            case 6: { // horizontal lines
                return (float(p.y % 4) + 0.5) / 4.0;
            }
            case 7: { // crosshatch-ish cluster dot 4x4
                const float cl[16] = {12, 5, 6, 13, 4, 0, 1, 7, 11, 3, 2, 8, 15, 10, 9, 14};
                return (cl[(p.y % 4) * 4 + (p.x % 4)] + 0.5) / 16.0;
            }
        }
        return 0.5;
    }

    // u: kind, mono, levels, spread, seed, dark rgb(5..7), light rgb(8..10)
    // ex: palette (count, rgb...)
    kernel void k_dither_ordered(KIN, texture2d<float> a [[texture(1)]]) {
        GUARD
        float4 s = a.read(gid);
        float t = ditherThreshold(gid, int(u[0]), u[4]);
        float spread = u[3];
        float3 outc;
        if (u[1] > 0.5) {
            float L = max(2.0, u[2]) - 1.0;
            float v = s.r * L;
            float q = floor(v + mix(0.5, t, spread));
            q = clamp(q / L, 0.0, 1.0);
            outc = mix(float3(u[5], u[6], u[7]), float3(u[8], u[9], u[10]), q);
        } else {
            int n = int(ex[0]);
            float r = spread * 0.5 / max(1.0, pow(float(n), 1.0 / 3.0) - 0.5);
            float3 c = s.rgb + (t - 0.5) * r;
            outc = nearestPalette(c, ex, n);
        }
        o.write(float4(outc, s.a), gid);
    }
    """
}

struct DitherEffect: Effect {
    let id = "dither"
    let name = "Dither"
    let category = EffectCategory.stylize
    var symbol: String { "circle.grid.3x3.fill" }

    static let diffusionKinds = ["floyd", "atkinson", "jjn", "stucki", "sierra", "sierralite", "burkes"]
    static let orderedKinds = ["bayer2": 0, "bayer4": 1, "bayer8": 2, "threshold": 3, "random": 4, "bluenoise": 5, "lines": 6, "cluster": 7]

    let params: [ParamDef] = [
        .choice("algorithm", "Algorithm", [
            ("floyd", "Floyd–Steinberg"), ("atkinson", "Atkinson"), ("jjn", "Jarvis–Judice–Ninke"), ("stucki", "Stucki"),
            ("sierra", "Sierra"), ("sierralite", "Sierra Lite"), ("burkes", "Burkes"),
            ("bayer2", "Bayer 2×2"), ("bayer4", "Bayer 4×4"), ("bayer8", "Bayer 8×8"), ("cluster", "Clustered Dot"),
            ("bluenoise", "Blue Noise"), ("random", "Random"), ("lines", "Lines"), ("threshold", "Threshold"),
        ], "floyd", section: "Dither"),
        .slider("pixelSize", "Pixel Size", 0.2, 24, 2, step: 0.1, unit: "", section: "Dither"),
        .slider("strength", "Strength", 0, 100, 100, unit: "%", section: "Dither"),
        .toggle("serpentine", "Serpentine", true, section: "Dither",
                visible: { DitherEffect.diffusionKinds.contains($0.choice("algorithm")) }),
        .seed(section: "Dither"),
        .segmented("mode", "Color Mode", [("mono", "Mono"), ("palette", "Palette")], "mono", section: "Color"),
        .slider("levels", "Levels", 2, 8, 2, step: 1, section: "Color", visible: when("mode", "mono")),
        .color("dark", "Dark", RGBA(hex: "#0A0A0A"), section: "Color", visible: when("mode", "mono")),
        .color("light", "Light", RGBA(hex: "#F2F2F2"), section: "Color", visible: when("mode", "mono")),
        ParamDef(key: "palette", label: "Palette", kind: .palette(def: PalettePresets.all[1]), section: "Color",
                 visible: when("mode", "palette")),
        .slider("brightness", "Brightness", -100, 100, 0, section: "Tone"),
        .slider("contrast", "Contrast", -100, 100, 0, section: "Tone"),
        .slider("gamma", "Gamma", 0.2, 3, 1, section: "Tone"),
    ]

    func apply(_ input: MTLTexture, _ p: Params, _ ctx: RenderContext) -> MTLTexture {
        let mono = p.choice("mode", "mono") == "mono"
        let pix = max(1, ctx.px(p.num("pixelSize", 2)))
        let sw = max(1, Int((Double(input.width) / pix).rounded()))
        let sh = max(1, Int((Double(input.height) / pix).rounded()))
        let small = ctx.resize(input, sw, sh)
        let pre = ctx.apply("k_dither_pre", [small], [p.f("brightness"), p.f("contrast"), mono ? 1 : 0, p.f("gamma", 1)])
        let palette = p.palette("palette") ?? PalettePresets.all[1]
        let dark = p.color("dark", RGBA(hex: "#0A0A0A")), light = p.color("light", RGBA(hex: "#F2F2F2"))
        let levels = Int(p.num("levels", 2))
        let strength = Float(p.num("strength", 100) / 100)
        let algo = p.choice("algorithm", "floyd")
        var result: MTLTexture
        if let kind = DitherEffect.orderedKinds[algo] {
            var ex: [Float] = [Float(palette.colors.count)]
            for c in palette.colors { ex += c.floats }
            result = ctx.apply("k_dither_ordered", [pre],
                               [Float(kind), mono ? 1 : 0, Float(levels), strength, p.f("seed", 1)] + dark.floats + light.floats,
                               extra: ex)
        } else {
            var px = ctx.readFloats(pre)
            ErrorDiffusion.run(&px, sw, sh, kernel: algo, serpentine: p.bool("serpentine", true), strength: strength,
                               mono: mono, levels: levels, dark: dark, light: light, palette: palette.colors)
            result = ctx.upload(px, sw, sh)
        }
        return ctx.upscaleNearest(result, input.width, input.height)
    }
}

enum ErrorDiffusion {
    typealias Tap = (dx: Int, dy: Int, w: Float)

    static func taps(_ k: String) -> [Tap] {
        switch k {
        case "atkinson":
            return [(1, 0, 1), (2, 0, 1), (-1, 1, 1), (0, 1, 1), (1, 1, 1), (0, 2, 1)].map { ($0.0, $0.1, $0.2 / 8) }
        case "jjn":
            return [(1, 0, 7), (2, 0, 5), (-2, 1, 3), (-1, 1, 5), (0, 1, 7), (1, 1, 5), (2, 1, 3),
                    (-2, 2, 1), (-1, 2, 3), (0, 2, 5), (1, 2, 3), (2, 2, 1)].map { ($0.0, $0.1, $0.2 / 48) }
        case "stucki":
            return [(1, 0, 8), (2, 0, 4), (-2, 1, 2), (-1, 1, 4), (0, 1, 8), (1, 1, 4), (2, 1, 2),
                    (-2, 2, 1), (-1, 2, 2), (0, 2, 4), (1, 2, 2), (2, 2, 1)].map { ($0.0, $0.1, $0.2 / 42) }
        case "sierra":
            return [(1, 0, 5), (2, 0, 3), (-2, 1, 2), (-1, 1, 4), (0, 1, 5), (1, 1, 4), (2, 1, 2),
                    (-1, 2, 2), (0, 2, 3), (1, 2, 2)].map { ($0.0, $0.1, $0.2 / 32) }
        case "sierralite":
            return [(1, 0, 2), (-1, 1, 1), (0, 1, 1)].map { ($0.0, $0.1, $0.2 / 4) }
        case "burkes":
            return [(1, 0, 8), (2, 0, 4), (-2, 1, 2), (-1, 1, 4), (0, 1, 8), (1, 1, 4), (2, 1, 2)].map { ($0.0, $0.1, $0.2 / 32) }
        default: // floyd
            return [(1, 0, 7), (-1, 1, 3), (0, 1, 5), (1, 1, 1)].map { ($0.0, $0.1, $0.2 / 16) }
        }
    }

    /// In-place error diffusion on an RGBA float buffer.
    static func run(_ px: inout [Float], _ w: Int, _ h: Int, kernel: String, serpentine: Bool, strength: Float,
                    mono: Bool, levels: Int, dark: RGBA, light: RGBA, palette: [RGBA]) {
        let taps = taps(kernel)
        let pal = palette.map { SIMD3<Float>(Float($0.r), Float($0.g), Float($0.b)) }
        let L = Float(max(2, levels) - 1)
        let d = SIMD3<Float>(Float(dark.r), Float(dark.g), Float(dark.b))
        let l = SIMD3<Float>(Float(light.r), Float(light.g), Float(light.b))
        px.withUnsafeMutableBufferPointer { buf in
            let b = buf.baseAddress!
            if mono {
                // single channel (r) — grey already neutral
                var g = [Float](repeating: 0, count: w * h)
                for i in 0..<(w * h) { g[i] = b[i * 4] }
                g.withUnsafeMutableBufferPointer { gb in
                    let gp = gb.baseAddress!
                    for y in 0..<h {
                        let rev = serpentine && y % 2 == 1
                        for xi in 0..<w {
                            let x = rev ? w - 1 - xi : xi
                            let i = y * w + x
                            let old = gp[i]
                            let q = min(L, max(0, (old * L).rounded())) / L
                            gp[i] = q
                            let err = (old - q) * strength
                            for t in taps {
                                let nx = x + (rev ? -t.dx : t.dx), ny = y + t.dy
                                if nx >= 0 && nx < w && ny < h { gp[ny * w + nx] += err * t.w }
                            }
                        }
                    }
                }
                for i in 0..<(w * h) {
                    let q = min(1, max(0, g[i]))
                    let c = d + (l - d) * q
                    b[i * 4] = c.x; b[i * 4 + 1] = c.y; b[i * 4 + 2] = c.z
                }
            } else {
                for y in 0..<h {
                    let rev = serpentine && y % 2 == 1
                    for xi in 0..<w {
                        let x = rev ? w - 1 - xi : xi
                        let i = (y * w + x) * 4
                        let old = SIMD3<Float>(b[i], b[i + 1], b[i + 2])
                        var best = pal.first ?? .zero
                        var bd = Float.infinity
                        for c in pal {
                            let e = old - c
                            let dist = e.x * e.x * 0.299 + e.y * e.y * 0.587 + e.z * e.z * 0.114
                            if dist < bd { bd = dist; best = c }
                        }
                        b[i] = best.x; b[i + 1] = best.y; b[i + 2] = best.z
                        let err = (old - best) * strength
                        for t in taps {
                            let nx = x + (rev ? -t.dx : t.dx), ny = y + t.dy
                            if nx >= 0 && nx < w && ny < h {
                                let j = (ny * w + nx) * 4
                                b[j] += err.x * t.w; b[j + 1] += err.y * t.w; b[j + 2] += err.z * t.w
                            }
                        }
                    }
                }
            }
        }
    }
}

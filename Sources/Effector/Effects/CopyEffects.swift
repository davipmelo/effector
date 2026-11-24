import Foundation
import Metal

enum CopyShaders {
    static let source = """
    // Photocopy. u: 0 seed, 1 unit, 2 threshold, 3 softness, 4 toner noise, 5 streaks, 6 edge shadow, 7 uneven,
    //    8..10 toner rgb, 11..13 paper rgb, 14 background noise, 15 dither
    kernel void k_xerox(KIN, texture2d<float> a [[texture(1)]]) {
        GUARD
        float2 size = OSIZE;
        float2 p = float2(gid) + 0.5;
        float unit = max(u[1], 1e-3);
        float2 q = p / unit;
        float2 uv = p / size;
        float seed = u[0];
        float4 s = a.read(gid);
        float d = 1.0 - neutralGray(clamp(s.rgb, 0.0, 1.0));     // darkness
        // uneven toner density across the page
        d += (fbm(q / 180.0, seed, 3) - 0.5) * u[7] * 0.5;
        // drum streaks (vertical)
        float col = floor(q.x / 2.0);
        float st = hash12(float2(col, seed + 7.0));
        float streak = (step(0.985, st) * 0.6 - step(0.992, hash12(float2(col, seed + 9.0))) * 0.5) * u[5];
        streak *= 0.6 + 0.4 * vnoise(float2(col * 0.1, q.y / 300.0), seed + 3.0);
        d += streak;
        // shadow along the page edges (lid not closed)
        float2 e = min(uv, 1.0 - uv) * size / unit;
        float edge = 1.0 - smoothstep(0.0, 70.0, min(e.x, e.y) + (fbm(q / 25.0, seed + 5.0, 3) - 0.5) * 40.0);
        d += edge * u[6];
        // toner noise
        d += (hash12(floor(q * 1.2) + seed) - 0.5) * u[4] * 0.6;
        float th = u[2];
        float sft = max(0.003, u[3]);
        float cov;
        if (u[15] > 0.5) {
            float ign = fract(52.9829189 * fract(0.06711056 * floor(q.x) + 0.00583715 * floor(q.y)));
            cov = step(ign, smoothstep(th - sft * 3.0, th + sft * 3.0, d));
        } else {
            cov = smoothstep(th - sft, th + sft, d);
        }
        // background specks
        float bg = step(1.0 - u[14] * 0.012, hash12(floor(q / 1.5) + seed * 3.1));
        cov = max(cov, bg * 0.85);
        float3 toner = float3(u[8], u[9], u[10]), paper = float3(u[11], u[12], u[13]);
        o.write(float4(mix(paper, toner, clamp(cov, 0.0, 1.0)), s.a), gid);
    }

    // Thermal print finish. u: 0 seed, 1 unit, 2..4 ink, 5..7 paper, 8 fade, 9 dead lines, 10 banding, 11 gradient fade
    kernel void k_thermal(KIN, texture2d<float> a [[texture(1)]]) {
        GUARD
        float2 size = OSIZE;
        float2 p = float2(gid) + 0.5;
        float unit = max(u[1], 1e-3);
        float2 q = p / unit;
        float2 uv = p / size;
        float seed = u[0];
        float4 s = a.read(gid);
        float ink = 1.0 - s.r;                        // dithered 0/1 coverage (white = paper)
        // print head density varies along the paper feed + overall fade
        float dens = 1.0 - u[8] * 0.6;
        dens -= (fbm(float2(q.y / 220.0, 0.5), seed, 3) - 0.5) * u[10] * 0.7;
        dens -= uv.y * u[11] * 0.5;
        // dead heating elements leave thin white lines down the page
        float col = floor(q.x / 1.6);
        float dead = step(1.0 - u[9] * 0.02, hash12(float2(col, seed + 3.0)));
        dens *= 1.0 - dead * 0.85;
        ink *= clamp(dens + (hash12(floor(q) + seed) - 0.5) * 0.15, 0.0, 1.0);
        float3 c = mix(float3(u[5], u[6], u[7]), float3(u[2], u[3], u[4]), ink);
        o.write(float4(c, s.a), gid);
    }
    """
}

struct PhotocopyEffect: Effect {
    let id = "xerox"
    let name = "Photocopy"
    let category = EffectCategory.stylize
    var symbol: String { "doc.on.doc" }
    var summary: String { "High-contrast toner copy with streaks, specks and dark edges (zines, flyers)." }
    let params: [ParamDef] = [
        .slider("threshold", "Threshold", 0, 100, 62, step: 1, unit: "%", section: "Toner"),
        .slider("softness", "Softness", 0, 30, 6, step: 0.5, unit: "%", section: "Toner"),
        .slider("contrast", "Contrast", -100, 100, 25, step: 1, section: "Toner"),
        .slider("blur", "Blur", 0, 6, 0.8, step: 0.05, curve: 1.6, section: "Toner"),
        .toggle("dither", "Dither", false, section: "Toner"),
        .sectionSwitch("wearOn", section: "Wear"),
        .slider("noise", "Toner Noise", 0, 100, 35, step: 1, unit: "%", section: "Wear"),
        .slider("uneven", "Uneven Density", 0, 100, 40, step: 1, unit: "%", section: "Wear"),
        .slider("streaks", "Drum Streaks", 0, 100, 35, step: 1, unit: "%", section: "Wear"),
        .slider("edge", "Edge Shadow", 0, 100, 45, step: 1, unit: "%", section: "Wear"),
        .slider("specks", "Background Specks", 0, 100, 30, step: 1, unit: "%", section: "Wear"),
        .slider("generations", "Copy Generations", 1, 6, 1, step: 1, section: "Wear"),
        .color("toner", "Toner", RGBA(hex: "#111111"), section: "Colors"),
        .color("paper", "Paper", RGBA(hex: "#F2F2F2"), section: "Colors"),
        .seed(section: "Colors"),
    ]
    func apply(_ input: MTLTexture, _ p: Params, _ ctx: RenderContext) -> MTLTexture {
        var img = ctx.apply("k_dither_pre", [input], [0, p.f("contrast", 25), 1, 1])
        let wear: Float = p.bool("wearOn", true) ? 1 : 0
        let gens = p.bool("wearOn", true) ? Int(p.num("generations", 1)) : 1
        for g in 0..<gens {
            img = ctx.blur(img, sigma: ctx.px(p.num("blur", 0.8)) * (g == 0 ? 1 : 0.7))
            img = ctx.apply("k_xerox", [img], [p.f("seed", 1) * 1.9 + Float(g) * 11, Float(ctx.scale), p.f("threshold", 62) / 100,
                                               p.f("softness", 6) / 100, p.f("noise", 35) / 100 * wear, p.f("streaks", 35) / 100 * wear,
                                               p.f("edge", 45) / 100 * 0.8 * wear, p.f("uneven", 40) / 100 * wear]
                            + p.color("toner", RGBA(hex: "#111111")).floats + p.color("paper", RGBA(hex: "#F2F2F2")).floats
                            + [p.f("specks", 30) / 100 * wear, p.bool("dither") ? 1 : 0])
        }
        return img
    }
}

struct ThermalPrintEffect: Effect {
    let id = "receipt"
    let name = "Thermal Receipt"
    let category = EffectCategory.stylize
    var symbol: String { "scroll" }
    var summary: String { "Receipt printer: 1-bit dots, faded thermal ink, dead print-head lines." }
    let params: [ParamDef] = [
        .slider("dots", "Dot Size", 0.5, 6, 1.4, step: 0.1, curve: 1.5, section: "Print"),
        .choice("algorithm", "Dither", [("atkinson", "Atkinson"), ("floyd", "Floyd–Steinberg"), ("bayer4", "Ordered")], "atkinson",
                section: "Print"),
        .slider("brightness", "Brightness", -100, 100, 10, step: 1, section: "Print"),
        .slider("contrast", "Contrast", -100, 100, 20, step: 1, section: "Print"),
        .sectionSwitch("wearOn", section: "Wear"),
        .slider("fade", "Fade", 0, 100, 25, step: 1, unit: "%", section: "Wear"),
        .slider("banding", "Banding", 0, 100, 35, step: 1, unit: "%", section: "Wear"),
        .slider("gradient", "Fade Down the Roll", 0, 100, 15, step: 1, unit: "%", section: "Wear"),
        .slider("dead", "Dead Lines", 0, 100, 25, step: 1, unit: "%", section: "Wear"),
        .color("ink", "Ink", RGBA(hex: "#232323"), section: "Colors"),
        .color("paper", "Paper", RGBA(hex: "#F4F4F4"), section: "Colors"),
        .seed(section: "Colors"),
    ]
    func apply(_ input: MTLTexture, _ p: Params, _ ctx: RenderContext) -> MTLTexture {
        var dither = LayerNode.effect("dither").params
        dither["algorithm"] = .choice(p.choice("algorithm", "atkinson"))
        dither["pixelSize"] = .number(p.num("dots", 1.4))
        dither["brightness"] = .number(p.num("brightness", 10))
        dither["contrast"] = .number(p.num("contrast", 20))
        dither["mode"] = .choice("mono")
        dither["levels"] = .number(2)
        dither["dark"] = .color(.black)
        dither["light"] = .color(.white)
        let d = DitherEffect().apply(input, dither, ctx)
        return ctx.apply("k_thermal", [d], [p.f("seed", 1) * 2.3, Float(ctx.scale)] + p.color("ink", RGBA(hex: "#232323")).floats
                         + p.color("paper", RGBA(hex: "#F4F4F4")).floats
                         + (p.bool("wearOn", true) ? [p.f("fade", 25) / 100, p.f("dead", 25) / 100, p.f("banding", 35) / 100, p.f("gradient", 15) / 100]
                            : [0, 0, 0, 0]))
    }
}

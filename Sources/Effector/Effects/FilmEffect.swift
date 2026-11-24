import Foundation
import Metal

struct FilmStock {
    let id: String
    let name: String
    var group = "Color Negative"
    var bw = false
    var saturation: Float = 1
    var contrast: Float = 1.1          // S-curve strength (1 = none)
    var fade: Float = 0.02             // black lift
    var whiteClip: Float = 0.98        // highlight ceiling
    var gamma: SIMD3<Float> = [1, 1, 1] // per-channel gamma (colour cast)
    var shadowTint: SIMD3<Float> = .zero
    var highlightTint: SIMD3<Float> = .zero
    var bwWeights: SIMD3<Float> = [0.3, 0.59, 0.11]
    var grain: Float = 1               // grain multiplier
    var grainSize: Float = 1
    var halation: Float = 0.5          // halation multiplier
    /// Channel mixing (rows = output r, g, b) for false-colour / cross stocks.
    var matrix: [Float] = [1, 0, 0, 0, 1, 0, 0, 0, 1]

    static let groups = ["Color Negative", "Slide", "Cinema", "Instant", "Creative", "Black & White"]

    private static let list: [FilmStock] = [
        // Colour negative
        FilmStock(id: "portra160", name: "Kodak Portra 160", saturation: 0.9, contrast: 1.04, fade: 0.025,
                  gamma: [0.97, 1.0, 1.03], shadowTint: [0.0, 0.004, 0.012], highlightTint: [0.025, 0.012, -0.012], grain: 0.5, halation: 0.4),
        FilmStock(id: "portra400", name: "Kodak Portra 400", saturation: 0.94, contrast: 1.08, fade: 0.03,
                  gamma: [0.96, 0.995, 1.03], shadowTint: [0.0, 0.006, 0.014], highlightTint: [0.03, 0.014, -0.016], grain: 0.8, halation: 0.5),
        FilmStock(id: "portra800", name: "Kodak Portra 800", saturation: 0.98, contrast: 1.1, fade: 0.035,
                  gamma: [0.95, 0.99, 1.04], shadowTint: [0.0, 0.006, 0.016], highlightTint: [0.035, 0.016, -0.02], grain: 1.2, halation: 0.6),
        FilmStock(id: "ektar100", name: "Kodak Ektar 100", saturation: 1.28, contrast: 1.22, fade: 0.008,
                  gamma: [0.98, 1.0, 0.99], shadowTint: [-0.006, 0.0, 0.012], highlightTint: [0.02, 0.006, -0.01], grain: 0.35, halation: 0.3),
        FilmStock(id: "gold200", name: "Kodak Gold 200", saturation: 1.12, contrast: 1.12, fade: 0.03,
                  gamma: [0.94, 0.98, 1.06], shadowTint: [0.008, 0.002, -0.004], highlightTint: [0.045, 0.028, -0.035], grain: 0.9, halation: 0.6),
        FilmStock(id: "ultramax400", name: "Kodak UltraMax 400", saturation: 1.15, contrast: 1.16, fade: 0.03,
                  gamma: [0.95, 0.98, 1.05], shadowTint: [0.006, 0.004, 0.0], highlightTint: [0.04, 0.02, -0.03], grain: 1.0, halation: 0.55),
        FilmStock(id: "colorplus200", name: "Kodak ColorPlus 200", saturation: 1.05, contrast: 1.08, fade: 0.045,
                  gamma: [0.93, 0.98, 1.08], shadowTint: [0.01, 0.004, -0.006], highlightTint: [0.05, 0.03, -0.04], grain: 1.0, halation: 0.6),
        FilmStock(id: "pro400h", name: "Fuji Pro 400H", saturation: 0.88, contrast: 1.0, fade: 0.04,
                  gamma: [1.02, 0.97, 0.98], shadowTint: [-0.01, 0.018, 0.016], highlightTint: [0.012, 0.01, 0.006], grain: 0.8, halation: 0.4),
        FilmStock(id: "superia400", name: "Fuji Superia 400", saturation: 1.08, contrast: 1.15, fade: 0.025,
                  gamma: [1.01, 0.96, 0.99], shadowTint: [-0.012, 0.012, 0.018], highlightTint: [0.01, 0.012, -0.012], grain: 1.0, halation: 0.5),
        FilmStock(id: "c200", name: "Fuji C200", saturation: 1.0, contrast: 1.1, fade: 0.035,
                  gamma: [1.02, 0.97, 0.98], shadowTint: [-0.01, 0.014, 0.012], highlightTint: [0.0, 0.012, 0.0], grain: 0.9, halation: 0.45),
        FilmStock(id: "vista200", name: "Agfa Vista 200", saturation: 1.18, contrast: 1.14, fade: 0.03,
                  gamma: [0.97, 1.0, 1.02], shadowTint: [0.0, 0.0, 0.02], highlightTint: [0.03, -0.005, 0.0], grain: 0.9, halation: 0.5),
        FilmStock(id: "lomo800", name: "Lomography 800", saturation: 1.3, contrast: 1.25, fade: 0.04,
                  gamma: [0.95, 1.0, 1.02], shadowTint: [-0.01, 0.012, 0.03], highlightTint: [0.04, 0.02, -0.02], grain: 1.5, halation: 0.8),
        FilmStock(id: "expired", name: "Expired Drugstore Film", saturation: 0.8, contrast: 0.95, fade: 0.09, whiteClip: 0.94,
                  gamma: [0.92, 1.0, 1.1], shadowTint: [0.03, -0.01, 0.04], highlightTint: [0.05, 0.03, -0.03], grain: 1.6, halation: 0.6),
        // Slide
        FilmStock(id: "velvia50", name: "Fuji Velvia 50", group: "Slide", saturation: 1.5, contrast: 1.35, fade: 0.0, whiteClip: 1.0,
                  gamma: [1.0, 1.01, 0.96], shadowTint: [0.0, -0.004, 0.018], highlightTint: [0.01, 0.0, -0.01], grain: 0.3, halation: 0.25),
        FilmStock(id: "provia100f", name: "Fuji Provia 100F", group: "Slide", saturation: 1.2, contrast: 1.22, fade: 0.005, whiteClip: 1.0,
                  gamma: [1.0, 0.99, 0.98], shadowTint: [0.0, 0.0, 0.012], grain: 0.3, halation: 0.25),
        FilmStock(id: "e100", name: "Kodak Ektachrome E100", group: "Slide", saturation: 1.2, contrast: 1.25, fade: 0.01, whiteClip: 1.0,
                  gamma: [1.0, 0.99, 0.96], shadowTint: [0.0, 0.0, 0.02], highlightTint: [0.0, 0.005, 0.01], grain: 0.3, halation: 0.25),
        FilmStock(id: "kodachrome64", name: "Kodachrome 64", group: "Slide", saturation: 1.25, contrast: 1.3, fade: 0.01,
                  gamma: [0.96, 1.0, 1.04], shadowTint: [0.012, 0.0, -0.01], highlightTint: [0.03, 0.015, -0.02], grain: 0.4, halation: 0.3),
        // Cinema
        FilmStock(id: "cinestill800t", name: "CineStill 800T", group: "Cinema", saturation: 1.02, contrast: 1.12, fade: 0.03,
                  gamma: [1.06, 1.0, 0.92], shadowTint: [-0.01, 0.006, 0.03], highlightTint: [0.02, 0.0, -0.01], grain: 1.1, halation: 1.6),
        FilmStock(id: "cinestill50d", name: "CineStill 50D", group: "Cinema", saturation: 1.05, contrast: 1.1, fade: 0.02,
                  gamma: [0.98, 1.0, 1.01], highlightTint: [0.015, 0.008, -0.005], grain: 0.4, halation: 1.3),
        FilmStock(id: "vision250d", name: "Kodak Vision3 250D", group: "Cinema", saturation: 0.95, contrast: 1.02, fade: 0.04,
                  gamma: [0.98, 1.0, 1.0], shadowTint: [-0.01, 0.008, 0.016], highlightTint: [0.02, 0.01, -0.01], grain: 0.5, halation: 0.6),
        FilmStock(id: "vision500t", name: "Kodak Vision3 500T", group: "Cinema", saturation: 0.95, contrast: 1.06, fade: 0.04,
                  gamma: [1.04, 1.0, 0.95], shadowTint: [-0.012, 0.008, 0.024], highlightTint: [0.015, 0.0, -0.01], grain: 0.8, halation: 1.0),
        FilmStock(id: "bleach", name: "Bleach Bypass", group: "Cinema", saturation: 0.45, contrast: 1.45, fade: 0.02,
                  gamma: [1.0, 0.99, 0.98], grain: 0.7, halation: 0.3),
        // Instant
        FilmStock(id: "polaroid600", name: "Polaroid 600", group: "Instant", saturation: 0.85, contrast: 0.92, fade: 0.07, whiteClip: 0.93,
                  gamma: [0.96, 1.0, 1.06], shadowTint: [0.0, 0.02, 0.03], highlightTint: [0.04, 0.03, -0.02], grain: 0.4, halation: 0.4),
        FilmStock(id: "sx70", name: "Polaroid SX-70", group: "Instant", saturation: 0.75, contrast: 0.9, fade: 0.09, whiteClip: 0.9,
                  gamma: [0.92, 0.98, 1.1], shadowTint: [0.03, 0.0, -0.01], highlightTint: [0.06, 0.04, -0.03], grain: 0.5, halation: 0.5),
        FilmStock(id: "instax", name: "Instax Mini", group: "Instant", saturation: 1.05, contrast: 1.0, fade: 0.05, whiteClip: 0.96,
                  gamma: [1.0, 0.98, 0.97], shadowTint: [-0.01, 0.01, 0.02], highlightTint: [0.01, 0.02, 0.02], grain: 0.3, halation: 0.3),
        // Creative
        FilmStock(id: "crossprocess", name: "Cross Process (E-6 → C-41)", group: "Creative", saturation: 1.35, contrast: 1.35, fade: 0.03,
                  gamma: [1.05, 0.92, 1.25], shadowTint: [-0.02, 0.02, 0.05], highlightTint: [0.05, 0.06, -0.08], grain: 1.0, halation: 0.5),
        FilmStock(id: "lomopurple", name: "LomoChrome Purple", group: "Creative", saturation: 1.15, contrast: 1.15, fade: 0.03,
                  gamma: [0.98, 1.0, 1.0], grain: 1.1, halation: 0.6, matrix: [0.7, 0.35, -0.05, 0.05, 0.25, 0.7, 0.0, 0.6, 0.4]),
        FilmStock(id: "aerochrome", name: "Infrared Aerochrome", group: "Creative", saturation: 1.2, contrast: 1.18, fade: 0.02,
                  grain: 0.9, halation: 0.6, matrix: [0.3, 0.8, -0.1, 0.85, 0.1, 0.05, 0.0, 0.3, 0.7]),
        // Black & white (always exactly neutral)
        FilmStock(id: "trix400", name: "Kodak Tri-X 400", group: "Black & White", bw: true, contrast: 1.38, fade: 0.015,
                  bwWeights: [0.27, 0.6, 0.13], grain: 1.4, grainSize: 1.2, halation: 0.3),
        FilmStock(id: "hp5", name: "Ilford HP5 Plus", group: "Black & White", bw: true, contrast: 1.15, fade: 0.03,
                  bwWeights: [0.3, 0.59, 0.11], grain: 1.2, grainSize: 1.1, halation: 0.3),
        FilmStock(id: "fp4", name: "Ilford FP4 Plus 125", group: "Black & White", bw: true, contrast: 1.2, fade: 0.012,
                  bwWeights: [0.28, 0.6, 0.12], grain: 0.6, grainSize: 0.9, halation: 0.2),
        FilmStock(id: "panf50", name: "Ilford Pan F Plus 50", group: "Black & White", bw: true, contrast: 1.32, fade: 0.0,
                  bwWeights: [0.26, 0.62, 0.12], grain: 0.3, grainSize: 0.75, halation: 0.15),
        FilmStock(id: "xp2", name: "Ilford XP2 Super", group: "Black & White", bw: true, contrast: 1.08, fade: 0.025,
                  bwWeights: [0.3, 0.58, 0.12], grain: 0.35, grainSize: 0.8, halation: 0.25),
        FilmStock(id: "sfx200", name: "Ilford SFX 200 (Red Filter)", group: "Black & White", bw: true, contrast: 1.3, fade: 0.01,
                  bwWeights: [0.62, 0.35, 0.03], grain: 1.0, grainSize: 1.0, halation: 0.3),
        FilmStock(id: "tmax100", name: "Kodak T-Max 100", group: "Black & White", bw: true, contrast: 1.22, fade: 0.008,
                  bwWeights: [0.25, 0.62, 0.13], grain: 0.4, grainSize: 0.8, halation: 0.2),
        FilmStock(id: "tmax3200", name: "Kodak T-Max P3200", group: "Black & White", bw: true, contrast: 1.3, fade: 0.05,
                  bwWeights: [0.28, 0.6, 0.12], grain: 2.0, grainSize: 1.5, halation: 0.4),
        FilmStock(id: "delta3200", name: "Ilford Delta 3200", group: "Black & White", bw: true, contrast: 1.25, fade: 0.06,
                  bwWeights: [0.32, 0.58, 0.1], grain: 2.2, grainSize: 1.6, halation: 0.4),
        FilmStock(id: "acros100", name: "Fuji Acros 100", group: "Black & White", bw: true, contrast: 1.28, fade: 0.0,
                  bwWeights: [0.24, 0.63, 0.13], grain: 0.35, grainSize: 0.8, halation: 0.2),
        FilmStock(id: "fomapan400", name: "Fomapan 400", group: "Black & White", bw: true, contrast: 1.18, fade: 0.04,
                  bwWeights: [0.32, 0.56, 0.12], grain: 1.6, grainSize: 1.3, halation: 0.35),
        FilmStock(id: "retro80s", name: "Rollei Retro 80S", group: "Black & White", bw: true, contrast: 1.4, fade: 0.0,
                  bwWeights: [0.45, 0.5, 0.05], grain: 0.5, grainSize: 0.85, halation: 0.2),
    ]

    /// Stocks ordered by group (the picker shows one section per group).
    static let all: [FilmStock] = groups.flatMap { g in list.filter { $0.group == g } }

    static func by(_ id: String) -> FilmStock { all.first { $0.id == id } ?? all[1] }
}

enum FilmShaders {
    static let source = """
    inline float sCurve(float x, float k) {
        x = clamp(x, 0.0, 1.0);
        float a = pow(x, k), b = pow(1.0 - x, k);
        return a / max(a + b, 1e-5);
    }
    // u: bw, sat, contrast, fade, white, gamma rgb (5..7), shadowTint (8..10), highlightTint (11..13),
    //    weights (14..16), intensity, exposure, warmth
    kernel void k_film(KIN, texture2d<float> a [[texture(1)]]) {
        GUARD
        float4 s = a.read(gid);
        float3 c0 = clamp(s.rgb, 0.0, 1.0);
        float3 c = toSRGB(toLinear(c0) * exp2(u[18]));
        float warm = u[19] / 100.0;
        c = clamp(c * float3(1.0 + warm * 0.06, 1.0, 1.0 - warm * 0.08), 0.0, 1.0);
        if (u[0] > 0.5) {
            float3 w = float3(u[14], u[15], u[16]);
            float g = dot(toLinear(c), w / max(1e-4, w.x + w.y + w.z));
            g = toSRGB(float3(g)).x;
            g = sCurve(g, u[2]);
            g = u[3] + g * (u[4] - u[3]);
            float base = neutralGray(c0);
            g = mix(base, g, u[17]);
            o.write(float4(g, g, g, s.a), gid);   // exactly neutral
            return;
        }
        float3x3 mx = float3x3(float3(u[20], u[23], u[26]), float3(u[21], u[24], u[27]), float3(u[22], u[25], u[28]));
        c = clamp(mx * c, 0.0, 1.0);
        float l = luma(c);
        c = mix(float3(l), c, u[1]);
        c = pow(clamp(c, 0.0, 1.0), float3(u[5], u[6], u[7]));
        c = float3(sCurve(c.r, u[2]), sCurve(c.g, u[2]), sCurve(c.b, u[2]));
        float l2 = luma(c);
        c += float3(u[8], u[9], u[10]) * (1.0 - l2) * (1.0 - l2) * 3.0;
        c += float3(u[11], u[12], u[13]) * l2 * l2 * 2.0;
        c = u[3] + clamp(c, 0.0, 1.0) * (u[4] - u[3]);
        o.write(float4(mix(c0, clamp(c, 0.0, 1.0), u[17]), s.a), gid);
    }
    """
}

struct FilmEffect: Effect {
    let id = "film"
    let name = "Film Emulation"
    let category = EffectCategory.film
    var symbol: String { "film" }
    var summary: String { "Famous film stocks: colour negative, slide, cinema, instant, creative and B&W." }
    var params: [ParamDef] {
        [
            .choice("stock", "Film Stock", options: FilmStock.all.map { ChoiceOption(id: $0.id, label: $0.name, group: $0.group) },
                    "portra400", section: "Film"),
            .slider("intensity", "Intensity", 0, 100, 100, step: 1, unit: "%", section: "Film"),
            .slider("exposure", "Exposure", -2, 2, 0, step: 0.05, unit: " EV", section: "Film"),
            .slider("warmth", "Warmth", -100, 100, 0, step: 1, section: "Film"),
            .slider("fade", "Fade", 0, 100, 0, step: 1, unit: "%", section: "Film"),
            .sectionSwitch("grainOn", section: "Grain"),
            .slider("grain", "Amount", 0, 100, 35, step: 1, unit: "%", section: "Grain"),
            .slider("grainSize", "Size", 0.25, 4, 1, step: 0.05, curve: 1.8, section: "Grain"),
            .seed(section: "Grain"),
            .sectionSwitch("halationOn", section: "Halation"),
            .slider("halation", "Amount", 0, 100, 30, step: 1, unit: "%", section: "Halation"),
        ]
    }

    func apply(_ input: MTLTexture, _ p: Params, _ ctx: RenderContext) -> MTLTexture {
        let st = FilmStock.by(p.choice("stock", "portra400"))
        let intensity = p.f("intensity", 100) / 100
        let fade = st.fade + p.f("fade") / 100 * 0.25
        var u: [Float] = [st.bw ? 1 : 0, st.saturation, st.contrast, fade, st.whiteClip]
        u += [st.gamma.x, st.gamma.y, st.gamma.z, st.shadowTint.x, st.shadowTint.y, st.shadowTint.z,
              st.highlightTint.x, st.highlightTint.y, st.highlightTint.z, st.bwWeights.x, st.bwWeights.y, st.bwWeights.z,
              intensity, p.f("exposure"), st.bw ? 0 : p.f("warmth")]
        u += st.matrix
        // Halation from the original highlights, before tone mapping
        var img = input
        let hal = p.bool("halationOn", true) ? p.f("halation", 30) / 100 * st.halation * intensity : 0
        if hal > 0.001 {
            img = GlowPipeline.run(img, ctx, threshold: 0.68, knee: 0.15, radius: 9, intensity: hal * 1.2,
                                   tint: st.bw ? .white : RGBA(hex: "#FF4418"), colorize: true, saturation: 1, mode: 0,
                                   multiScale: true, neutral: st.bw)
        }
        img = ctx.apply("k_film", [img], u)
        let g = p.bool("grainOn", true) ? p.num("grain", 35) / 100 * Double(st.grain) * Double(intensity) : 0
        if g > 0.001 {
            var gp = Params()
            gp["amount"] = .number(g * 100)
            gp["size"] = .number(p.num("grainSize", 1) * Double(st.grainSize))
            gp["roughness"] = .number(st.bw ? 55 : 40)
            gp["color"] = .bool(!st.bw)
            gp["colorAmount"] = .number(25)
            gp["seed"] = .number(p.num("seed", 1))
            gp["shadows"] = .number(80); gp["midtones"] = .number(100); gp["highlights"] = .number(35)
            img = ctx.apply("k_grain", [img], GrainEffect.uniforms(gp, ctx))
        }
        return img
    }
}

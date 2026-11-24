import Foundation
import Metal

enum GrainShaders {
    static let source = """
    // Film-like grain. u: amount, size(px), roughness, color, shadows, mids, highs, seed, colorAmt
    inline float grainNoise(float2 p, float size, float rough, float seed) {
        float2 q = p / max(size, 0.3);
        // smooth component (2 octaves of value noise, centered)
        float sm = (vnoise(q, seed) - 0.5) * 2.0 + (vnoise(q * 2.03 + 11.0, seed + 1.0) - 0.5) * 1.0;
        sm *= 1.25;
        // rough component: per-cell gaussian
        float rg = gnoise(floor(q), seed + 7.0) * 0.6;
        return mix(sm, rg, rough);
    }
    kernel void k_grain(KIN, texture2d<float> a [[texture(1)]]) {
        GUARD
        float4 s = a.read(gid);
        float3 c = s.rgb;
        float2 p = float2(gid);
        float l = clamp(luma(c), 0.0, 1.0);
        float w = u[4] * (1.0 - l) * (1.0 - l) * 1.6 + u[5] * 4.0 * l * (1.0 - l) + u[6] * l * l * 1.6;
        float amt = u[0] * 0.22 * w;
        if (u[3] > 0.5) {
            float n0 = grainNoise(p, u[1], u[2], u[7]);
            float3 n = float3(grainNoise(p, u[1], u[2], u[7] + 31.0), grainNoise(p, u[1], u[2], u[7] + 57.0), grainNoise(p, u[1], u[2], u[7] + 83.0));
            n = mix(float3(n0), n, u[8]);
            c += n * amt;
        } else {
            c += grainNoise(p, u[1], u[2], u[7]) * amt;
        }
        o.write(float4(clamp(c, 0.0, 1.0), s.a), gid);
    }
    """
}

struct GrainEffect: Effect {
    let id = "grain"
    let name = "Grain"
    let category = EffectCategory.film
    var symbol: String { "aqi.high" }
    var summary: String { "Film-like grain that responds to shadows, midtones and highlights." }
    let params: [ParamDef] = [
        .slider("amount", "Amount", 0, 100, 35, step: 1, unit: "%", curve: 1.4, section: "Grain"),
        .slider("size", "Size", 0.25, 6, 1.0, step: 0.05, unit: "", curve: 2.2, section: "Grain"),
        .slider("roughness", "Roughness", 0, 100, 45, step: 1, unit: "%", section: "Grain"),
        .toggle("color", "Color Grain", false, section: "Grain"),
        .slider("colorAmount", "Color Amount", 0, 100, 50, step: 1, unit: "%", section: "Grain", visible: whenOn("color")),
        .seed(section: "Grain"),
        .slider("shadows", "Shadows", 0, 100, 70, step: 1, unit: "%", section: "Response"),
        .slider("midtones", "Midtones", 0, 100, 100, step: 1, unit: "%", section: "Response"),
        .slider("highlights", "Highlights", 0, 100, 40, step: 1, unit: "%", section: "Response"),
    ]
    func apply(_ input: MTLTexture, _ p: Params, _ ctx: RenderContext) -> MTLTexture {
        ctx.apply("k_grain", [input], GrainEffect.uniforms(p, ctx))
    }

    static func uniforms(_ p: Params, _ ctx: RenderContext, amountScale: Double = 1) -> [Float] {
        [Float(p.num("amount", 35) / 100 * amountScale), Float(max(0.35, ctx.px(p.num("size", 1.0)))), p.f("roughness", 45) / 100,
         p.bool("color") ? 1 : 0, p.f("shadows", 70) / 100, p.f("midtones", 100) / 100, p.f("highlights", 40) / 100,
         p.f("seed", 1) * 13.7, p.f("colorAmount", 50) / 100]
    }
}

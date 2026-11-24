import Foundation
import Metal
import AppKit

enum VHSShaders {
    static let source = """
    inline float3 rgb2yiq(float3 c) {
        return float3(dot(c, float3(0.299, 0.587, 0.114)), dot(c, float3(0.596, -0.274, -0.322)), dot(c, float3(0.211, -0.523, 0.312)));
    }
    inline float3 yiq2rgb(float3 y) {
        return float3(y.x + 0.956 * y.y + 0.621 * y.z, y.x - 0.272 * y.y - 0.647 * y.z, y.x - 1.106 * y.y + 1.703 * y.z);
    }

    // u: 0 seed, 1 unit(px per ref px),
    //    2 tracking on, 3 trackAmt, 4 bandPos, 5 bandH,
    //    6 chroma on, 7 bleed, 8 shift,
    //    9 noise on, 10 noiseAmt,
    //    11 scan on, 12 scanAmt, 13 scanSpacing,
    //    14 wobble on, 15 wobbleAmt, 16 wobbleFreq,
    //    17 soft on, 18 softAmt, 19 desat,
    //    20 dropout on, 21 dropAmt,
    //    22 vignette on, 23 vigAmt
    kernel void k_vhs(KIN, texture2d<float> a [[texture(1)]]) {
        GUARD
        float2 size = OSIZE;
        float2 p = float2(gid) + 0.5;
        float unit = u[1];
        float seed = u[0];
        float y = p.y;
        float lineH = max(1.0, unit * 1.5);
        float line = floor(y / lineH);
        float dx = 0.0;
        float yn = y / size.y;
        // wobble
        if (u[14] > 0.5) {
            dx += sin(yn * u[16] * 6.2831 + seed) * u[15] * unit
                + (vnoise(float2(line * 0.05, seed), seed) - 0.5) * u[15] * unit * 0.8;
        }
        // tracking: jitter + distorted band
        float bandMask = 0.0;
        if (u[2] > 0.5) {
            dx += (hash12(float2(line, seed)) - 0.5) * u[3] * unit * 0.25;
            float bandC = u[4];
            float bandH = max(0.005, u[5]);
            float d = abs(yn - bandC) / bandH;
            bandMask = smoothstep(1.0, 0.0, d);
            dx += bandMask * (vnoise(float2(line * 0.4, seed + 3.0), seed) - 0.3) * u[3] * unit * 3.0;
        }
        float2 uv = (p + float2(dx, 0.0)) / size;
        float3 c;
        if (u[6] > 0.5 || u[17] > 0.5) {
            // horizontal filtering in YIQ: luma soft, chroma bleeds to the right
            float3 yiq = float3(0.0);
            float lumaR = u[17] > 0.5 ? u[18] * unit : 0.0;
            float chromaR = u[6] > 0.5 ? u[7] * unit : 0.0;
            float shift = u[6] > 0.5 ? u[8] * unit : 0.0;
            float wl = 0.0, wc = 0.0;
            float3 accL = float3(0.0); float2 accC = float2(0.0);
            for (int i = -6; i <= 6; i++) {
                float t = float(i) / 6.0;
                float wL = exp(-t * t * 2.0);
                float3 sL = rgb2yiq(a.sample(sLin, uv + float2(t * lumaR, 0.0) / size).rgb);
                accL += sL * wL; wl += wL;
                // chroma: one-sided smear to the right + shift
                float tc = (float(i) + 6.0) / 12.0;
                float wC = exp(-tc * 2.0);
                float3 sC = rgb2yiq(a.sample(sLin, uv + float2(-tc * chromaR - shift, 0.0) / size).rgb);
                accC += sC.yz * wC; wc += wC;
            }
            yiq.x = (accL / wl).x;
            yiq.yz = u[6] > 0.5 ? accC / wc : (accL / wl).yz;
            c = yiq2rgb(yiq);
        } else {
            c = a.sample(sLin, uv).rgb;
        }
        // soften: lift blacks, lower contrast & saturation
        if (u[17] > 0.5) {
            float l = luma(c);
            c = mix(float3(l), c, 1.0 - u[19]);
            c = c * (1.0 - 0.08 * u[18] / 3.0) + 0.04 * u[18] / 3.0;
        }
        // tape noise
        if (u[9] > 0.5) {
            float n = gnoise(float2(floor(p.x / max(1.0, unit)), line), seed + 1.0);
            float lineNoise = (hash12(float2(line, seed + 9.0)) - 0.5);
            c += (n * 0.06 + lineNoise * 0.05) * u[10];
            // occasional bright streaks
            float streak = step(0.995 - u[10] * 0.01, hash12(float2(line * 1.7, seed + 4.0)));
            float sx = hash12(float2(line, seed + 5.0)) * size.x;
            float len = (hash12(float2(line, seed + 6.0)) * 0.3 + 0.05) * size.x;
            if (streak > 0.5 && p.x > sx && p.x < sx + len) c += 0.35 * u[10] * (1.0 - (p.x - sx) / len);
        }
        // tracking band noise
        if (bandMask > 0.0) {
            float n = hash13(float3(floor(p.x / max(1.0, unit * 2.0)), line, seed + 2.0));
            c = mix(c, float3(n), bandMask * 0.35 * min(1.0, u[3] / 10.0));
        }
        // dropouts
        if (u[20] > 0.5) {
            float r = hash12(float2(line, seed + 11.0));
            if (r < 0.02 * u[21]) {
                float sx = hash12(float2(line, seed + 12.0)) * size.x;
                float len = (hash12(float2(line, seed + 13.0)) * 0.06 + 0.01) * size.x;
                if (p.x > sx && p.x < sx + len) c = mix(c, float3(0.95), 0.85);
            }
        }
        // scanlines
        if (u[11] > 0.5) {
            float sp = max(2.0, u[13] * unit);
            float ph = fract(y / sp);
            float s = 0.5 + 0.5 * cos(ph * 6.2831);
            c *= 1.0 - u[12] * s;
        }
        if (u[22] > 0.5) {
            float2 q = p / size - 0.5;
            c *= 1.0 - u[23] * smoothstep(0.25, 0.75, length(q * float2(1.0, 0.9)));
        }
        o.write(float4(clamp(c, 0.0, 1.0), a.read(gid).a), gid);
    }

    // Composite a premultiplied rgba8 overlay over base.
    kernel void k_over_pm(KIN, texture2d<float> base [[texture(1)]], texture2d<float> ov [[texture(2)]]) {
        GUARD
        float4 b = base.read(gid);
        float4 t = ov.sample(sLin, UV);
        o.write(float4(b.rgb * (1.0 - t.a) + t.rgb, b.a), gid);
    }
    """
}

struct VHSEffect: Effect {
    let id = "vhs"
    let name = "VHS"
    let category = EffectCategory.glitch
    var symbol: String { "videoprojector" }
    let params: [ParamDef] = [
        .toggle("tracking", "Tracking Errors", true, section: "Tracking"),
        .slider("trackAmt", "Jitter", 0, 30, 6, section: "Tracking", visible: whenOn("tracking")),
        .slider("bandPos", "Band Position", 0, 100, 82, unit: "%", section: "Tracking", visible: whenOn("tracking")),
        .slider("bandH", "Band Height", 0, 30, 6, unit: "%", section: "Tracking", visible: whenOn("tracking")),
        .toggle("chroma", "Chroma Bleed", true, section: "Color Bleed"),
        .slider("bleed", "Bleed", 0, 30, 8, section: "Color Bleed", visible: whenOn("chroma")),
        .slider("shift", "Shift", -15, 15, 3, section: "Color Bleed", visible: whenOn("chroma")),
        .toggle("noise", "Tape Noise", true, section: "Noise"),
        .slider("noiseAmt", "Amount", 0, 100, 45, unit: "%", section: "Noise", visible: whenOn("noise")),
        .toggle("dropouts", "Dropouts", true, section: "Noise"),
        .slider("dropAmt", "Dropout Amount", 0, 100, 35, unit: "%", section: "Noise", visible: whenOn("dropouts")),
        .toggle("scanlines", "Scanlines", true, section: "Scanlines"),
        .slider("scanAmt", "Intensity", 0, 100, 25, unit: "%", section: "Scanlines", visible: whenOn("scanlines")),
        .slider("scanSpacing", "Spacing", 1, 12, 3, section: "Scanlines", visible: whenOn("scanlines")),
        .toggle("wobble", "Tape Wobble", true, section: "Wobble"),
        .slider("wobbleAmt", "Amount", 0, 20, 2, section: "Wobble", visible: whenOn("wobble")),
        .slider("wobbleFreq", "Frequency", 0.5, 20, 3, section: "Wobble", visible: whenOn("wobble")),
        .toggle("soft", "Soft / Lo-fi", true, section: "Lo-fi"),
        .slider("softAmt", "Softness", 0, 10, 3, section: "Lo-fi", visible: whenOn("soft")),
        .slider("desat", "Desaturate", 0, 100, 20, unit: "%", section: "Lo-fi", visible: whenOn("soft")),
        .toggle("vignette", "Vignette", true, section: "Lo-fi"),
        .slider("vigAmt", "Vignette Amount", 0, 100, 35, unit: "%", section: "Lo-fi", visible: whenOn("vignette")),
        .toggle("osd", "On-Screen Display", true, section: "OSD"),
        ParamDef(key: "osdTop", label: "Top Text", kind: .text(def: "PLAY ▶"), section: "OSD", visible: whenOn("osd")),
        ParamDef(key: "osdBottom", label: "Bottom Text", kind: .text(def: "AM 10:24\nNOV. 24 1997"), section: "OSD", visible: whenOn("osd")),
        .slider("osdSize", "Text Size", 10, 80, 32, section: "OSD", visible: whenOn("osd")),
        .color("osdColor", "Text Color", .white, section: "OSD", visible: whenOn("osd")),
        .seed(section: "OSD"),
    ]

    func apply(_ input: MTLTexture, _ p: Params, _ ctx: RenderContext) -> MTLTexture {
        func on(_ k: String) -> Float { p.bool(k, true) ? 1 : 0 }
        let u: [Float] = [
            p.f("seed", 1) * 7.31, Float(ctx.scale),
            on("tracking"), p.f("trackAmt", 6), p.f("bandPos", 82) / 100, p.f("bandH", 6) / 100,
            on("chroma"), p.f("bleed", 8), p.f("shift", 3),
            on("noise"), p.f("noiseAmt", 45) / 100,
            on("scanlines"), p.f("scanAmt", 25) / 100, p.f("scanSpacing", 3),
            on("wobble"), p.f("wobbleAmt", 2), p.f("wobbleFreq", 3),
            on("soft"), p.f("softAmt", 3), p.f("desat", 20) / 100,
            on("dropouts"), p.f("dropAmt", 35) / 100,
            on("vignette"), p.f("vigAmt", 35) / 100,
        ]
        var out = ctx.apply("k_vhs", [input], u)
        if p.bool("osd", true) {
            let ov = OverlayCanvas(input.width, input.height)
            let size = CGFloat(ctx.px(p.num("osdSize", 32)))
            let margin = CGFloat(min(input.width, input.height)) * 0.06
            let color = p.color("osdColor", .white)
            let top = p.text("osdTop", "PLAY ▶")
            if !top.isEmpty { ov.text(top, at: CGPoint(x: margin, y: margin), size: size, color: color, weight: .bold, shadow: true) }
            let lines = p.text("osdBottom", "AM 10:24\nNOV. 24 1997").split(separator: "\n", omittingEmptySubsequences: false)
            var y = CGFloat(input.height) - margin - CGFloat(lines.count) * size * 1.15
            for l in lines {
                ov.text(String(l), at: CGPoint(x: margin, y: y), size: size, color: color, weight: .bold, shadow: true)
                y += size * 1.15
            }
            out = ctx.apply("k_over_pm", [out, ov.texture()])
        }
        return out
    }
}

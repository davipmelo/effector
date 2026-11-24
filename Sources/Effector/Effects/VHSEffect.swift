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

    // Pass 1 — tape transport: per-line jitter, wobble, tracking band and head-switching skew.
    // u: 0 seed, 1 unit, 2 tracking on, 3 jitter, 4 bandPos, 5 bandH, 6 wobble on, 7 wobbleAmt, 8 wobbleFreq,
    //    9 headSwitch on, 10 headH, 11 lineH
    kernel void k_vhs_geo(KIN, texture2d<float> a [[texture(1)]]) {
        GUARD
        float2 size = OSIZE;
        float2 p = float2(gid) + 0.5;
        float unit = u[1], seed = u[0];
        float lineH = max(1.0, u[11]);
        float line = floor(p.y / lineH);
        float yn = p.y / size.y;
        float dx = 0.0;
        if (u[6] > 0.5) {
            dx += sin(yn * u[8] * 6.2831 + seed) * u[7] * unit
                + (vnoise(float2(line * 0.03, seed), seed) - 0.5) * u[7] * unit * 1.2;
        }
        float band = 0.0;
        if (u[2] > 0.5) {
            dx += (hash12(float2(line, seed)) - 0.5) * u[3] * unit * 0.18;
            float d = abs(yn - u[4]) / max(0.004, u[5]);
            band = smoothstep(1.0, 0.0, d);
            dx += band * (vnoise(float2(line * 0.35, seed + 3.0), seed) - 0.25) * u[3] * unit * 3.5;
        }
        // head switching: the bottom lines are pulled sideways
        float hs = 0.0;
        if (u[9] > 0.5) {
            float h0 = 1.0 - u[10];
            hs = smoothstep(h0, h0 + 0.004, yn);
            float k = (yn - h0) / max(1e-3, u[10]);
            dx += hs * (18.0 + 26.0 * k + (hash12(float2(line, seed + 21.0)) - 0.5) * 14.0) * unit;
        }
        float2 uv = (p + float2(dx, 0.0)) / size;
        float3 c = a.sample(sLin, uv).rgb;
        if (uv.x < 0.0 || uv.x > 1.0) c = float3(0.02);
        o.write(float4(c, 1.0), gid);
    }

    // Pass 2 — signal path + tape noise + tone.
    // u: 0 seed, 1 unit, 2 lumaR, 3 sharpen, 4 chroma on, 5 bleed, 6 shift, 7 chromaNoise, 8 saturation,
    //    9 noise on, 10 noiseAmt, 11 dropouts on, 12 dropAmt, 13 scan on, 14 scanAmt, 15 scanSpacing,
    //    16 vignette on, 17 vigAmt, 18 blackLift, 19 highlight roll-off, 20 warmth, 21 tint,
    //    22 tracking on, 23 bandPos, 24 bandH, 25 jitter, 26 headSwitch on, 27 headH, 28 lineH, 29 contrast
    kernel void k_vhs(KIN, texture2d<float> a [[texture(1)]], texture2d<float> orig [[texture(2)]]) {
        GUARD
        float2 size = OSIZE;
        float2 p = float2(gid) + 0.5;
        float2 uv = p / size;
        float unit = u[1], seed = u[0];
        float lineH = max(1.0, u[28]);
        float line = floor(p.y / lineH);
        float yn = p.y / size.y;

        // --- luma: band-limited + VCR edge enhancement (ringing halos)
        float lumaR = max(0.5, u[2] * unit);
        float accN = 0.0, accW = 0.0, wN = 0.0, wW = 0.0;
        for (int i = -6; i <= 6; i++) {
            float t = float(i) / 6.0;
            float g = exp(-t * t * 2.2);
            accN += rgb2yiq(a.sample(sLin, uv + float2(t * lumaR, 0.0) / size).rgb).x * g; wN += g;
            accW += rgb2yiq(a.sample(sLin, uv + float2(t * lumaR * 3.2, 0.0) / size).rgb).x * g; wW += g;
        }
        float Y = accN / wN;
        float Yw = accW / wW;
        Y += (Y - Yw) * u[3] * 1.6;

        // --- chroma: very low bandwidth, smeared right, delayed, line-averaged vertically
        float2 IQ;
        if (u[4] > 0.5) {
            float chromaR = max(1.0, u[5] * unit);
            float shift = u[6] * unit;
            float2 acc = float2(0.0); float wc = 0.0;
            for (int i = 0; i <= 12; i++) {
                float t = float(i) / 12.0;
                float w = exp(-t * 2.5);
                float2 o1 = float2(-t * chromaR - shift, 0.0) / size;
                float2 o2 = float2(-t * chromaR - shift, -lineH) / size;
                acc += (rgb2yiq(a.sample(sLin, uv + o1).rgb).yz + rgb2yiq(a.sample(sLin, uv + o2).rgb).yz) * 0.5 * w;
                wc += w;
            }
            IQ = acc / wc;
        } else {
            IQ = rgb2yiq(a.sample(sLin, uv).rgb).yz;
        }
        // chroma noise: horizontal coloured streaks
        if (u[7] > 0.0) {
            float sx = floor(p.x / max(1.0, unit * 6.0));
            float n1 = vnoise(float2(sx * 0.35, line), seed + 41.0) - 0.5;
            float n2 = vnoise(float2(sx * 0.35, line), seed + 77.0) - 0.5;
            IQ += float2(n1, n2) * u[7] * 0.22 * (1.0 - smoothstep(0.6, 1.0, Y));
        }
        IQ *= u[8];
        float3 c = yiq2rgb(float3(Y, IQ));

        // --- tape noise: snow elongated along the scan direction
        if (u[9] > 0.5) {
            float n = gnoise(float2(floor(p.x / max(1.0, unit * 2.5)), line), seed + 1.0);
            float ln = hash12(float2(line, seed + 9.0)) - 0.5;
            c += (n * 0.055 + ln * 0.035) * u[10];
            float streak = step(0.993 - u[10] * 0.012, hash12(float2(line * 1.7, seed + 4.0)));
            float sx = hash12(float2(line, seed + 5.0)) * size.x;
            float len = (hash12(float2(line, seed + 6.0)) * 0.25 + 0.04) * size.x;
            if (streak > 0.5 && p.x > sx && p.x < sx + len) c += 0.4 * u[10] * (1.0 - (p.x - sx) / len);
        }
        // tracking band noise
        if (u[22] > 0.5) {
            float d = abs(yn - u[23]) / max(0.004, u[24]);
            float band = smoothstep(1.0, 0.0, d);
            if (band > 0.0) {
                float n = hash13(float3(floor(p.x / max(1.0, unit * 2.0)), line, seed + 2.0));
                float lines = step(0.55, hash12(float2(line, seed + 31.0)));
                c = mix(c, float3(n * 0.9 + 0.1), band * lines * 0.55 * min(1.0, u[25] / 8.0));
            }
        }
        // head-switching noise at the bottom
        if (u[26] > 0.5) {
            float h0 = 1.0 - u[27];
            float hs = smoothstep(h0, h0 + 0.004, yn);
            if (hs > 0.0) {
                float n = hash13(float3(floor(p.x / max(1.0, unit * 3.0)), line, seed + 17.0));
                c = mix(c, float3(n), hs * 0.25);
                c *= 1.0 - hs * 0.25;
            }
        }
        // dropouts: oxide loss = bright horizontal flecks
        if (u[11] > 0.5) {
            float r = hash12(float2(line, seed + 11.0));
            if (r < 0.025 * u[12]) {
                float sx = hash12(float2(line, seed + 12.0)) * size.x;
                float len = (hash12(float2(line, seed + 13.0)) * 0.05 + 0.006) * size.x;
                float t = (p.x - sx) / len;
                if (t > 0.0 && t < 1.0) c = mix(c, float3(0.97), 0.9 * (1.0 - t * t));
            }
        }

        // --- tone: lifted blacks, soft highlight roll-off, colour cast of the tape
        float k = u[29];
        c = (c - 0.5) * k + 0.5;
        float roll = u[19];
        c = mix(c, (1.0 - exp(-max(c, 0.0) * 2.0)) / (1.0 - exp(-2.0)), roll * 0.6);
        c = u[18] + c * (1.0 - u[18]);
        float w = u[20], t = u[21];
        c *= float3(1.0 + 0.07 * w, 1.0 - 0.07 * t * 0.6, 1.0 - 0.09 * w);
        c *= float3(1.0 + 0.05 * t, 1.0 - 0.06 * t, 1.0 + 0.05 * t);

        // --- interlaced scanlines
        if (u[13] > 0.5) {
            float sp = max(2.0, u[15]);
            float ph = fract((p.y - 0.5) / sp);
            float s = smoothstep(0.35, 0.65, abs(ph - 0.5) * 2.0);
            c *= 1.0 - u[14] * s;
        }
        if (u[16] > 0.5) {
            float2 q = uv - 0.5;
            c *= 1.0 - u[17] * smoothstep(0.2, 0.8, length(q * float2(1.0, 0.85)));
        }
        o.write(float4(clamp(c, 0.0, 1.0), orig.read(gid).a), gid);
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
    var summary: String { "Analog tape: soft luma, bleeding chroma, tracking errors, noise and OSD. Try the Looks." }

    let params: [ParamDef] = [
        .sectionSwitch("soft", section: "Signal"),
        .slider("softAmt", "Softness", 0, 10, 3, step: 0.1, section: "Signal"),
        .slider("sharpen", "Edge Enhance", 0, 100, 35, step: 1, unit: "%", section: "Signal"),
        .toggle("chroma", "Chroma Bleed", true, section: "Signal"),
        .slider("bleed", "Bleed", 0, 40, 10, step: 0.5, curve: 1.4, section: "Signal", visible: whenOn("chroma")),
        .slider("shift", "Chroma Delay", -15, 15, 3, step: 0.5, section: "Signal", visible: whenOn("chroma")),
        .slider("chromaNoise", "Chroma Noise", 0, 100, 25, step: 1, unit: "%", section: "Signal"),

        .sectionSwitch("colorOn", section: "Color"),
        .slider("saturation", "Saturation", 0, 150, 85, step: 1, unit: "%", section: "Color"),
        .slider("contrast", "Contrast", 50, 150, 92, step: 1, unit: "%", section: "Color"),
        .slider("blackLift", "Black Lift", 0, 30, 6, step: 0.5, unit: "%", section: "Color"),
        .slider("rolloff", "Highlight Roll-off", 0, 100, 40, step: 1, unit: "%", section: "Color"),
        .slider("warmth", "Warmth", -100, 100, 10, step: 1, section: "Color",
                track: [RGBA(hex: "#3A7BFF"), RGBA(hex: "#9A9A9A"), RGBA(hex: "#FFB020")]),
        .slider("tint", "Tint", -100, 100, 8, step: 1, section: "Color",
                track: [RGBA(hex: "#2FBF5A"), RGBA(hex: "#9A9A9A"), RGBA(hex: "#E040C0")]),

        .toggle("tracking", "Tracking Errors", true, section: "Tracking"),
        .slider("trackAmt", "Jitter", 0, 30, 5, step: 0.5, curve: 1.5, section: "Tracking", visible: whenOn("tracking")),
        .slider("bandPos", "Band Position", 0, 100, 82, step: 1, unit: "%", section: "Tracking", visible: whenOn("tracking")),
        .slider("bandH", "Band Height", 0, 30, 5, step: 0.5, unit: "%", section: "Tracking", visible: whenOn("tracking")),
        .toggle("headSwitch", "Head Switching", true, section: "Tracking"),
        .slider("headH", "Head Switch Height", 0.5, 10, 2.5, step: 0.1, unit: "%", section: "Tracking", visible: whenOn("headSwitch")),

        .toggle("noise", "Tape Noise", true, section: "Noise"),
        .slider("noiseAmt", "Amount", 0, 100, 40, step: 1, unit: "%", section: "Noise", visible: whenOn("noise")),
        .toggle("dropouts", "Dropouts", true, section: "Noise"),
        .slider("dropAmt", "Dropout Amount", 0, 100, 30, step: 1, unit: "%", section: "Noise", visible: whenOn("dropouts")),

        .sectionSwitch("scanlines", section: "Scanlines"),
        .slider("scanAmt", "Intensity", 0, 100, 18, step: 1, unit: "%", section: "Scanlines"),
        .slider("scanSpacing", "Spacing", 1, 12, 3, step: 0.5, section: "Scanlines"),

        .sectionSwitch("wobble", section: "Wobble"),
        .slider("wobbleAmt", "Amount", 0, 20, 1.5, step: 0.1, curve: 1.6, section: "Wobble"),
        .slider("wobbleFreq", "Frequency", 0.5, 20, 3, step: 0.1, section: "Wobble"),

        .sectionSwitch("vignette", section: "Vignette"),
        .slider("vigAmt", "Amount", 0, 100, 30, step: 1, unit: "%", section: "Vignette"),

        .sectionSwitch("osd", section: "OSD"),
        .choice("osdStyle", "OSD Style", [("vcr", "VCR (crisp)"), ("camcorder", "Camcorder (burned in)")], "vcr",
                section: "OSD"),
        ParamDef(key: "osdTop", label: "Top Text", kind: .text(def: "PLAY ▶"), section: "OSD"),
        ParamDef(key: "osdBottom", label: "Bottom Text", kind: .text(def: "AM 10:24\nNOV. 24 1997"), section: "OSD"),
        .slider("osdSize", "Text Size", 10, 80, 30, step: 1, section: "OSD"),
        .color("osdColor", "Text Color", .white, section: "OSD"),
        .seed(section: "OSD"),
    ]

    var looks: [EffectLook] {
        [
            EffectLook(id: "clean", name: "Clean SP Tape", group: "VHS", values: [
                "softAmt": 2.0, "sharpen": 25.0, "bleed": 6.0, "shift": 2.0, "chromaNoise": 10.0, "saturation": 92.0,
                "blackLift": 4.0, "tracking": false, "headSwitch": true, "headH": 1.5, "noiseAmt": 15.0, "dropAmt": 8.0,
                "scanAmt": 12.0, "wobbleAmt": 0.6, "vigAmt": 20.0, "osd": false]),
            EffectLook(id: "home86", name: "Home Video '86", group: "VHS", values: [
                "softAmt": 4.0, "sharpen": 30.0, "bleed": 16.0, "shift": 4.0, "chromaNoise": 35.0, "saturation": 80.0,
                "contrast": 88.0, "blackLift": 9.0, "rolloff": 60.0, "warmth": 30.0, "tint": 12.0, "trackAmt": 4.0,
                "bandH": 4.0, "noiseAmt": 45.0, "dropAmt": 35.0, "wobbleAmt": 2.5, "vigAmt": 40.0,
                "osdTop": ParamValue.text("PLAY ▶"), "osdBottom": ParamValue.text("JUL. 4 1986\n6:42 PM")]),
            EffectLook(id: "music89", name: "Music Video '89", group: "VHS", values: [
                "softAmt": 2.5, "sharpen": 55.0, "bleed": 12.0, "shift": 5.0, "chromaNoise": 20.0, "saturation": 135.0,
                "contrast": 110.0, "blackLift": 3.0, "warmth": -5.0, "tint": 30.0, "tracking": false, "noiseAmt": 25.0,
                "dropAmt": 15.0, "scanAmt": 22.0, "wobbleAmt": 1.0, "vigAmt": 30.0, "osd": false]),
            EffectLook(id: "rental94", name: "Rental Tape '94", group: "VHS", values: [
                "softAmt": 3.5, "sharpen": 35.0, "bleed": 12.0, "shift": 3.0, "chromaNoise": 30.0, "saturation": 78.0,
                "blackLift": 7.0, "warmth": 5.0, "tint": 6.0, "trackAmt": 9.0, "bandPos": 86.0, "bandH": 7.0,
                "noiseAmt": 55.0, "dropAmt": 55.0, "wobbleAmt": 1.8, "vigAmt": 30.0,
                "osdTop": ParamValue.text("PLAY ▶"), "osdBottom": ParamValue.text("SP  0:47:12")]),
            EffectLook(id: "dubbed", name: "Dubbed 3rd Generation", group: "VHS", values: [
                "softAmt": 6.5, "sharpen": 60.0, "bleed": 26.0, "shift": 7.0, "chromaNoise": 60.0, "saturation": 70.0,
                "contrast": 85.0, "blackLift": 12.0, "rolloff": 70.0, "warmth": 15.0, "tint": 25.0, "trackAmt": 7.0,
                "noiseAmt": 65.0, "dropAmt": 45.0, "wobbleAmt": 3.0, "vigAmt": 45.0, "osd": false]),
            EffectLook(id: "latenight", name: "Late Night TV Rip", group: "VHS", values: [
                "softAmt": 3.0, "sharpen": 40.0, "bleed": 10.0, "chromaNoise": 40.0, "saturation": 88.0, "blackLift": 5.0,
                "warmth": -15.0, "tint": -18.0, "tracking": false, "noiseAmt": 70.0, "dropAmt": 20.0, "scanAmt": 35.0,
                "scanSpacing": 2.0, "wobbleAmt": 1.0, "vigAmt": 35.0, "osd": true, "osdTop": ParamValue.text("CH 03"),
                "osdBottom": ParamValue.text("")]),
            EffectLook(id: "eaten", name: "Eaten Tape", group: "VHS", values: [
                "softAmt": 5.0, "sharpen": 50.0, "bleed": 22.0, "shift": 9.0, "chromaNoise": 70.0, "saturation": 75.0,
                "blackLift": 10.0, "trackAmt": 22.0, "bandPos": 55.0, "bandH": 16.0, "headH": 6.0, "noiseAmt": 90.0,
                "dropAmt": 100.0, "wobbleAmt": 7.0, "wobbleFreq": 6.0, "vigAmt": 40.0, "osd": false]),
            EffectLook(id: "camcorder98", name: "Camcorder '98", group: "Camcorder", values: [
                "softAmt": 2.0, "sharpen": 75.0, "bleed": 9.0, "shift": 2.0, "chromaNoise": 20.0, "saturation": 95.0,
                "contrast": 98.0, "blackLift": 5.0, "warmth": -8.0, "tint": 4.0, "tracking": false, "headH": 1.8,
                "noiseAmt": 30.0, "dropAmt": 10.0, "scanAmt": 10.0, "wobbleAmt": 0.8, "vigAmt": 25.0,
                "osdStyle": "camcorder", "osdTop": ParamValue.text("● REC"), "osdBottom": ParamValue.text("AUG 14 1998\n10:24:07 PM"),
                "osdSize": 26.0]),
            EffectLook(id: "hi8", name: "Hi8 Vacation", group: "Camcorder", values: [
                "softAmt": 1.6, "sharpen": 60.0, "bleed": 7.0, "chromaNoise": 15.0, "saturation": 110.0, "contrast": 104.0,
                "blackLift": 3.0, "warmth": 25.0, "tint": 0.0, "tracking": false, "headSwitch": false, "noiseAmt": 22.0,
                "dropAmt": 5.0, "scanAmt": 8.0, "wobbleAmt": 0.5, "vigAmt": 20.0, "osdStyle": "camcorder",
                "osdTop": ParamValue.text(""), "osdBottom": ParamValue.text("12. 7. 1996")]),
            EffectLook(id: "minidv", name: "MiniDV 2003", group: "Digital Video", values: [
                "softAmt": 0.8, "sharpen": 70.0, "chroma": true, "bleed": 3.0, "shift": 1.0, "chromaNoise": 8.0,
                "saturation": 100.0, "contrast": 104.0, "blackLift": 2.0, "rolloff": 15.0, "warmth": -20.0, "tint": -5.0,
                "tracking": false, "headSwitch": false, "noise": true, "noiseAmt": 12.0, "dropouts": false,
                "scanAmt": 14.0, "scanSpacing": 2.0, "wobble": false, "vigAmt": 10.0, "osdStyle": "camcorder",
                "osdTop": ParamValue.text("SP  ▶"), "osdBottom": ParamValue.text("2003.07.12  16:20")]),
            EffectLook(id: "cctv", name: "CCTV Recorder", group: "Digital Video", values: [
                "softAmt": 3.0, "sharpen": 45.0, "chroma": false, "chromaNoise": 0.0, "saturation": 0.0, "contrast": 112.0,
                "blackLift": 6.0, "warmth": 0.0, "tint": 0.0, "trackAmt": 3.0, "bandH": 3.0, "noiseAmt": 55.0, "dropAmt": 15.0,
                "scanAmt": 25.0, "scanSpacing": 2.0, "wobbleAmt": 0.4, "vigAmt": 45.0,
                "osdTop": ParamValue.text("CAM 04"), "osdBottom": ParamValue.text("11-24-1999  03:14:07")]),
        ]
    }

    func apply(_ input: MTLTexture, _ p: Params, _ ctx: RenderContext) -> MTLTexture {
        func on(_ k: String, _ d: Bool = true) -> Float { p.bool(k, d) ? 1 : 0 }
        let seed = p.f("seed", 1) * 7.31
        let unit = Float(ctx.scale)
        let lineH = Float(max(1, (ctx.px(1.5)).rounded()))
        let osdOn = p.bool("osd", true)
        let burnIn = p.choice("osdStyle", "vcr") == "camcorder"

        var src = input
        // Camcorder date stamps are recorded on tape: draw them before the degradation.
        if osdOn && burnIn { src = ctx.apply("k_over_pm", [src, osd(p, ctx, input.width, input.height, camcorder: true).texture()]) }

        let geo = ctx.apply("k_vhs_geo", [src], [
            seed, unit, on("tracking"), p.f("trackAmt", 5), p.f("bandPos", 82) / 100, p.f("bandH", 5) / 100,
            on("wobble"), p.f("wobbleAmt", 1.5), p.f("wobbleFreq", 3),
            on("headSwitch"), p.f("headH", 2.5) / 100, lineH,
        ])
        let softOn = p.bool("soft", true)
        let colorOn = p.bool("colorOn", true)
        let u: [Float] = [
            seed, unit, softOn ? p.f("softAmt", 3) : 0.3, softOn ? p.f("sharpen", 35) / 100 : 0,
            softOn ? on("chroma") : 0, p.f("bleed", 10), p.f("shift", 3), softOn ? p.f("chromaNoise", 25) / 100 : 0,
            colorOn ? p.f("saturation", 85) / 100 * (1 - p.f("desat", 0) / 100) : 1,
            on("noise"), p.f("noiseAmt", 40) / 100, on("dropouts"), p.f("dropAmt", 30) / 100,
            on("scanlines"), p.f("scanAmt", 18) / 100, Float(max(2, ctx.px(p.num("scanSpacing", 3)).rounded())),
            on("vignette"), p.f("vigAmt", 30) / 100, colorOn ? p.f("blackLift", 6) / 100 : 0, colorOn ? p.f("rolloff", 40) / 100 : 0,
            colorOn ? p.f("warmth", 10) / 100 : 0, colorOn ? p.f("tint", 8) / 100 : 0,
            on("tracking"), p.f("bandPos", 82) / 100, p.f("bandH", 5) / 100, p.f("trackAmt", 5),
            on("headSwitch"), p.f("headH", 2.5) / 100, lineH, colorOn ? p.f("contrast", 92) / 100 : 1,
        ]
        var out = ctx.apply("k_vhs", [geo, input], u)
        if osdOn && !burnIn {
            out = ctx.apply("k_over_pm", [out, osd(p, ctx, input.width, input.height, camcorder: false).texture()])
        }
        return out
    }

    /// VCR-style on-screen text: chunky, slightly stretched monospace with an outline.
    private func osd(_ p: Params, _ ctx: RenderContext, _ w: Int, _ h: Int, camcorder: Bool) -> OverlayCanvas {
        let ov = OverlayCanvas(w, h)
        let size = CGFloat(ctx.px(p.num("osdSize", 30)))
        let margin = CGFloat(min(w, h)) * 0.07
        let color = p.color("osdColor", .white)
        let outline: RGBA? = camcorder ? RGBA(r: 0.05, g: 0.05, b: 0.05) : nil
        func draw(_ s: String, _ pt: CGPoint) {
            ov.text(s, at: pt, size: size, color: color, weight: .heavy, shadow: !camcorder,
                    stretch: 1.18, outline: outline, kern: 0.04)
        }
        let top = p.text("osdTop", "PLAY ▶")
        if !top.isEmpty { draw(top, CGPoint(x: margin, y: margin)) }
        let lines = p.text("osdBottom", "AM 10:24\nNOV. 24 1997").split(separator: "\n", omittingEmptySubsequences: false)
        let lh = size * 1.2
        var y = CGFloat(h) - margin - CGFloat(lines.count) * lh
        for l in lines where !l.isEmpty || lines.count > 1 {
            if camcorder {
                // camcorders print the date bottom-right
                let est = CGFloat(l.count) * size * 0.62 * 1.18
                draw(String(l), CGPoint(x: CGFloat(w) - margin - est, y: y))
            } else {
                draw(String(l), CGPoint(x: margin, y: y))
            }
            y += lh
        }
        return ov
    }
}

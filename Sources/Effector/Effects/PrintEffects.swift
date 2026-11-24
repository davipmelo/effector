import Foundation
import Metal

enum HalftoneShaders {
    static let source = """
    inline float2 rot2(float2 p, float a) { float c = cos(a), s = sin(a); return float2(c * p.x - s * p.y, s * p.x + c * p.y); }

    // coverage v in [0,1] → ink (0/1, antialiased) for local cell coords d in [-0.5,0.5]
    inline float dotInk(float2 d, float v, int shape, float aa) {
        v = clamp(v, 0.0, 1.0);
        float m, t;
        switch (shape) {
            case 1: { // square
                m = max(abs(d.x), abs(d.y)) * 2.0; t = sqrt(v); break; }
            case 2: { // diamond
                m = abs(d.x) + abs(d.y);
                t = v <= 0.5 ? sqrt(v / 2.0) : 1.0 - sqrt((1.0 - v) / 2.0); break; }
            case 3: { // line
                m = abs(d.y) * 2.0; t = v; break; }
            case 4: { // ellipse
                m = length(d * float2(0.78, 1.28)) * 2.0;
                t = 2.0 * sqrt(v / 3.14159); t = v > 0.7 ? mix(t, 1.6, (v - 0.7) / 0.3) : t; break; }
            case 5: { // cross
                m = min(abs(d.x), abs(d.y)) * 2.0;
                t = 1.0 - sqrt(max(0.0, 1.0 - v)); break; }
            default: { // circle
                m = length(d) * 2.0;
                t = 2.0 * sqrt(v / 3.14159); t = v > 0.785 ? mix(t, 1.42, (v - 0.785) / 0.215) : t; break; }
        }
        if (v <= 0.001) return 0.0;
        if (v >= 0.999) return 1.0;
        return 1.0 - smoothstep(t - aa, t + aa, m);
    }

    // Sample one screen at angle; returns ink amount for channel ch of a coverage function
    // u: cell(px), angle(deg), shape, mode(0 bw,1 cmyk,2 color), dotScale, softness,
    //    paper rgb (6..8), ink rgb (9..11), angC, angM, angY, angK (12..15), gcr (16)
    inline float4 cmykOf(float3 c, float gcr) {
        float k = 1.0 - max(c.r, max(c.g, c.b));
        k *= gcr;
        float d = max(1e-4, 1.0 - k);
        return float4((1.0 - c.r - k) / d, (1.0 - c.g - k) / d, (1.0 - c.b - k) / d, k);
    }

    kernel void k_halftone(KIN, texture2d<float> src [[texture(1)]]) {
        GUARD
        float2 p = float2(gid) + 0.5;
        float2 size = OSIZE;
        float cs = max(2.0, u[0]);
        int shape = int(u[2]);
        int mode = int(u[3]);
        float scale = u[4];
        float aa = max(u[5], 1.2 / cs);
        float3 paper = float3(u[6], u[7], u[8]);
        float3 ink = float3(u[9], u[10], u[11]);
        float a0 = src.read(gid).a;

        if (mode == 1) {
            float angs[4] = { u[12], u[13], u[14], u[15] };
            float3 inks[4] = { float3(0.0, 0.62, 0.88), float3(0.90, 0.05, 0.52), float3(1.0, 0.93, 0.0), float3(0.08, 0.08, 0.09) };
            float3 col = paper;
            for (int i = 0; i < 4; i++) {
                float a = angs[i] * 0.0174533;
                float2 q = rot2(p, -a);
                float2 cell = floor(q / cs);
                float2 center = (cell + 0.5) * cs;
                float2 uv = rot2(center, a) / size;
                float3 c = src.sample(sLin, uv).rgb;
                float v = cmykOf(c, u[16])[i] * scale;
                float amt = dotInk((q - center) / cs, v, shape, aa);
                col *= mix(float3(1.0), inks[i], amt);
            }
            o.write(float4(col, a0), gid);
            return;
        }
        float a = u[1] * 0.0174533;
        float2 q = rot2(p, -a);
        float2 cell = floor(q / cs);
        float2 center = (cell + 0.5) * cs;
        float2 uv = rot2(center, a) / size;
        float3 c = src.sample(sLin, uv).rgb;
        float v = (1.0 - neutralGray(c)) * scale;
        float amt = dotInk((q - center) / cs, v, shape, aa);
        float3 dotColor = mode == 2 ? c : ink;
        if (mode == 2) {
            // colour dots: dot size from darkness of a desaturated copy, colour from image
            v = (1.0 - neutralGray(c) * 0.85) * scale;
            amt = dotInk((q - center) / cs, v, shape, aa);
        }
        o.write(float4(mix(paper, dotColor, amt), a0), gid);
    }
    """
}

struct HalftoneEffect: Effect {
    let id = "halftone"
    let name = "Halftone"
    let category = EffectCategory.stylize
    var symbol: String { "circle.hexagongrid" }
    let params: [ParamDef] = [
        .segmented("mode", "Mode", [("bw", "B&W"), ("cmyk", "CMYK"), ("color", "Color")], "bw", section: "Screen"),
        .choice("shape", "Dot Shape", [("circle", "Circle"), ("square", "Square"), ("diamond", "Diamond"),
                                       ("line", "Line"), ("ellipse", "Ellipse"), ("cross", "Cross")], "circle", section: "Screen"),
        .slider("cell", "Cell Size", 1, 80, 8, step: 0.25, unit: "", section: "Screen"),
        .slider("angle", "Angle", 0, 180, 45, unit: "°", section: "Screen", visible: whenNot("mode", "cmyk")),
        .slider("dotScale", "Dot Size", 20, 200, 100, unit: "%", section: "Screen"),
        .slider("softness", "Softness", 0, 100, 10, unit: "%", section: "Screen"),
        .slider("smooth", "Detail Smoothing", 0, 100, 40, unit: "%", section: "Screen"),
        .slider("angC", "Cyan Angle", 0, 180, 15, unit: "°", section: "CMYK Angles", visible: when("mode", "cmyk")),
        .slider("angM", "Magenta Angle", 0, 180, 75, unit: "°", section: "CMYK Angles", visible: when("mode", "cmyk")),
        .slider("angY", "Yellow Angle", 0, 180, 0, unit: "°", section: "CMYK Angles", visible: when("mode", "cmyk")),
        .slider("angK", "Black Angle", 0, 180, 45, unit: "°", section: "CMYK Angles", visible: when("mode", "cmyk")),
        .slider("gcr", "Black Generation", 0, 100, 80, unit: "%", section: "CMYK Angles", visible: when("mode", "cmyk")),
        .color("paper", "Paper", RGBA(hex: "#F4F1EA"), section: "Colors"),
        .color("ink", "Ink", RGBA(hex: "#111111"), section: "Colors", visible: when("mode", "bw")),
        .slider("contrast", "Contrast", -100, 100, 0, section: "Tone"),
        .slider("brightness", "Brightness", -100, 100, 0, section: "Tone"),
    ]

    func apply(_ input: MTLTexture, _ p: Params, _ ctx: RenderContext) -> MTLTexture {
        let cell = max(2, ctx.px(p.num("cell", 8)))
        let toned = ctx.apply("k_dither_pre", [input], [p.f("brightness"), p.f("contrast"), 0, 1])
        let smoothed = ctx.blur(toned, sigma: cell * 0.5 * p.num("smooth", 40) / 100)
        let modeIdx: Float = ["bw": 0, "cmyk": 1, "color": 2][p.choice("mode", "bw")] ?? 0
        let shapeIdx: Float = ["circle": 0, "square": 1, "diamond": 2, "line": 3, "ellipse": 4, "cross": 5][p.choice("shape", "circle")] ?? 0
        let u: [Float] = [Float(cell), p.f("angle", 45), shapeIdx, modeIdx, p.f("dotScale", 100) / 100, p.f("softness", 10) / 100 * 0.5]
            + p.color("paper", RGBA(hex: "#F4F1EA")).floats + p.color("ink", RGBA(hex: "#111111")).floats
            + [p.f("angC", 15), p.f("angM", 75), p.f("angY", 0), p.f("angK", 45), p.f("gcr", 80) / 100]
        return ctx.apply("k_halftone", [smoothed], u)
    }
}

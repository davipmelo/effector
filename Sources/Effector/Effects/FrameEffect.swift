import Foundation
import Metal
import CoreGraphics

enum FrameShaders {
    static let source = """
    // Frame. u: 0..3 inner rect (px), 4 radius(px), 5..7 frame rgb, 8 texture, 9 inner shadow, 10 seed, 11 unit,
    //    12..15 source rect (uv of input used to fill the window)
    kernel void k_frame(KIN, texture2d<float> a [[texture(1)]]) {
        GUARD
        float2 p = float2(gid) + 0.5;
        float2 r0 = float2(u[0], u[1]), rs = float2(u[2], u[3]);
        float rad = u[4];
        float2 c = r0 + rs * 0.5;
        float2 q = abs(p - c) - (rs * 0.5 - rad);
        float sd = length(max(q, 0.0)) + min(max(q.x, q.y), 0.0) - rad;   // signed distance to the window
        float inside = 1.0 - smoothstep(-0.75, 0.75, sd);
        float unit = max(u[11], 1e-3);
        float3 frame = float3(u[5], u[6], u[7]);
        float2 tq = p / unit;
        frame *= 1.0 + ((fbm(tq / 1.3, u[10], 3) - 0.5) * 0.08 + (fbm(tq / 90.0, u[10] + 4.0, 3) - 0.5) * 0.05) * u[8];
        float2 luv = (p - r0) / rs;
        float2 suv = float2(u[12], u[13]) + luv * float2(u[14], u[15]);
        float3 img = a.sample(sLin, suv).rgb;
        // the print sits slightly below the frame: soft shadow inside the window edge
        float sh = exp(-max(-sd, 0.0) / (unit * 4.0)) * u[9] * 0.45;
        img *= 1.0 - sh;
        o.write(float4(mix(frame, img, inside), a.read(gid).a), gid);
    }
    """
}

struct FrameEffect: Effect {
    let id = "frame"
    let name = "Frame & Letterbox"
    let category = EffectCategory.overlay
    var symbol: String { "photo.artframe" }
    var summary: String { "Cinema black bars (letterbox) in classic aspect ratios, Polaroid, a simple border or a rounded print." }

    static let ratios: [(String, String, Double)] = [
        ("1.33", "1.33:1 (Academy 4:3)", 4.0 / 3), ("1.66", "1.66:1 (European)", 1.66), ("1.78", "1.78:1 (16:9)", 16.0 / 9),
        ("1.85", "1.85:1 (Flat)", 1.85), ("2.00", "2.00:1 (Univisium)", 2.0), ("2.20", "2.20:1 (70 mm)", 2.2),
        ("2.39", "2.39:1 (Scope)", 2.39), ("2.76", "2.76:1 (Ultra Panavision)", 2.76), ("3.00", "3.00:1 (Ultra-wide)", 3.0),
    ]

    let params: [ParamDef] = [
        .choice("style", "Style", [("letterbox", "Black Bars (Letterbox)"), ("polaroid", "Polaroid"), ("border", "Simple Border"),
                                   ("rounded", "Rounded Print")], "letterbox", section: "Frame"),
        .choice("ratio", "Aspect Ratio", options: FrameEffect.ratios.map { ChoiceOption(id: $0.0, label: $0.1) }, "2.39",
                section: "Frame", visible: when("style", "letterbox")),
        .slider("offset", "Reframe", -100, 100, 0, step: 1, unit: "%", section: "Frame", visible: when("style", "letterbox")),
        .slider("size", "Border Size", 30, 250, 100, step: 1, unit: "%", section: "Frame", visible: whenNot("style", "letterbox")),
        .slider("radius", "Corner Radius", 0, 60, 0, step: 0.5, section: "Frame", visible: whenAny("style", ["border", "rounded"])),
        .color("color", "Frame Color", RGBA(hex: "#F5F4F0"), section: "Frame", visible: whenNot("style", "letterbox")),
        .color("barColor", "Bar Color", .black, section: "Frame", visible: when("style", "letterbox")),
        .slider("texture", "Texture", 0, 100, 50, step: 1, unit: "%", section: "Frame", visible: whenNot("style", "letterbox")),
        .slider("shadow", "Inner Shadow", 0, 100, 35, step: 1, unit: "%", section: "Frame", visible: whenNot("style", "letterbox")),
        .sectionSwitch("textOn", section: "Text", false),
        ParamDef(key: "caption", label: "Caption / Subtitle", kind: .text(def: "I've seen things you people wouldn't believe."), section: "Text"),
        .color("textColor", "Text Color", RGBA(hex: "#F2E36B"), section: "Text"),
        .slider("textSize", "Text Size", 8, 80, 26, step: 1, section: "Text"),
        .segmented("textPlace", "Placement", [("bar", "In the Bar"), ("image", "On the Image")], "bar", section: "Text",
                   visible: when("style", "letterbox")),
        .seed(section: "Text"),
    ]

    var looks: [EffectLook] {
        [
            EffectLook(id: "scope", name: "Scope 2.39:1", group: "Letterbox", values: ["style": "letterbox", "ratio": "2.39"]),
            EffectLook(id: "flat", name: "Flat 1.85:1", group: "Letterbox", values: ["style": "letterbox", "ratio": "1.85"]),
            EffectLook(id: "ultra", name: "Ultra Panavision 2.76:1", group: "Letterbox", values: ["style": "letterbox", "ratio": "2.76"]),
            EffectLook(id: "subtitle", name: "Movie Still with Subtitle", group: "Letterbox",
                       values: ["style": "letterbox", "ratio": "2.39", "textOn": true]),
            EffectLook(id: "academy", name: "Academy 4:3", group: "Letterbox", values: ["style": "letterbox", "ratio": "1.33"]),
            EffectLook(id: "polaroid", name: "Polaroid", group: "Print", values: ["style": "polaroid"]),
            EffectLook(id: "white", name: "White Border", group: "Print", values: ["style": "border", "color": RGBA(hex: "#F7F7F5")]),
            EffectLook(id: "black", name: "Black Border", group: "Print", values: ["style": "border", "color": RGBA(hex: "#0B0B0B"), "texture": 0.0]),
            EffectLook(id: "rounded", name: "Rounded Print", group: "Print", values: ["style": "rounded"]),
        ]
    }

    func apply(_ input: MTLTexture, _ p: Params, _ ctx: RenderContext) -> MTLTexture {
        let W = CGFloat(input.width), H = CGFloat(input.height)
        let m = min(W, H)
        let k = CGFloat(p.num("size", 100) / 100)
        let style = p.choice("style", "letterbox")
        var frameColor = p.color("color", RGBA(hex: "#F5F4F0"))
        var rect: CGRect
        var radius: CGFloat = CGFloat(ctx.px(p.num("radius", 0)))
        var texture = p.f("texture", 50) / 100
        var shadow = p.f("shadow", 35) / 100
        var src = CGRect(x: 0, y: 0, width: 1, height: 1)
        switch style {
        case "letterbox":
            frameColor = p.color("barColor", .black)
            texture = 0; shadow = 0; radius = 0
            let a = CGFloat(FrameEffect.ratios.first { $0.0 == p.choice("ratio", "2.39") }?.2 ?? 2.39)
            let canvas = W / H
            if a >= canvas {
                let h = (W / a).rounded()
                rect = CGRect(x: 0, y: ((H - h) / 2).rounded(), width: W, height: h)
            } else {
                let w = (H * a).rounded()
                rect = CGRect(x: ((W - w) / 2).rounded(), y: 0, width: w, height: H)
            }
            // the bars cover the image (same scale); Reframe slides the visible part
            let slackY = 1 - rect.height / H, slackX = 1 - rect.width / W
            let o = CGFloat(p.num("offset") / 100)
            src = CGRect(x: slackX / 2 * (1 + o), y: slackY / 2 * (1 + o), width: rect.width / W, height: rect.height / H)
        case "border", "rounded":
            let s = m * 0.05 * k
            rect = CGRect(x: s, y: s, width: W - 2 * s, height: H - 2 * s).integral
            if style == "rounded" && radius == 0 { radius = m * 0.02 }
        default: // polaroid
            let s = m * 0.055 * k, b = m * 0.22 * k
            rect = CGRect(x: s, y: s, width: W - 2 * s, height: H - s - b).integral
            radius = m * 0.002
        }
        if style != "letterbox" {
            // cover-fit the image into the window
            let ra = rect.width / max(1, rect.height), ia = W / H
            if ia > ra { let w = ra / ia; src = CGRect(x: (1 - w) / 2, y: 0, width: w, height: 1) }
            else { let h = ia / ra; src = CGRect(x: 0, y: (1 - h) / 2, width: 1, height: h) }
        }
        var out = ctx.apply("k_frame", [input], [Float(rect.minX), Float(rect.minY), Float(rect.width), Float(rect.height),
                                                  Float(radius)] + frameColor.floats + [texture, shadow, p.f("seed", 1) * 3.3,
                                                  Float(ctx.scale), Float(src.minX), Float(src.minY), Float(src.width), Float(src.height)])
        let caption = p.text("caption", "")
        if p.bool("textOn", false) && !caption.isEmpty {
            let ov = OverlayCanvas(input.width, input.height)
            let fs = CGFloat(ctx.px(p.num("textSize", 26)))
            let color = p.color("textColor", RGBA(hex: "#F2E36B"))
            switch style {
            case "letterbox":
                let inBar = p.choice("textPlace", "bar") == "bar" && rect.minY > fs * 1.6
                let y = inBar ? rect.maxY + (H - rect.maxY - fs) / 2 : rect.maxY - fs * 2.2
                let est = CGFloat(caption.count) * fs * 0.5
                ov.text(caption, at: CGPoint(x: (W - est) / 2, y: y), size: fs, color: color, weight: .medium, mono: false,
                        shadow: !inBar)
            case "polaroid":
                let bottom = H - rect.maxY
                ov.text(caption, at: CGPoint(x: rect.minX + rect.width * 0.06, y: rect.maxY + (bottom - fs) * 0.45), size: fs,
                        color: color, weight: .regular, mono: false, fontName: "Noteworthy-Bold")
            default:
                ov.text(caption, at: CGPoint(x: rect.minX, y: rect.maxY + max(2, (H - rect.maxY - fs) / 2)), size: fs,
                        color: color, weight: .medium, mono: false)
            }
            out = ctx.apply("k_over_pm", [out, ov.texture()])
        }
        return out
    }
}

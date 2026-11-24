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
    let name = "Frame & Border"
    let category = EffectCategory.overlay
    var symbol: String { "photo.artframe" }
    var summary: String { "Polaroid, Instax, 35 mm film strip, slide mount or a simple border around the photo." }
    let params: [ParamDef] = [
        .choice("style", "Style", [("polaroid", "Polaroid"), ("instax", "Instax Mini"), ("film", "35mm Film Strip"),
                                   ("slide", "Slide Mount"), ("border", "Simple Border"), ("rounded", "Rounded Print")],
                "polaroid", section: "Frame"),
        .slider("size", "Border Size", 30, 250, 100, step: 1, unit: "%", section: "Frame"),
        .slider("radius", "Corner Radius", 0, 60, 0, step: 0.5, section: "Frame", visible: whenAny("style", ["border", "rounded"])),
        .color("color", "Frame Color", RGBA(hex: "#F5F4F0"), section: "Frame", visible: whenNot("style", "film")),
        .slider("texture", "Texture", 0, 100, 50, step: 1, unit: "%", section: "Frame"),
        .slider("shadow", "Inner Shadow", 0, 100, 35, step: 1, unit: "%", section: "Frame"),
        ParamDef(key: "caption", label: "Caption", kind: .text(def: ""), section: "Text",
                 visible: whenAny("style", ["polaroid", "instax", "slide"])),
        ParamDef(key: "edgeText", label: "Edge Print", kind: .text(def: "KODAK PORTRA 400"), section: "Text", visible: when("style", "film")),
        .slider("frameNo", "Frame Number", 0, 40, 12, step: 1, section: "Text", visible: when("style", "film")),
        .seed(section: "Text"),
    ]

    func apply(_ input: MTLTexture, _ p: Params, _ ctx: RenderContext) -> MTLTexture {
        let W = CGFloat(input.width), H = CGFloat(input.height)
        let m = min(W, H)
        let k = CGFloat(p.num("size", 100) / 100)
        let style = p.choice("style", "polaroid")
        let vertical = H > W
        var frameColor = p.color("color", RGBA(hex: "#F5F4F0"))
        var rect: CGRect
        var radius: CGFloat = CGFloat(ctx.px(p.num("radius", 0)))
        switch style {
        case "instax":
            let s = m * 0.065 * k, t = m * 0.09 * k, b = m * 0.24 * k
            rect = CGRect(x: s, y: t, width: W - 2 * s, height: H - t - b)
            radius = m * 0.006
        case "film":
            frameColor = RGBA(hex: "#15110F")
            let band = (vertical ? W : H) * 0.16 * k
            let gap = (vertical ? H : W) * 0.035 * k
            rect = vertical ? CGRect(x: band, y: gap, width: W - 2 * band, height: H - 2 * gap)
                            : CGRect(x: gap, y: band, width: W - 2 * gap, height: H - 2 * band)
            radius = m * 0.008
        case "slide":
            let s = m * 0.17 * k
            rect = CGRect(x: s, y: s, width: W - 2 * s, height: H - 2 * s)
            radius = m * 0.025
        case "border", "rounded":
            let s = m * 0.05 * k
            rect = CGRect(x: s, y: s, width: W - 2 * s, height: H - 2 * s)
            if style == "rounded" && radius == 0 { radius = m * 0.02 }
        default: // polaroid
            let s = m * 0.055 * k, b = m * 0.22 * k
            rect = CGRect(x: s, y: s, width: W - 2 * s, height: H - s - b)
            radius = m * 0.002
        }
        rect = rect.integral
        // cover-fit the image into the window
        let ra = rect.width / max(1, rect.height), ia = W / H
        var src = CGRect(x: 0, y: 0, width: 1, height: 1)
        if ia > ra { let w = ra / ia; src = CGRect(x: (1 - w) / 2, y: 0, width: w, height: 1) }
        else { let h = ia / ra; src = CGRect(x: 0, y: (1 - h) / 2, width: 1, height: h) }
        let tex: Float = style == "film" ? 0.2 : 1
        var out = ctx.apply("k_frame", [input], [Float(rect.minX), Float(rect.minY), Float(rect.width), Float(rect.height),
                                                  Float(radius)] + frameColor.floats + [p.f("texture", 50) / 100 * tex,
                                                  p.f("shadow", 35) / 100, p.f("seed", 1) * 3.3, Float(ctx.scale),
                                                  Float(src.minX), Float(src.minY), Float(src.width), Float(src.height)])
        // decorations
        let ov = OverlayCanvas(input.width, input.height)
        let g = ov.ctx
        var hasOverlay = false
        let caption = p.text("caption", "")
        if style == "film" {
            hasOverlay = true
            let band = (vertical ? W : H) * 0.16 * k
            let hole = band * 0.32, holeL = band * 0.24
            let pitch = holeL * 1.9
            g.setFillColor(CGColor(srgbRed: 0.95, green: 0.94, blue: 0.9, alpha: 1))
            let length = vertical ? H : W
            var t = pitch * 0.4
            while t < length {
                for side in 0..<2 {
                    let off = side == 0 ? band * 0.2 : (vertical ? W : H) - band * 0.2 - hole
                    let r = vertical ? CGRect(x: off, y: t, width: hole, height: holeL) : CGRect(x: t, y: off, width: holeL, height: hole)
                    g.addPath(CGPath(roundedRect: r, cornerWidth: holeL * 0.18, cornerHeight: holeL * 0.18, transform: nil))
                }
                t += pitch
            }
            g.fillPath()
            let orange = RGBA(hex: "#E39A3B")
            let fs = band * 0.17
            let edge = p.text("edgeText", "KODAK PORTRA 400")
            let n = Int(p.num("frameNo", 12))
            if !vertical {
                ov.text(edge + "   ▸ \(n)", at: CGPoint(x: W * 0.08, y: band * 0.62), size: fs, color: orange, weight: .bold)
                ov.text("\(n)A", at: CGPoint(x: W * 0.5, y: H - band * 0.62 - fs), size: fs, color: orange, weight: .bold)
                ov.text(edge, at: CGPoint(x: W * 0.62, y: band * 0.62), size: fs, color: orange, weight: .bold)
            } else {
                ov.text("\(n)  \(n)A", at: CGPoint(x: band * 0.6, y: H * 0.02), size: fs, color: orange, weight: .bold)
            }
        } else if !caption.isEmpty && ["polaroid", "instax"].contains(style) {
            hasOverlay = true
            let bottom = H - rect.maxY
            let fs = bottom * 0.3
            ov.text(caption, at: CGPoint(x: rect.minX + rect.width * 0.06, y: rect.maxY + (bottom - fs) * 0.45), size: fs,
                    color: RGBA(hex: "#1D2A5A"), weight: .regular, mono: false, fontName: "Noteworthy-Bold")
        } else if style == "slide" {
            hasOverlay = true
            let s = rect.minY
            let fs = s * 0.2
            let label = caption.isEmpty ? "KODACHROME TRANSPARENCY" : caption
            ov.text(label, at: CGPoint(x: rect.minX, y: (s - fs) * 0.5), size: fs, color: RGBA(hex: "#8E8A82"), weight: .bold, mono: false)
        }
        if hasOverlay { out = ctx.apply("k_over_pm", [out, ov.texture()]) }
        return out
    }
}

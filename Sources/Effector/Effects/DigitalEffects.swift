import Foundation
import Metal
import CoreGraphics
import ImageIO
import UniformTypeIdentifiers

enum DigitalShaders {
    static let source = """
    // Early digital camera colour science.
    // u: 0 seed, 1 unit, 2 white balance (-1 cool … 1 warm), 3 saturation, 4 contrast, 5 clip, 6 fringe,
    //    7 noise, 8 chroma noise, 9 exposure
    kernel void k_digicam(KIN, texture2d<float> a [[texture(1)]], texture2d<float> bright [[texture(2)]]) {
        GUARD
        float2 p = float2(gid) + 0.5;
        float unit = max(u[1], 1e-3);
        float4 s = a.read(gid);
        float3 c = clamp(s.rgb, 0.0, 1.0);
        c = toSRGB(toLinear(c) * exp2(u[9]));
        float wb = u[2];
        c *= float3(1.0 + 0.08 * wb, 1.0 + 0.01 * wb, 1.0 - 0.12 * wb);
        // punchy tone curve with a hard digital clip
        float k = u[4];
        c = (c - 0.45) * k + 0.45;
        float clip = mix(1.0, 0.86, u[5]);
        c = clamp(c / clip, 0.0, 1.0);
        float l = luma(c);
        c = mix(float3(l), c, u[3]);
        // purple fringing around blown highlights
        float b = bright.sample(sLin, UV).r;
        float fr = max(0.0, b - smoothstep(0.75, 1.0, l)) * u[6];
        c = mix(c, c * float3(0.75, 0.45, 1.0) + float3(0.25, 0.0, 0.45) * fr, clamp(fr * 1.6, 0.0, 0.85));
        // sensor noise: luma grain + blotchy chroma, strongest in the shadows
        float sh = 1.0 - smoothstep(0.0, 0.7, l);
        float2 q = p / unit;
        float n = gnoise(float2(gid), u[0]) * u[7] * 0.05 * (0.4 + sh);
        float2 cn = float2(vnoise(q * 0.45, u[0] + 3.0) - 0.5, vnoise(q * 0.45 + 9.0, u[0] + 5.0) - 0.5);
        c += n;
        c += float3(cn.x, -0.5 * (cn.x + cn.y), cn.y) * u[8] * 0.16 * sh;
        o.write(float4(clamp(c, 0.0, 1.0), s.a), gid);
    }
    kernel void k_brightmask(KIN, texture2d<float> a [[texture(1)]]) {
        GUARD
        float l = luma(clamp(a.read(gid).rgb, 0.0, 1.0));
        o.write(float4(float3(smoothstep(0.8, 0.97, l)), 1.0), gid);
    }
    """
}

extension RenderContext {
    /// Encodes the texture as a real JPEG (ImageIO) `generations` times and decodes it back.
    func jpegRoundtrip(_ tex: MTLTexture, quality: Double, generations: Int = 1) -> MTLTexture {
        let w = tex.width, h = tex.height
        let shared = gpu.makeSharedTexture(w, h, format: .rgba8Unorm)
        run("k_clamp", out: shared, [tex])
        flush()
        let cs = CGColorSpace(name: CGColorSpace.sRGB)!
        guard let bctx = CGContext(data: nil, width: w, height: h, bitsPerComponent: 8, bytesPerRow: w * 4, space: cs,
                                   bitmapInfo: CGImageAlphaInfo.noneSkipLast.rawValue) else { return tex }
        shared.getBytes(bctx.data!, bytesPerRow: w * 4, from: MTLRegionMake2D(0, 0, w, h), mipmapLevel: 0)
        guard var cg = bctx.makeImage() else { return tex }
        for g in 0..<max(1, generations) {
            let data = NSMutableData()
            guard let dest = CGImageDestinationCreateWithData(data, UTType.jpeg.identifier as CFString, 1, nil) else { break }
            // successive generations drift slightly in quality, like re-saved files
            let q = max(0.0, min(1.0, quality + Double(g % 3) * 0.03))
            CGImageDestinationAddImage(dest, cg, [kCGImageDestinationLossyCompressionQuality: q] as CFDictionary)
            guard CGImageDestinationFinalize(dest),
                  let src = CGImageSourceCreateWithData(data, nil),
                  let dec = CGImageSourceCreateImageAtIndex(src, 0, nil) else { break }
            cg = dec
        }
        bctx.draw(cg, in: CGRect(x: 0, y: 0, width: w, height: h))
        let d = MTLTextureDescriptor.texture2DDescriptor(pixelFormat: .rgba8Unorm, width: w, height: h, mipmapped: false)
        d.usage = [.shaderRead]
        d.storageMode = .shared
        let up = gpu.device.makeTexture(descriptor: d)!
        up.replace(region: MTLRegionMake2D(0, 0, w, h), mipmapLevel: 0, withBytes: bctx.data!, bytesPerRow: w * 4)
        return apply("k_copy", [up])
    }
}

struct JPEGEffect: Effect {
    let id = "jpeg"
    let name = "JPEG Artifacts"
    let category = EffectCategory.glitch
    var symbol: String { "square.grid.3x3.square" }
    var summary: String { "Real JPEG re-compression: blocky 8×8 artifacts, ringing and colour smear." }
    let params: [ParamDef] = [
        .slider("quality", "Quality", 0, 100, 12, step: 1, unit: "%"),
        .slider("block", "Block Size", 1, 12, 2, step: 0.5, curve: 1.4),
        .slider("generations", "Generations", 1, 12, 1, step: 1),
    ]
    func apply(_ input: MTLTexture, _ p: Params, _ ctx: RenderContext) -> MTLTexture {
        // the block size is relative: compress a downscaled copy so blocks stay visible at any resolution
        let f = max(1, ctx.px(p.num("block", 2)) / 1.0)
        let w = max(8, Int(Double(input.width) / f)), h = max(8, Int(Double(input.height) / f))
        let small = ctx.resize(input, w, h)
        let jpg = ctx.jpegRoundtrip(small, quality: p.num("quality", 12) / 100, generations: Int(p.num("generations", 1)))
        if w == input.width && h == input.height { return jpg }
        return ctx.upscaleNearest(jpg, input.width, input.height)
    }
}

struct DigicamEffect: Effect {
    let id = "digicam"
    let name = "Digicam 2003"
    let category = EffectCategory.film
    var symbol: String { "camera.compact" }
    var summary: String { "Early-2000s compact digital camera: CCD colour, clipped highlights, noise, JPEG and date stamp." }
    let params: [ParamDef] = [
        .slider("resolution", "Sensor Resolution", 10, 100, 45, step: 1, unit: "%", section: "Camera"),
        .slider("exposure", "Exposure", -2, 2, 0.2, step: 0.05, unit: " EV", section: "Camera"),
        .slider("wb", "White Balance", -100, 100, -25, step: 1, section: "Camera",
                track: [RGBA(hex: "#3A7BFF"), RGBA(hex: "#9A9A9A"), RGBA(hex: "#FFB020")]),
        .slider("saturation", "Saturation", 0, 200, 115, step: 1, unit: "%", section: "Camera"),
        .slider("contrast", "Contrast", 50, 200, 118, step: 1, unit: "%", section: "Camera"),
        .slider("clip", "Highlight Clipping", 0, 100, 45, step: 1, unit: "%", section: "Camera"),
        .slider("fringe", "Purple Fringing", 0, 100, 40, step: 1, unit: "%", section: "Camera"),
        .slider("sharpen", "Over-sharpening", 0, 300, 120, step: 1, unit: "%", section: "Processing"),
        .slider("noise", "Noise", 0, 100, 35, step: 1, unit: "%", section: "Processing"),
        .slider("chromaNoise", "Chroma Noise", 0, 100, 40, step: 1, unit: "%", section: "Processing"),
        .slider("quality", "JPEG Quality", 0, 100, 55, step: 1, unit: "%", section: "Processing"),
        .toggle("date", "Date Stamp", true, section: "Date Stamp"),
        ParamDef(key: "dateText", label: "Text", kind: .text(def: "'03 8 14"), section: "Date Stamp", visible: whenOn("date")),
        .choice("datePos", "Position", [("br", "Bottom Right"), ("bl", "Bottom Left"), ("tr", "Top Right")], "br",
                section: "Date Stamp", visible: whenOn("date")),
        .slider("dateSize", "Size", 8, 80, 28, step: 1, section: "Date Stamp", visible: whenOn("date")),
        .color("dateColor", "Color", RGBA(hex: "#FF8A1E"), section: "Date Stamp", visible: whenOn("date")),
        .seed(section: "Date Stamp"),
    ]
    var looks: [EffectLook] {
        [
            EffectLook(id: "cybershot", name: "2 Megapixel Point & Shoot", values: [:]),
            EffectLook(id: "flash", name: "Party Flash", values: ["exposure": 0.6, "wb": -40.0, "clip": 75.0, "contrast": 130.0,
                                                                "fringe": 55.0, "dateText": ParamValue.text("2004/12/31 23:58")]),
            EffectLook(id: "phone", name: "Flip Phone VGA", values: ["resolution": 18.0, "saturation": 90.0, "contrast": 110.0,
                                                                   "noise": 60.0, "chromaNoise": 70.0, "quality": 25.0, "sharpen": 60.0,
                                                                   "date": false]),
            EffectLook(id: "webcam", name: "Webcam 2002", values: ["resolution": 14.0, "wb": 20.0, "saturation": 80.0, "noise": 70.0,
                                                                 "chromaNoise": 80.0, "quality": 18.0, "clip": 60.0, "date": false]),
            EffectLook(id: "sunny", name: "Holiday Snapshot", values: ["exposure": 0.3, "wb": 10.0, "saturation": 135.0,
                                                                     "dateText": ParamValue.text("'02 7 21")]),
        ]
    }
    func apply(_ input: MTLTexture, _ p: Params, _ ctx: RenderContext) -> MTLTexture {
        let W = input.width, H = input.height
        let res = max(0.05, p.num("resolution", 45) / 100)
        var img = ctx.resize(input, max(8, Int(Double(W) * res)), max(8, Int(Double(H) * res)))
        let unit = Float(ctx.scale * res)
        let bright = ctx.blur(ctx.apply("k_brightmask", [img]), sigma: max(1, Double(unit) * 3))
        img = ctx.apply("k_digicam", [img, bright], [p.f("seed", 1) * 4.1, unit, p.f("wb", -25) / 100, p.f("saturation", 115) / 100,
                                                     p.f("contrast", 118) / 100, p.f("clip", 45) / 100, p.f("fringe", 40) / 100,
                                                     p.f("noise", 35) / 100, p.f("chromaNoise", 40) / 100, p.f("exposure", 0.2)])
        let sh = p.num("sharpen", 120) / 100
        if sh > 0.01 {
            let b = ctx.blur(img, sigma: max(0.6, Double(unit) * 1.2))
            img = ctx.apply("k_unsharp", [img, b], [Float(sh), 0.0, 1])
        }
        img = ctx.resize(img, W, H, smooth: true)
        if p.bool("date", true) {
            let ov = OverlayCanvas(W, H)
            let size = CGFloat(ctx.px(p.num("dateSize", 28)))
            let text = p.text("dateText", "'03 8 14")
            let color = p.color("dateColor", RGBA(hex: "#FF8A1E"))
            let margin = CGFloat(min(W, H)) * 0.055
            let est = CGFloat(text.count) * size * 0.62
            let pos: CGPoint
            switch p.choice("datePos", "br") {
            case "bl": pos = CGPoint(x: margin, y: CGFloat(H) - margin - size)
            case "tr": pos = CGPoint(x: CGFloat(W) - margin - est, y: margin)
            default: pos = CGPoint(x: CGFloat(W) - margin - est, y: CGFloat(H) - margin - size)
            }
            ov.text(text, at: pos, size: size, color: color, weight: .bold, fontName: "Menlo-Bold", kern: 0.06)
            // a soft glow like the LED imprint
            var glow = ctx.apply("k_copy", [ov.texture()])
            glow = ctx.blur(glow, sigma: Double(size) * 0.12)
            img = ctx.apply("k_over_pm_op", [img, glow], [0.6])
            img = ctx.apply("k_over_pm_op", [img, ov.texture()], [0.95])
        }
        let q = p.num("quality", 55) / 100
        if q < 0.99 {
            // compress at camera resolution so the blocks have the right size
            let small = ctx.resize(img, max(8, Int(Double(W) * res)), max(8, Int(Double(H) * res)))
            img = ctx.resize(ctx.jpegRoundtrip(small, quality: q), W, H, smooth: true)
        }
        return img
    }
}

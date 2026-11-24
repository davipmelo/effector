import Foundation
import CoreGraphics
import CoreText
import AppKit
import Metal

/// CoreGraphics drawing surface (top-left origin) that becomes a straight-alpha Metal texture.
final class OverlayCanvas {
    let w: Int, h: Int
    let ctx: CGContext

    init(_ w: Int, _ h: Int) {
        self.w = w; self.h = h
        ctx = CGContext(data: nil, width: w, height: h, bitsPerComponent: 8, bytesPerRow: w * 4,
                        space: CGColorSpace(name: CGColorSpace.sRGB)!,
                        bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue)!
        ctx.translateBy(x: 0, y: CGFloat(h))
        ctx.scaleBy(x: 1, y: -1)
        ctx.setShouldAntialias(true)
    }

    func cg(_ c: RGBA, alpha: Double = 1) -> CGColor {
        CGColor(srgbRed: c.r, green: c.g, blue: c.b, alpha: c.a * alpha)
    }

    /// Draws text with its top-left at `p`. Returns the text size.
    @discardableResult
    func text(_ s: String, at p: CGPoint, size: CGFloat, color: RGBA, weight: NSFont.Weight = .semibold,
              mono: Bool = true, shadow: Bool = false, background: RGBA? = nil, padding: CGFloat = 0,
              fontName: String? = nil, stretch: CGFloat = 1, outline: RGBA? = nil, kern: CGFloat = 0) -> CGSize {
        let font = fontName.flatMap { NSFont(name: $0, size: size) }
            ?? (mono ? NSFont.monospacedSystemFont(ofSize: size, weight: weight) : NSFont.systemFont(ofSize: size, weight: weight))
        var attrs: [NSAttributedString.Key: Any] = [.font: font, .foregroundColor: NSColor(cgColor: cg(color)) ?? .white]
        if kern != 0 { attrs[.kern] = kern * size }
        let attr = NSAttributedString(string: s, attributes: attrs)
        let line = CTLineCreateWithAttributedString(attr)
        var ascent: CGFloat = 0, descent: CGFloat = 0, leading: CGFloat = 0
        let width = CGFloat(CTLineGetTypographicBounds(line, &ascent, &descent, &leading)) * stretch
        let height = ascent + descent
        if let bg = background {
            ctx.setFillColor(cg(bg))
            ctx.fill(CGRect(x: p.x - padding, y: p.y - padding, width: width + padding * 2, height: height + padding * 2))
        }
        ctx.saveGState()
        // flip back locally so glyphs are upright
        ctx.translateBy(x: p.x, y: p.y + ascent)
        ctx.scaleBy(x: stretch, y: -1)
        if shadow {
            ctx.setShadow(offset: CGSize(width: size * 0.08, height: -size * 0.08), blur: size * 0.15,
                          color: CGColor(gray: 0, alpha: 0.8))
        }
        if let outline {
            // draw a stroked copy underneath for the classic outlined OSD look
            let o = NSMutableAttributedString(attributedString: attr)
            o.addAttributes([.strokeWidth: 18, .strokeColor: NSColor(cgColor: cg(outline)) ?? .black],
                            range: NSRange(location: 0, length: o.length))
            ctx.textPosition = .zero
            CTLineDraw(CTLineCreateWithAttributedString(o), ctx)
        }
        ctx.textPosition = .zero
        CTLineDraw(line, ctx)
        ctx.restoreGState()
        return CGSize(width: width, height: height)
    }

    /// Uploads as an rgba texture (premultiplied; use k_overlay_unpremul or shaders that unpremultiply).
    func texture() -> MTLTexture {
        let d = MTLTextureDescriptor.texture2DDescriptor(pixelFormat: .rgba8Unorm, width: w, height: h, mipmapped: false)
        d.usage = [.shaderRead]
        d.storageMode = .shared
        let t = GPU.shared.device.makeTexture(descriptor: d)!
        t.replace(region: MTLRegionMake2D(0, 0, w, h), mipmapLevel: 0, withBytes: ctx.data!, bytesPerRow: w * 4)
        return t
    }
}

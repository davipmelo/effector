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
              mono: Bool = true, shadow: Bool = false, background: RGBA? = nil, padding: CGFloat = 0) -> CGSize {
        let font = mono ? NSFont.monospacedSystemFont(ofSize: size, weight: weight) : NSFont.systemFont(ofSize: size, weight: weight)
        let attr = NSAttributedString(string: s, attributes: [.font: font, .foregroundColor: NSColor(cgColor: cg(color)) ?? .white])
        let line = CTLineCreateWithAttributedString(attr)
        var ascent: CGFloat = 0, descent: CGFloat = 0, leading: CGFloat = 0
        let width = CGFloat(CTLineGetTypographicBounds(line, &ascent, &descent, &leading))
        let height = ascent + descent
        if let bg = background {
            ctx.setFillColor(cg(bg))
            ctx.fill(CGRect(x: p.x - padding, y: p.y - padding, width: width + padding * 2, height: height + padding * 2))
        }
        ctx.saveGState()
        // flip back locally so glyphs are upright
        ctx.translateBy(x: p.x, y: p.y + ascent)
        ctx.scaleBy(x: 1, y: -1)
        if shadow {
            ctx.setShadow(offset: CGSize(width: size * 0.08, height: -size * 0.08), blur: size * 0.15,
                          color: CGColor(gray: 0, alpha: 0.8))
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

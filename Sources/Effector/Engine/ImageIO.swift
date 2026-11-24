import Foundation
import AppKit
import CoreImage
import ImageIO
import Metal
import UniformTypeIdentifiers

/// The original image of a document.
final class SourceImage {
    let cgImage: CGImage
    let data: Data
    let fileName: String
    let id = UUID()

    var width: Int { cgImage.width }
    var height: Int { cgImage.height }

    init(cgImage: CGImage, data: Data, fileName: String) {
        self.cgImage = cgImage
        self.data = data
        self.fileName = fileName
    }

    enum LoadError: LocalizedError {
        case unreadable
        var errorDescription: String? { tr("This image could not be opened.") }
    }

    static func load(url: URL) throws -> SourceImage {
        let data = try Data(contentsOf: url)
        return try load(data: data, fileName: url.lastPathComponent, typeHint: UTType(filenameExtension: url.pathExtension))
    }

    static func load(data: Data, fileName: String, typeHint: UTType? = nil) throws -> SourceImage {
        if let t = typeHint, t.conforms(to: .rawImage) {
            if let f = CIRAWFilter(imageData: data, identifierHint: t.identifier),
               let ci = f.outputImage,
               let cg = GPU.shared.ciContext.createCGImage(ci, from: ci.extent, format: .RGBA16,
                                                           colorSpace: CGColorSpace(name: CGColorSpace.sRGB)) {
                return SourceImage(cgImage: cg, data: data, fileName: fileName)
            }
        }
        guard let src = CGImageSourceCreateWithData(data as CFData, nil) else { throw LoadError.unreadable }
        let props = CGImageSourceCopyPropertiesAtIndex(src, 0, nil) as? [CFString: Any]
        let w = props?[kCGImagePropertyPixelWidth] as? Int ?? 0
        let h = props?[kCGImagePropertyPixelHeight] as? Int ?? 0
        let opts: [CFString: Any] = [
            kCGImageSourceCreateThumbnailFromImageAlways: true,
            kCGImageSourceCreateThumbnailWithTransform: true,
            kCGImageSourceThumbnailMaxPixelSize: max(w, h, 1),
            kCGImageSourceShouldCacheImmediately: true,
        ]
        if let cg = CGImageSourceCreateThumbnailAtIndex(src, 0, opts as CFDictionary) {
            return SourceImage(cgImage: cg, data: data, fileName: fileName)
        }
        // Fallback: maybe a RAW without a type hint.
        if let f = CIRAWFilter(imageData: data, identifierHint: nil), let ci = f.outputImage,
           let cg = GPU.shared.ciContext.createCGImage(ci, from: ci.extent) {
            return SourceImage(cgImage: cg, data: data, fileName: fileName)
        }
        throw LoadError.unreadable
    }

    static func fromPasteboard() -> SourceImage? {
        let pb = NSPasteboard.general
        if let urls = pb.readObjects(forClasses: [NSURL.self], options: [.urlReadingFileURLsOnly: true]) as? [URL],
           let u = urls.first, let s = try? load(url: u) { return s }
        for type in [NSPasteboard.PasteboardType.png, .tiff] {
            if let d = pb.data(forType: type), let s = try? load(data: d, fileName: "Pasted Image.png") { return s }
        }
        if let img = NSImage(pasteboard: pb), let tiff = img.tiffRepresentation,
           let s = try? load(data: tiff, fileName: "Pasted Image.png") { return s }
        return nil
    }

    var hasAlpha: Bool {
        switch cgImage.alphaInfo {
        case .none, .noneSkipFirst, .noneSkipLast: return false
        default: return true
        }
    }

    /// Cropped size in original pixels.
    func croppedSize(_ crop: CropRect) -> (Int, Int) {
        let r = crop.pixelRect(width: width, height: height)
        return (Int(r.width), Int(r.height))
    }

    /// Creates a straight-alpha rgba16Float texture of the cropped image at `scale`.
    func makeTexture(crop: CropRect, scale: Double) -> MTLTexture {
        let rect = crop.pixelRect(width: width, height: height)
        let tw = max(1, Int((rect.width * scale).rounded()))
        let th = max(1, Int((rect.height * scale).rounded()))
        let gpu = GPU.shared
        let hdr = cgImage.bitsPerComponent > 8
        let cs = CGColorSpace(name: CGColorSpace.sRGB)!
        let bpc = hdr ? 16 : 8
        let bytesPerPixel = hdr ? 8 : 4
        let info: UInt32 = hdr
            ? CGImageAlphaInfo.premultipliedLast.rawValue | CGBitmapInfo.floatComponents.rawValue | CGImageByteOrderInfo.order16Little.rawValue
            : CGImageAlphaInfo.premultipliedLast.rawValue
        let ctx = CGContext(data: nil, width: tw, height: th, bitsPerComponent: bpc, bytesPerRow: tw * bytesPerPixel,
                            space: cs, bitmapInfo: info)!
        ctx.interpolationQuality = .high
        // Draw so that the crop rect maps onto the context (CG origin is bottom-left).
        let sx = Double(tw) / rect.width, sy = Double(th) / rect.height
        let drawRect = CGRect(x: -rect.minX * sx,
                              y: -(Double(height) - rect.maxY) * sy,
                              width: Double(width) * sx, height: Double(height) * sy)
        ctx.draw(cgImage, in: drawRect)
        let fmt: MTLPixelFormat = hdr ? .rgba16Float : .rgba8Unorm
        let d = MTLTextureDescriptor.texture2DDescriptor(pixelFormat: fmt, width: tw, height: th, mipmapped: false)
        d.usage = [.shaderRead]
        d.storageMode = .shared
        let up = gpu.device.makeTexture(descriptor: d)!
        up.replace(region: MTLRegionMake2D(0, 0, tw, th), mipmapLevel: 0, withBytes: ctx.data!, bytesPerRow: tw * bytesPerPixel)
        let out = gpu.makeTexture(tw, th)
        let rc = RenderContext(scale: 1, source: out, isExport: false)
        rc.run("k_unpremul", out: out, [up])
        rc.flush()
        return out
    }
}

enum ImageExport {
    enum Format: String, CaseIterable, Codable { case png, jpg }

    /// Converts a straight-alpha texture into a CIImage (premultiplied, correct orientation).
    static func ciImage(_ tex: MTLTexture) -> CIImage {
        let rc = RenderContext(scale: 1, source: tex, isExport: true)
        let pre = rc.apply("k_premul", [tex])
        rc.flush()
        let cs = CGColorSpace(name: CGColorSpace.sRGB)!
        let img = CIImage(mtlTexture: pre, options: [.colorSpace: cs])!
        return img.oriented(.downMirrored)
    }

    static func cgImage(_ tex: MTLTexture) -> CGImage? {
        let ci = ciImage(tex)
        return GPU.shared.ciContext.createCGImage(ci, from: ci.extent, format: .RGBA8,
                                                  colorSpace: CGColorSpace(name: CGColorSpace.sRGB))
    }

    static func write(_ tex: MTLTexture, to url: URL, format: Format, quality: Double, keepAlpha: Bool) throws {
        let ci = ciImage(tex)
        let ctx = GPU.shared.ciContext
        let cs = CGColorSpace(name: CGColorSpace.sRGB)!
        switch format {
        case .png:
            if keepAlpha {
                try ctx.writePNGRepresentation(of: ci, to: url, format: .RGBA8, colorSpace: cs)
            } else {
                let flat = ci.composited(over: CIImage(color: .white).cropped(to: ci.extent))
                try ctx.writePNGRepresentation(of: flat, to: url, format: .RGBA8, colorSpace: cs)
            }
        case .jpg:
            let flat = ci.composited(over: CIImage(color: .white).cropped(to: ci.extent))
            try ctx.writeJPEGRepresentation(of: flat, to: url, colorSpace: cs,
                                            options: [kCGImageDestinationLossyCompressionQuality as CIImageRepresentationOption: quality])
        }
    }

    static func pngData(_ tex: MTLTexture) -> Data? {
        guard let cg = cgImage(tex) else { return nil }
        let rep = NSBitmapImageRep(cgImage: cg)
        return rep.representation(using: .png, properties: [:])
    }
}

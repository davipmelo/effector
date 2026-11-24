import Foundation
import Metal
import MetalPerformanceShaders
import CoreImage

/// Shared Metal device, queue and compiled shader library.
final class GPU {
    static let shared = GPU()

    let device: MTLDevice
    let queue: MTLCommandQueue
    let library: MTLLibrary
    private var pipelines: [String: MTLComputePipelineState] = [:]
    private let lock = NSLock()
    lazy var ciContext = CIContext(mtlDevice: device, options: [
        .workingColorSpace: CGColorSpace(name: CGColorSpace.sRGB)!,
        .cacheIntermediates: false,
    ])

    private init() {
        guard let d = MTLCreateSystemDefaultDevice() else { fatalError("Metal is not available") }
        device = d
        queue = d.makeCommandQueue()!
        queue.label = "Effector"
        let source = ShaderLibrary.fullSource
        do {
            let options = MTLCompileOptions()
            options.mathMode = .fast
            library = try d.makeLibrary(source: source, options: options)
        } catch {
            // Print with line numbers to make shader errors debuggable.
            let lines = source.split(separator: "\n", omittingEmptySubsequences: false)
            for (i, l) in lines.enumerated() { FileHandle.standardError.write("\(i + 1): \(l)\n".data(using: .utf8)!) }
            fatalError("Shader compile failed: \(error)")
        }
    }

    func pipeline(_ name: String) -> MTLComputePipelineState {
        lock.lock(); defer { lock.unlock() }
        if let p = pipelines[name] { return p }
        guard let fn = library.makeFunction(name: name) else { fatalError("Missing kernel \(name)") }
        let p = try! device.makeComputePipelineState(function: fn)
        pipelines[name] = p
        return p
    }

    func makeTexture(_ w: Int, _ h: Int, format: MTLPixelFormat = .rgba16Float) -> MTLTexture {
        let d = MTLTextureDescriptor.texture2DDescriptor(pixelFormat: format, width: max(1, w), height: max(1, h), mipmapped: false)
        d.usage = [.shaderRead, .shaderWrite, .renderTarget]
        d.storageMode = .private
        return device.makeTexture(descriptor: d)!
    }

    func makeSharedTexture(_ w: Int, _ h: Int, format: MTLPixelFormat = .rgba16Float) -> MTLTexture {
        let d = MTLTextureDescriptor.texture2DDescriptor(pixelFormat: format, width: max(1, w), height: max(1, h), mipmapped: false)
        d.usage = [.shaderRead, .shaderWrite]
        d.storageMode = .shared
        return device.makeTexture(descriptor: d)!
    }

    /// 1D lookup table stored as a Nx1 rgba32Float texture.
    func makeLUT1D(_ values: [SIMD4<Float>]) -> MTLTexture {
        let d = MTLTextureDescriptor.texture2DDescriptor(pixelFormat: .rgba32Float, width: values.count, height: 1, mipmapped: false)
        d.usage = [.shaderRead]
        d.storageMode = .shared
        let t = device.makeTexture(descriptor: d)!
        values.withUnsafeBytes { buf in
            t.replace(region: MTLRegionMake2D(0, 0, values.count, 1), mipmapLevel: 0,
                      withBytes: buf.baseAddress!, bytesPerRow: values.count * 16)
        }
        return t
    }

    func makeTexture(fromRGBA floats: [Float], width: Int, height: Int) -> MTLTexture {
        let d = MTLTextureDescriptor.texture2DDescriptor(pixelFormat: .rgba32Float, width: width, height: height, mipmapped: false)
        d.usage = [.shaderRead]
        d.storageMode = .shared
        let t = device.makeTexture(descriptor: d)!
        floats.withUnsafeBytes { buf in
            t.replace(region: MTLRegionMake2D(0, 0, width, height), mipmapLevel: 0,
                      withBytes: buf.baseAddress!, bytesPerRow: width * 16)
        }
        return t
    }
}

/// Per-render context: command buffer management and helpers used by effects.
final class RenderContext {
    let gpu = GPU.shared
    private(set) var cb: MTLCommandBuffer
    /// Render pixels per original-image pixel. Size params are in original pixels.
    let scale: Double
    let width: Int
    let height: Int
    /// Full-resolution source of the document (cropped), at render size. Used by effects needing the original.
    let source: MTLTexture
    /// Is this a final export render (allows slower / higher quality paths).
    let isExport: Bool

    init(scale: Double, source: MTLTexture, isExport: Bool) {
        self.scale = scale
        self.source = source
        self.width = source.width
        self.height = source.height
        self.isExport = isExport
        cb = gpu.queue.makeCommandBuffer()!
    }

    /// Converts a size in original-image pixels to render pixels.
    func px(_ v: Double) -> Double { v * scale }

    func newTex(_ w: Int? = nil, _ h: Int? = nil) -> MTLTexture {
        gpu.makeTexture(w ?? width, h ?? height)
    }

    func newTex(like t: MTLTexture) -> MTLTexture { gpu.makeTexture(t.width, t.height) }

    /// Runs a compute kernel. Output bound at texture(0), inputs at texture(1...).
    /// Uniform floats at buffer(0); optional extra floats at buffer(1).
    func run(_ kernel: String, out: MTLTexture, _ inputs: [MTLTexture] = [], _ u: [Float] = [], extra: [Float]? = nil) {
        let p = gpu.pipeline(kernel)
        guard let enc = cb.makeComputeCommandEncoder() else { return }
        enc.setComputePipelineState(p)
        enc.setTexture(out, index: 0)
        for (i, t) in inputs.enumerated() { enc.setTexture(t, index: i + 1) }
        var uu = u.isEmpty ? [Float(0)] : u
        if uu.count < 4 { uu += [Float](repeating: 0, count: 4 - uu.count) }
        enc.setBytes(&uu, length: uu.count * 4, index: 0)
        if var e = extra {
            if e.isEmpty { e = [0] }
            if e.count * 4 <= 4096 {
                enc.setBytes(&e, length: e.count * 4, index: 1)
            } else {
                let b = gpu.device.makeBuffer(bytes: &e, length: e.count * 4, options: .storageModeShared)
                enc.setBuffer(b, offset: 0, index: 1)
            }
        } else {
            var z: Float = 0
            enc.setBytes(&z, length: 4, index: 1)
        }
        let tg = MTLSize(width: 16, height: 16, depth: 1)
        enc.dispatchThreads(MTLSize(width: out.width, height: out.height, depth: 1), threadsPerThreadgroup: tg)
        enc.endEncoding()
    }

    /// Convenience: run a kernel producing a new texture of the given size.
    func apply(_ kernel: String, _ inputs: [MTLTexture], _ u: [Float] = [], extra: [Float]? = nil,
               w: Int? = nil, h: Int? = nil) -> MTLTexture {
        let out = newTex(w ?? inputs.first?.width ?? width, h ?? inputs.first?.height ?? height)
        run(kernel, out: out, inputs, u, extra: extra)
        return out
    }

    func blur(_ src: MTLTexture, sigma: Double) -> MTLTexture {
        let s = Float(max(0.01, sigma))
        if s < 0.3 { return src }
        // Large radii: blur a downscaled copy (fast + same look).
        if s > 12 {
            let f = max(1, Int(s / 6))
            let small = resize(src, max(1, src.width / f), max(1, src.height / f))
            let b = blur(small, sigma: Double(s) / Double(f))
            return resize(b, src.width, src.height, smooth: true)
        }
        let out = newTex(like: src)
        let k = MPSImageGaussianBlur(device: gpu.device, sigma: s)
        k.edgeMode = .clamp
        k.encode(commandBuffer: cb, sourceTexture: src, destinationTexture: out)
        return out
    }

    /// High-quality resize (Lanczos for downscale, bilinear for upscale).
    func resize(_ src: MTLTexture, _ w: Int, _ h: Int, smooth: Bool = false) -> MTLTexture {
        let w = max(1, w), h = max(1, h)
        if w == src.width && h == src.height { return src }
        let out = newTex(w, h)
        if w < src.width || h < src.height {
            if !smooth {
                let k = MPSImageLanczosScale(device: gpu.device)
                k.edgeMode = .clamp
                k.encode(commandBuffer: cb, sourceTexture: src, destinationTexture: out)
                // Lanczos can ring below 0 / above 1.
                return apply("k_clamp", [out])
            }
        }
        let k = MPSImageBilinearScale(device: gpu.device)
        k.edgeMode = .clamp
        k.encode(commandBuffer: cb, sourceTexture: src, destinationTexture: out)
        return out
    }

    /// Nearest-neighbour upscale (for pixel-art style effects).
    func upscaleNearest(_ src: MTLTexture, _ w: Int, _ h: Int) -> MTLTexture {
        apply("k_nearest", [src], [], w: w, h: h)
    }

    /// Upscale where every source texel becomes an exact pix×pix block (no seams).
    func upscaleBlocks(_ src: MTLTexture, pix: Int, _ w: Int, _ h: Int) -> MTLTexture {
        apply("k_block", [src], [Float(pix)], w: w, h: h)
    }

    /// Commit pending GPU work and wait (needed before CPU reads).
    func flush() {
        cb.commit()
        cb.waitUntilCompleted()
        cb = gpu.queue.makeCommandBuffer()!
    }

    // MARK: CPU readback / upload (rgba float)

    func readFloats(_ tex: MTLTexture) -> [Float] {
        let shared = gpu.makeSharedTexture(tex.width, tex.height, format: .rgba32Float)
        run("k_copy", out: shared, [tex])
        flush()
        var out = [Float](repeating: 0, count: tex.width * tex.height * 4)
        out.withUnsafeMutableBytes { buf in
            shared.getBytes(buf.baseAddress!, bytesPerRow: tex.width * 16,
                            from: MTLRegionMake2D(0, 0, tex.width, tex.height), mipmapLevel: 0)
        }
        return out
    }

    func upload(_ floats: [Float], _ w: Int, _ h: Int) -> MTLTexture {
        let t = gpu.makeTexture(fromRGBA: floats, width: w, height: h)
        return apply("k_copy", [t])
    }
}

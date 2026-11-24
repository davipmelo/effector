import Foundation
import Metal

/// Renders a layer tree on top of a source texture.
/// Layers are ordered top-first; the bottom-most layer is applied first.
final class Renderer {
    private var cache: [Int: MTLTexture] = [:]
    private let useCache: Bool

    init(useCache: Bool = true) {
        self.useCache = useCache
    }

    func clearCache() { cache.removeAll() }

    /// Size parameters are relative: expressed in pixels of a 1000 px (long side) reference image,
    /// so a stack looks identical at any resolution (preview, export 1x/2x, thumbnails).
    static let referenceSize = 1000.0

    /// `sourceKey` must change whenever the source texture content changes.
    func render(layers: [LayerNode], source: MTLTexture, sourceKey: Int, isExport: Bool = false, canvas: CanvasSpec? = nil) -> MTLTexture {
        let scale = Double(max(source.width, source.height)) / Renderer.referenceSize
        let ctx = RenderContext(scale: scale, source: source, isExport: isExport)
        var newCache: [Int: MTLTexture] = [:]
        var h = Hasher()
        h.combine(sourceKey); h.combine(scale); h.combine(source.width); h.combine(source.height)
        var (out, _) = renderStack(layers, input: source, inputHash: h.finalize(), ctx: ctx, newCache: &newCache)
        if let canvas { out = CanvasCompose.apply(out, canvas, ctx) }
        ctx.flush()
        if useCache { cache = newCache }
        return out
    }

    private func renderStack(_ nodes: [LayerNode], input: MTLTexture, inputHash: Int, ctx: RenderContext,
                             newCache: inout [Int: MTLTexture]) -> (MTLTexture, Int) {
        var current = input
        var currentHash = inputHash
        for node in nodes.reversed() {
            guard node.enabled else { continue }
            var h = Hasher()
            h.combine(currentHash)
            node.renderHash(into: &h)
            let key = h.finalize()
            if useCache, let cached = cache[key] {
                current = cached
                currentHash = key
                newCache[key] = cached
                continue
            }
            var out: MTLTexture
            if node.isGroup {
                let (sub, _) = renderStack(node.children, input: current, inputHash: currentHash, ctx: ctx, newCache: &newCache)
                out = sub
            } else if let eff = EffectRegistry.effect(node.effectID) {
                out = eff.apply(current, node.params, ctx)
                if out.width != current.width || out.height != current.height {
                    out = ctx.resize(out, current.width, current.height)
                }
                // Effects never change transparency: restore the exact input alpha.
                if out !== current && !eff.producesAlpha {
                    out = ctx.apply("k_withalpha", [out, current])
                }
            } else {
                continue
            }
            if node.opacity < 0.999 || node.blend != .normal || out !== current {
                out = ctx.apply("k_blend", [current, out], [Float(node.opacity), node.blend.shaderIndex])
            }
            if useCache { newCache[key] = out }
            current = out
            currentHash = key
        }
        return (current, currentHash)
    }
}

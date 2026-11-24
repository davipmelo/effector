import SwiftUI
import MetalKit
import AppKit

struct CanvasContainer: View {
    @ObservedObject var app = AppState.shared

    var body: some View {
        GeometryReader { geo in
            ZStack {
                (app.presenting ? Color.black : Theme.canvasBG)
                if app.hasImage {
                    CanvasView(app: app)
                    if app.compareSplit && !app.isCropping && !app.showOriginal {
                        SplitDivider(width: geo.size.width)
                    }
                    if app.isCropping {
                        CropOverlay(viewSize: geo.size)
                    }
                } else {
                    EmptyStateView()
                }
                VStack {
                    HStack {
                        if app.showOriginal && app.hasImage {
                            badge(tr("ORIGINAL"), highlight: true)
                        }
                        if let s = app.soloID, let n = app.state.layers.find(s) {
                            badge(tr("SOLO") + " · " + n.name, highlight: true)
                        }
                        Spacer()
                    }
                    Spacer()
                    if let t = app.toast {
                        Text(t)
                            .font(.system(size: 12, weight: .medium))
                            .padding(.horizontal, 14).padding(.vertical, 8)
                            .background(.ultraThinMaterial, in: Capsule())
                            .transition(.move(edge: .bottom).combined(with: .opacity))
                            .padding(.bottom, 16)
                    }
                }
                .padding(12)
                .animation(.snappy, value: app.toast)
                if app.isExporting {
                    ProgressView().controlSize(.large)
                        .padding(24)
                        .background(.ultraThinMaterial, in: RoundedRectangle(cornerRadius: 14))
                }
            }
            .onAppear { app.viewportSize = geo.size }
            .onChange(of: geo.size) { _, s in app.viewportSize = s }
        }
        .onDrop(of: [.fileURL], isTargeted: nil) { providers in
            guard let p = providers.first else { return false }
            _ = p.loadObject(ofClass: URL.self) { url, _ in
                guard let url else { return }
                DispatchQueue.main.async {
                    if url.pathExtension.lowercased() == "cube" {
                        // a LUT dropped on the canvas: save it and add a LUT layer
                        if app.hasImage, let l = LUTLibrary.shared.add(urls: [url]) { app.addLUTLayer(l) }
                    } else if app.confirmDiscardChanges() { app.open(url: url) }
                }
            }
            return true
        }
    }

    private func badge(_ s: String, highlight: Bool) -> some View {
        Text(s)
            .font(.system(size: 10.5, weight: .bold))
            .tracking(0.5)
            .foregroundStyle(Color.black)
            .padding(.horizontal, 9).padding(.vertical, 4)
            .background(Capsule().fill(Theme.accent))
    }
}

struct EmptyStateView: View {
    @ObservedObject var app = AppState.shared
    @State private var hover = false
    var body: some View {
        VStack(spacing: 18) {
            ZStack {
                RoundedRectangle(cornerRadius: 18)
                    .strokeBorder(style: StrokeStyle(lineWidth: 1.5, dash: [6, 5]))
                    .foregroundStyle(hover ? Theme.accent : Theme.textFaint)
                    .frame(width: 120, height: 92)
                Image(systemName: "photo.badge.plus").font(.system(size: 34, weight: .light)).foregroundStyle(Theme.textDim)
            }
            VStack(spacing: 6) {
                Text(tr("Drop an image here")).font(.system(size: 17, weight: .semibold)).foregroundStyle(Theme.text)
                Text(tr("JPG, PNG, HEIC, TIFF, RAW or .effector projects")).font(.system(size: 12)).foregroundStyle(Theme.textFaint)
            }
            HStack(spacing: 10) {
                Button { app.openPanel() } label: { Text(tr("Open Image…")) }.buttonStyle(PillButtonStyle(prominent: true))
                Button { app.pasteImage() } label: { Text(tr("Paste")) }.buttonStyle(PillButtonStyle())
            }
            Text("⌘O · ⌘V").font(.system(size: 11)).foregroundStyle(Theme.textFaint)
        }
        .onHover { hover = $0 }
    }
}

// MARK: - Metal canvas

struct CanvasView: NSViewRepresentable {
    @ObservedObject var app: AppState

    func makeNSView(context: Context) -> CanvasMTKView {
        let v = CanvasMTKView(frame: .zero, device: GPU.shared.device)
        v.app = app
        return v
    }

    func updateNSView(_ v: CanvasMTKView, context: Context) {
        v.texture = app.isCropping ? v.cropTexture(for: app) : (app.showOriginal ? app.original : app.output)
        v.original = app.original
        v.split = app.compareSplit && !app.isCropping && !app.showOriginal ? app.splitPos : nil
        v.needsDisplay = true
    }
}

final class CanvasMTKView: MTKView, MTKViewDelegate {
    weak var app: AppState?
    var texture: MTLTexture?
    var original: MTLTexture?
    /// Split before/after: fraction of the view width showing the original on the left.
    var split: Double?
    private var pipeline: MTLRenderPipelineState?
    private var cropTex: (UUID, MTLTexture)?
    private var lastDrag: NSPoint?

    override init(frame: CGRect, device: MTLDevice?) {
        super.init(frame: frame, device: device)
        colorPixelFormat = .bgra8Unorm
        colorspace = CGColorSpace(name: CGColorSpace.sRGB)
        enableSetNeedsDisplay = true
        isPaused = true
        framebufferOnly = true
        delegate = self
        layer?.isOpaque = false
        let d = MTLRenderPipelineDescriptor()
        d.vertexFunction = GPU.shared.library.makeFunction(name: "v_display")
        d.fragmentFunction = GPU.shared.library.makeFunction(name: "f_display")
        d.colorAttachments[0].pixelFormat = colorPixelFormat
        pipeline = try? GPU.shared.device.makeRenderPipelineState(descriptor: d)
    }

    required init(coder: NSCoder) { fatalError() }

    override var acceptsFirstResponder: Bool { true }
    override var isFlipped: Bool { true }

    func cropTexture(for app: AppState) -> MTLTexture? {
        guard let src = app.source else { return nil }
        if let c = cropTex, c.0 == src.id { return c.1 }
        let scale = min(1, 2400 / Double(max(src.width, src.height)))
        let t = src.makeTexture(crop: .full, scale: scale)
        cropTex = (src.id, t)
        return t
    }

    override func viewDidChangeBackingProperties() {
        super.viewDidChangeBackingProperties()
        app?.backingScale = Double(window?.backingScaleFactor ?? 2)
    }

    /// Image rect in view points.
    func imageRect() -> CGRect {
        guard let app, let src = app.source else { return .zero }
        let (w, h): (Int, Int) = app.isCropping ? (src.width, src.height) : src.croppedSize(app.state.crop)
        var z = app.displayZoom
        var pan = app.pan
        if app.isCropping {
            z = min(Double(bounds.width - 80) / Double(w), Double(bounds.height - 130) / Double(h))
            pan = CGSize(width: 0, height: -25)
        }
        let size = CGSize(width: Double(w) * z, height: Double(h) * z)
        return CGRect(x: bounds.midX - size.width / 2 + pan.width, y: bounds.midY - size.height / 2 + pan.height,
                      width: size.width, height: size.height)
    }

    func mtkView(_ view: MTKView, drawableSizeWillChange size: CGSize) {}

    func draw(in view: MTKView) {
        guard let pipeline, let rpd = currentRenderPassDescriptor, let drawable = currentDrawable else { return }
        let dark = effectiveAppearance.bestMatch(from: [.darkAqua, .aqua]) == .darkAqua
        let bg = AppState.shared.presenting ? 0.0 : (dark ? 0.085 : 0.86)
        rpd.colorAttachments[0].clearColor = MTLClearColor(red: bg, green: bg, blue: bg, alpha: 1)
        rpd.colorAttachments[0].loadAction = .clear
        let cb = GPU.shared.queue.makeCommandBuffer()!
        let enc = cb.makeRenderCommandEncoder(descriptor: rpd)!
        if let tex = texture {
            let s = Float(window?.backingScaleFactor ?? 2)
            let r = imageRect()
            // nearest filtering when zoomed in past 2 screen px per texture px
            let texPerScreen = Double(tex.width) / (r.width * Double(s))
            var u: [Float] = [Float(drawableSize.width), Float(drawableSize.height),
                              Float(r.minX) * s, Float(r.minY) * s, Float(r.width) * s, Float(r.height) * s,
                              8 * s, texPerScreen < 0.5 ? 1 : 0]
            enc.setRenderPipelineState(pipeline)
            enc.setVertexBytes(&u, length: u.count * 4, index: 0)
            enc.setFragmentBytes(&u, length: u.count * 4, index: 0)
            enc.setFragmentTexture(tex, index: 0)
            enc.drawPrimitives(type: .triangleStrip, vertexStart: 0, vertexCount: 4)
            if let split, let orig = original, orig.width == tex.width {
                // draw the original again, clipped to the left of the divider
                let w = Int(drawableSize.width), h = Int(drawableSize.height)
                let x = max(0, min(w, Int(Double(w) * split)))
                if x > 0 {
                    enc.setScissorRect(MTLScissorRect(x: 0, y: 0, width: x, height: h))
                    enc.setFragmentTexture(orig, index: 0)
                    enc.drawPrimitives(type: .triangleStrip, vertexStart: 0, vertexCount: 4)
                }
            }
        }
        enc.endEncoding()
        cb.present(drawable)
        cb.commit()
    }

    // MARK: Interaction

    private func zoom(by factor: Double, at p: NSPoint) {
        guard let app, !app.isCropping else { return }
        let old = app.displayZoom
        let new = min(32, max(0.02, old * factor))
        let center = CGPoint(x: bounds.midX + app.pan.width, y: bounds.midY + app.pan.height)
        let rel = CGPoint(x: (p.x - center.x) / old, y: (p.y - center.y) / old)
        let newCenter = CGPoint(x: p.x - rel.x * new, y: p.y - rel.y * new)
        app.pan = CGSize(width: newCenter.x - bounds.midX, height: newCenter.y - bounds.midY)
        app.setZoom(new)
    }

    override func magnify(with event: NSEvent) {
        zoom(by: 1 + event.magnification, at: convert(event.locationInWindow, from: nil))
    }

    override func scrollWheel(with event: NSEvent) {
        guard let app, !app.isCropping else { return }
        let p = convert(event.locationInWindow, from: nil)
        if event.modifierFlags.contains(.command) || !event.hasPreciseScrollingDeltas {
            let dy = event.hasPreciseScrollingDeltas ? event.scrollingDeltaY * 0.01 : event.scrollingDeltaY * 0.1
            zoom(by: exp(dy), at: p)
        } else {
            app.pan.width += event.scrollingDeltaX
            app.pan.height += event.scrollingDeltaY
            needsDisplay = true
        }
    }

    override func mouseDown(with event: NSEvent) {
        window?.makeFirstResponder(self)
        if event.clickCount == 2 { app?.pan = .zero; app?.setZoom(nil); return }
        lastDrag = event.locationInWindow
        NSCursor.closedHand.push()
    }

    override func mouseDragged(with event: NSEvent) {
        guard let app, let last = lastDrag, !app.isCropping else { return }
        let p = event.locationInWindow
        app.pan.width += p.x - last.x
        app.pan.height -= p.y - last.y
        lastDrag = p
        needsDisplay = true
    }

    override func mouseUp(with event: NSEvent) {
        if lastDrag != nil { NSCursor.pop() }
        lastDrag = nil
    }
}

/// Draggable before/after divider (original on the left).
struct SplitDivider: View {
    let width: CGFloat
    @ObservedObject var app = AppState.shared
    var body: some View {
        let x = width * app.splitPos
        ZStack(alignment: .topLeading) {
            GeometryReader { g in
                Rectangle().fill(Color.white.opacity(0.9)).frame(width: 1.5, height: g.size.height)
                    .shadow(color: .black.opacity(0.5), radius: 2)
                    .offset(x: x - 0.75)
                Circle().fill(Color.white).frame(width: 26, height: 26)
                    .shadow(color: .black.opacity(0.4), radius: 3)
                    .overlay(Image(systemName: "arrow.left.and.right").font(.system(size: 11, weight: .bold)).foregroundStyle(.black))
                    .offset(x: x - 13, y: g.size.height / 2 - 13)
                    .gesture(DragGesture(coordinateSpace: .named("canvasArea")).onChanged { v in
                        app.splitPos = min(0.98, max(0.02, v.location.x / max(1, width)))
                    })
                    .help(tr("Drag to compare"))
                Text(tr("BEFORE")).font(.system(size: 10, weight: .bold)).tracking(0.6)
                    .padding(.horizontal, 7).padding(.vertical, 3)
                    .background(Capsule().fill(.black.opacity(0.55))).foregroundStyle(.white)
                    .offset(x: max(8, x - 70), y: 12)
                Text(tr("AFTER")).font(.system(size: 10, weight: .bold)).tracking(0.6)
                    .padding(.horizontal, 7).padding(.vertical, 3)
                    .background(Capsule().fill(.black.opacity(0.55))).foregroundStyle(.white)
                    .offset(x: min(width - 60, x + 10), y: 12)
            }
        }
        .coordinateSpace(name: "canvasArea")
    }
}

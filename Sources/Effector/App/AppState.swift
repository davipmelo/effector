import Foundation
import SwiftUI
import AppKit
import Metal
import UniformTypeIdentifiers

extension UTType {
    static let effectorProject = UTType(exportedAs: "com.davimelo.effector.project", conformingTo: .package)
    static let effectorPreset = UTType(exportedAs: "com.davimelo.effector.preset", conformingTo: .json)
}

final class AppState: ObservableObject {
    static let shared = AppState()

    // Document
    @Published private(set) var source: SourceImage?
    @Published var state = DocumentState() { didSet { if state != oldValue { stateChanged(old: oldValue) } } }
    @Published var projectURL: URL?
    @Published var isDirty = false
    @Published var selectedID: UUID?
    @Published var soloID: UUID? { didSet { requestRender() } }

    // View
    @Published var showOriginal = false
    @Published var zoom: Double? = nil      // nil = fit; otherwise screen points per original pixel
    @Published var fitZoom: Double = 1      // computed by canvas
    @Published var pan: CGSize = .zero
    @Published var viewportSize: CGSize = .zero { didSet { updatePreviewScale() } }
    @Published var backingScale: Double = 2
    @Published private(set) var output: MTLTexture?
    @Published private(set) var original: MTLTexture?
    @Published private(set) var isRendering = false
    @Published var isCropping = false
    @Published var cropDraft = CropRect.full
    @Published var cropAspect: String = "free"
    @Published var showEffectBrowser = false
    @Published var showCommandPalette = false
    @Published var showExport = false
    @Published var isExporting = false
    @Published var sidebarTab = 0 // 0 layers, 1 presets
    @Published var effectThumbs: [String: CGImage] = [:]
    @Published var toast: String?

    // Undo
    private var undoStack: [DocumentState] = []
    private var redoStack: [DocumentState] = []
    private var lastCoalesceKey: String?
    private var lastCoalesceTime = Date.distantPast
    private var suppressUndo = false
    var canUndo: Bool { !undoStack.isEmpty }
    var canRedo: Bool { !redoStack.isEmpty }

    // Rendering
    private let renderer = Renderer()
    private let renderQueue = DispatchQueue(label: "effector.render", qos: .userInteractive)
    private var renderPending = false
    private var renderRunning = false
    private(set) var previewScale: Double = 0.25
    private var interactiveUntil = Date.distantPast
    private var refineWork: DispatchWorkItem?
    private var sourceTexCache: (key: Int, tex: MTLTexture)?

    private init() {}

    // MARK: - Document lifecycle

    var hasImage: Bool { source != nil }

    func openImage(_ img: SourceImage) {
        source = img
        suppressUndo = true
        state = DocumentState()
        suppressUndo = false
        undoStack.removeAll(); redoStack.removeAll()
        projectURL = nil
        isDirty = false
        selectedID = nil
        soloID = nil
        zoom = nil
        pan = .zero
        sourceTexCache = nil
        renderer.clearCache()
        updatePreviewScale(force: true)
        regenerateThumbnails()
        objectWillChange.send()
    }

    func open(url: URL) {
        do {
            if url.pathExtension.lowercased() == "effector" {
                try openProject(url)
            } else {
                openImage(try SourceImage.load(url: url))
            }
            NSDocumentController.shared.noteNewRecentDocumentURL(url)
        } catch {
            alert(tr("Could not open file"), error.localizedDescription)
        }
    }

    func openPanel() {
        let p = NSOpenPanel()
        p.allowedContentTypes = [.image, .rawImage, .effectorProject]
        p.allowsMultipleSelection = false
        p.treatsFilePackagesAsDirectories = false
        if p.runModal() == .OK, let u = p.url { open(url: u) }
    }

    func pasteImage() {
        if let s = SourceImage.fromPasteboard() { openImage(s) } else { flash(tr("No image on the clipboard")) }
    }

    // MARK: - Editing & undo

    /// Applies an edit. Edits with the same `coalesce` key in quick succession share one undo step.
    func edit(_ coalesce: String? = nil, _ body: (inout DocumentState) -> Void) {
        var s = state
        body(&s)
        guard s != state else { return }
        let now = Date()
        let merge = coalesce != nil && coalesce == lastCoalesceKey && now.timeIntervalSince(lastCoalesceTime) < 1.0
        if !merge {
            undoStack.append(state)
            if undoStack.count > 300 { undoStack.removeFirst() }
            redoStack.removeAll()
        }
        lastCoalesceKey = coalesce
        lastCoalesceTime = now
        if coalesce != nil { interactiveUntil = now.addingTimeInterval(0.25) }
        suppressUndo = true
        state = s
        suppressUndo = false
    }

    func undo() {
        guard let prev = undoStack.popLast() else { return }
        redoStack.append(state)
        lastCoalesceKey = nil
        suppressUndo = true; state = prev; suppressUndo = false
        if let id = selectedID, state.layers.find(id) == nil { selectedID = nil }
    }

    func redo() {
        guard let next = redoStack.popLast() else { return }
        undoStack.append(state)
        lastCoalesceKey = nil
        suppressUndo = true; state = next; suppressUndo = false
    }

    private func stateChanged(old: DocumentState) {
        isDirty = true
        if old.crop != state.crop { sourceTexCache = nil; updatePreviewScale(force: true) }
        requestRender()
    }

    // MARK: - Layer operations

    var selectedLayer: LayerNode? { selectedID.flatMap { state.layers.find($0) } }

    func addEffect(_ id: String) {
        guard hasImage else { return }
        let node = LayerNode.effect(id)
        insertAboveSelection(node)
    }

    func insertAboveSelection(_ node: LayerNode) {
        edit { s in
            if let sel = selectedID, let loc = s.layers.location(of: sel) {
                if let n = s.layers.find(sel), n.isGroup, n.expanded {
                    s.layers.insert(node, parent: sel, index: 0)
                } else {
                    s.layers.insert(node, parent: loc.parent, index: loc.index)
                }
            } else {
                s.layers.insert(node, at: 0)
            }
        }
        selectedID = node.id
    }

    func updateLayer(_ id: UUID, coalesce: String? = nil, _ body: (inout LayerNode) -> Void) {
        edit(coalesce) { $0.layers.update(id, body) }
    }

    func setParam(_ id: UUID, _ key: String, _ value: ParamValue, coalesce: Bool = true) {
        updateLayer(id, coalesce: coalesce ? "\(id)-\(key)" : nil) { $0.params[key] = value }
    }

    func deleteSelected() {
        guard let id = selectedID else { return }
        let loc = state.layers.location(of: id)
        edit { $0.layers.remove(id) }
        if soloID == id { soloID = nil }
        // select a neighbour
        if let loc {
            let siblings = loc.parent.flatMap { state.layers.find($0)?.children } ?? state.layers
            if siblings.isEmpty { selectedID = loc.parent } else { selectedID = siblings[min(loc.index, siblings.count - 1)].id }
        } else { selectedID = nil }
    }

    func duplicateSelected() {
        guard let id = selectedID, let n = state.layers.find(id), let loc = state.layers.location(of: id) else { return }
        var copy = n.reidentified()
        copy.name = n.name + " " + tr("copy")
        edit { $0.layers.insert(copy, parent: loc.parent, index: loc.index) }
        selectedID = copy.id
    }

    func groupSelected() {
        guard hasImage else { return }
        guard let id = selectedID, let n = state.layers.find(id), let loc = state.layers.location(of: id) else {
            let g = LayerNode.group(tr("Group"), [])
            edit { $0.layers.insert(g, at: 0) }
            selectedID = g.id
            return
        }
        let g = LayerNode.group(tr("Group"), [n])
        edit { s in
            s.layers.remove(id)
            s.layers.insert(g, parent: loc.parent, index: loc.index)
        }
        selectedID = g.id
    }

    func ungroupSelected() {
        guard let id = selectedID, let g = state.layers.find(id), g.isGroup, let loc = state.layers.location(of: id) else { return }
        edit { s in
            s.layers.remove(id)
            for (i, c) in g.children.enumerated() {
                var c = c
                if g.opacity < 1 { c.opacity *= g.opacity }
                s.layers.insert(c, parent: loc.parent, index: loc.index + i)
            }
        }
        selectedID = g.children.first?.id
    }

    func toggleVisibility(_ id: UUID) {
        updateLayer(id) { $0.enabled.toggle() }
    }

    func resetParams(_ id: UUID) {
        guard let n = state.layers.find(id), !n.isGroup else { return }
        let fresh = LayerNode.effect(n.effectID)
        updateLayer(id) { $0.params = fresh.params }
    }

    enum DropPosition { case above, below, inside }

    /// Moves `id` relative to `target`.
    func move(_ id: UUID, to target: UUID, _ pos: DropPosition) {
        guard id != target, !state.layers.contains(target, inside: id) else { return }
        edit { s in
            guard let node = s.layers.remove(id) else { return }
            guard let loc = s.layers.location(of: target) else { s.layers.insert(node, at: 0); return }
            switch pos {
            case .above: s.layers.insert(node, parent: loc.parent, index: loc.index)
            case .below: s.layers.insert(node, parent: loc.parent, index: loc.index + 1)
            case .inside: s.layers.insert(node, parent: target, index: 0)
            }
        }
        if pos == .inside { updateLayerSilently(target) { $0.expanded = true } }
    }

    func moveToRootBottom(_ id: UUID) {
        edit { s in
            guard let n = s.layers.remove(id) else { return }
            s.layers.append(n)
        }
    }

    /// Move selected layer up / down among siblings (keyboard).
    func nudgeSelected(_ delta: Int) {
        guard let id = selectedID, let loc = state.layers.location(of: id) else { return }
        edit { s in
            guard let n = s.layers.remove(id) else { return }
            s.layers.insert(n, parent: loc.parent, index: loc.index + delta)
        }
    }

    /// Changes that should not create undo steps (UI state like expansion).
    func updateLayerSilently(_ id: UUID, _ body: (inout LayerNode) -> Void) {
        var s = state
        s.layers.update(id, body)
        suppressUndo = true
        let dirty = isDirty
        state = s
        isDirty = dirty
        suppressUndo = false
    }

    func applyPreset(_ preset: Preset) {
        guard hasImage else { return }
        var g = LayerNode.group(preset.name, preset.layers.map { $0.reidentified() })
        g.expanded = false
        edit { $0.layers.insert(g, at: 0) }
        selectedID = g.id
        flash(tr("Preset applied: %@", preset.name))
    }

    // MARK: - Rendering

    func requestRender() {
        renderPending = true
        kickRender()
    }

    private func currentScale() -> Double {
        var s = previewScale
        if Date() < interactiveUntil && s > 0.3 {
            s *= 0.5
            refineWork?.cancel()
            let w = DispatchWorkItem { [weak self] in self?.requestRender() }
            refineWork = w
            DispatchQueue.main.asyncAfter(deadline: .now() + 0.3, execute: w)
        }
        return s
    }

    private func kickRender() {
        guard renderPending, !renderRunning, let src = source else { return }
        renderPending = false
        renderRunning = true
        isRendering = true
        var layers = state.layers
        if let solo = soloID, let n = layers.find(solo) { var n = n; n.enabled = true; n.opacity = 1; n.blend = .normal; layers = [n] }
        let crop = state.crop
        let scale = currentScale()
        renderQueue.async { [weak self] in
            guard let self else { return }
            let srcTex = self.sourceTexture(src, crop: crop, scale: scale)
            var h = Hasher(); h.combine(src.id); h.combine(crop)
            let out = self.renderer.render(layers: layers, source: srcTex, sourceKey: h.finalize(), scale: scale)
            DispatchQueue.main.async {
                self.original = srcTex
                self.output = out
                self.renderRunning = false
                self.isRendering = self.renderPending
                self.kickRender()
            }
        }
    }

    private func sourceTexture(_ src: SourceImage, crop: CropRect, scale: Double) -> MTLTexture {
        var h = Hasher(); h.combine(src.id); h.combine(crop); h.combine(scale)
        let key = h.finalize()
        if let c = sourceTexCache, c.key == key { return c.tex }
        let t = src.makeTexture(crop: crop, scale: scale)
        sourceTexCache = (key, t)
        return t
    }

    /// Picks a preview resolution matching what is on screen (quantized to avoid churn).
    func updatePreviewScale(force: Bool = false) {
        guard let src = source, viewportSize.width > 10 else { return }
        let (cw, ch) = src.croppedSize(state.crop)
        let longSide = Double(max(cw, ch))
        let fit = min(Double(viewportSize.width) / Double(cw), Double(viewportSize.height) / Double(ch))
        fitZoom = fit
        let z = zoom ?? fit
        var needed = z * backingScale
        needed = min(1, max(needed, 600 / longSide))
        // quantize to 1/8 steps in log2 space
        let q = pow(2, (log2(needed) * 4).rounded(.up) / 4)
        let newScale = min(1, q)
        if force || abs(newScale - previewScale) > 1e-6 {
            previewScale = newScale
            requestRender()
        }
    }

    var displayZoom: Double { zoom ?? fitZoom }

    func setZoom(_ z: Double?) {
        if let z { zoom = min(32, max(0.02, z)) } else { zoom = nil; pan = .zero }
        updatePreviewScale()
    }

    func zoomIn() { setZoom(displayZoom * 1.25) }
    func zoomOut() { setZoom(displayZoom / 1.25) }
    func zoomActual() { setZoom(1 / backingScale) }

    // MARK: - Thumbnails

    func regenerateThumbnails() {
        guard let src = source else { return }
        let crop = state.crop
        DispatchQueue.global(qos: .utility).async {
            let (cw, ch) = src.croppedSize(crop)
            let s = 220.0 / Double(max(cw, ch))
            let tex = src.makeTexture(crop: crop, scale: s)
            let r = Renderer(useCache: false)
            var thumbs: [String: CGImage] = [:]
            // Thumbnails emulate viewing a ~1000px image so size params are visible.
            let effScale = 220.0 / 1000.0
            for eff in EffectRegistry.all {
                let node = LayerNode.effect(eff.id)
                let out = r.render(layers: [node], source: tex, sourceKey: 0, scale: effScale)
                if let cg = ImageExport.cgImage(out) { thumbs[eff.id] = cg }
            }
            DispatchQueue.main.async { self.effectThumbs = thumbs }
        }
    }

    func renderThumbnail(layers: [LayerNode], size: Double = 200) -> CGImage? {
        guard let src = source else { return nil }
        let (cw, ch) = src.croppedSize(state.crop)
        let s = size / Double(max(cw, ch))
        let tex = src.makeTexture(crop: state.crop, scale: s)
        let out = Renderer(useCache: false).render(layers: layers, source: tex, sourceKey: 0, scale: size / 1000)
        return ImageExport.cgImage(out)
    }

    // MARK: - Export

    func exportSize(scale: Int) -> (Int, Int) {
        guard let src = source else { return (0, 0) }
        let (w, h) = src.croppedSize(state.crop)
        return (w * scale, h * scale)
    }

    func renderFull(scale: Int) -> MTLTexture? {
        guard let src = source else { return nil }
        let tex = src.makeTexture(crop: state.crop, scale: Double(scale))
        return Renderer(useCache: false).render(layers: state.layers, source: tex, sourceKey: 0, scale: Double(scale), isExport: true)
    }

    func export(format: ImageExport.Format, quality: Double, scale: Int) {
        guard let src = source else { return }
        let panel = NSSavePanel()
        panel.allowedContentTypes = [format == .png ? .png : .jpeg]
        let base = (projectURL?.deletingPathExtension().lastPathComponent)
            ?? (src.fileName as NSString).deletingPathExtension
        panel.nameFieldStringValue = base + (scale > 1 ? "@\(scale)x" : "") + "." + format.rawValue
        guard panel.runModal() == .OK, let url = panel.url else { return }
        isExporting = true
        let layersHaveAlpha = src.hasAlpha
        DispatchQueue.global(qos: .userInitiated).async {
            var err: Error?
            autoreleasepool {
                if let tex = self.renderFull(scale: scale) {
                    do { try ImageExport.write(tex, to: url, format: format, quality: quality, keepAlpha: layersHaveAlpha) }
                    catch { err = error }
                }
            }
            DispatchQueue.main.async {
                self.isExporting = false
                self.showExport = false
                if let err { self.alert(tr("Export failed"), err.localizedDescription) }
                else { self.flash(tr("Exported %@", url.lastPathComponent)) }
            }
        }
    }

    func copyResult() {
        guard hasImage else { return }
        isExporting = true
        DispatchQueue.global(qos: .userInitiated).async {
            let tex = self.renderFull(scale: 1)
            let cg = tex.flatMap { ImageExport.cgImage($0) }
            DispatchQueue.main.async {
                self.isExporting = false
                guard let cg else { return }
                let img = NSImage(cgImage: cg, size: NSSize(width: cg.width, height: cg.height))
                NSPasteboard.general.clearContents()
                NSPasteboard.general.writeObjects([img])
                self.flash(tr("Image copied to clipboard"))
            }
        }
    }

    // MARK: - Project files

    func save(as saveAs: Bool = false) {
        guard let src = source else { return }
        var url = projectURL
        if url == nil || saveAs {
            let p = NSSavePanel()
            p.allowedContentTypes = [.effectorProject]
            p.nameFieldStringValue = (src.fileName as NSString).deletingPathExtension + ".effector"
            guard p.runModal() == .OK, let u = p.url else { return }
            url = u
        }
        guard let url else { return }
        do {
            let manifest = ProjectManifest(originalFileName: src.fileName, state: state)
            let enc = JSONEncoder()
            enc.outputFormatting = [.prettyPrinted, .sortedKeys]
            let json = try enc.encode(manifest)
            let ext = (src.fileName as NSString).pathExtension.isEmpty ? "png" : (src.fileName as NSString).pathExtension
            var files: [String: FileWrapper] = [
                "document.json": FileWrapper(regularFileWithContents: json),
                "original.\(ext.lowercased())": FileWrapper(regularFileWithContents: src.data),
            ]
            if let thumb = renderThumbnail(layers: state.layers, size: 512) {
                let rep = NSBitmapImageRep(cgImage: thumb)
                if let png = rep.representation(using: .png, properties: [:]) {
                    files["thumbnail.png"] = FileWrapper(regularFileWithContents: png)
                }
            }
            let wrapper = FileWrapper(directoryWithFileWrappers: files)
            try wrapper.write(to: url, options: .atomic, originalContentsURL: nil)
            projectURL = url
            isDirty = false
            NSDocumentController.shared.noteNewRecentDocumentURL(url)
            flash(tr("Saved"))
        } catch {
            alert(tr("Could not save"), error.localizedDescription)
        }
    }

    func openProject(_ url: URL) throws {
        let w = try FileWrapper(url: url, options: .immediate)
        guard let files = w.fileWrappers,
              let json = files["document.json"]?.regularFileContents,
              let (name, orig) = files.first(where: { $0.key.hasPrefix("original.") }),
              let data = orig.regularFileContents else { throw SourceImage.LoadError.unreadable }
        let manifest = try JSONDecoder().decode(ProjectManifest.self, from: json)
        let img = try SourceImage.load(data: data, fileName: manifest.originalFileName,
                                       typeHint: UTType(filenameExtension: (name as NSString).pathExtension))
        openImage(img)
        suppressUndo = true
        state = manifest.state
        suppressUndo = false
        projectURL = url
        isDirty = false
        sourceTexCache = nil
        updatePreviewScale(force: true)
    }

    // MARK: - Crop

    func beginCrop() {
        guard hasImage else { return }
        cropDraft = state.crop
        isCropping = true
        showOriginal = false
    }

    func commitCrop() {
        let c = cropDraft
        isCropping = false
        edit { $0.crop = c }
        zoom = nil
        regenerateThumbnails()
    }

    func cancelCrop() { isCropping = false }

    func resetCrop() { cropDraft = .full }

    // MARK: - Messages

    func flash(_ s: String) {
        toast = s
        let current = s
        DispatchQueue.main.asyncAfter(deadline: .now() + 2.2) { [weak self] in
            if self?.toast == current { self?.toast = nil }
        }
    }

    func alert(_ title: String, _ msg: String) {
        let a = NSAlert()
        a.messageText = title
        a.informativeText = msg
        a.runModal()
    }

    /// Returns false if the user cancelled.
    func confirmDiscardChanges() -> Bool {
        guard isDirty, hasImage, !state.layers.isEmpty else { return true }
        let a = NSAlert()
        a.messageText = tr("Save changes to this project?")
        a.informativeText = tr("Your changes will be lost if you don't save them.")
        a.addButton(withTitle: tr("Save"))
        a.addButton(withTitle: tr("Don't Save"))
        a.addButton(withTitle: tr("Cancel"))
        switch a.runModal() {
        case .alertFirstButtonReturn: save(); return !isDirty
        case .alertSecondButtonReturn: return true
        default: return false
        }
    }
}

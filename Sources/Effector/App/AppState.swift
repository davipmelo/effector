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
    /// Primary selection (shown in the inspector). Setting it directly collapses a multi-selection.
    @Published var selectedID: UUID? {
        didSet { if !extendingSelection { selectedIDs = selectedID.map { [$0] } ?? [] } }
    }
    /// All selected layers (shift/⌘-click).
    @Published var selectedIDs: Set<UUID> = []
    private var extendingSelection = false
    private var selectionAnchor: UUID?
    @Published var soloID: UUID? { didSet { requestRender() } }
    /// Temporary layers drawn on top while hovering / browsing effects and presets (not part of the document).
    @Published var previewLayers: [LayerNode]? { didSet { requestRender() } }

    // View
    @Published var showOriginal = false
    /// Split before/after view with a draggable divider.
    @Published var compareSplit = false { didSet { if compareSplit { showOriginal = false } } }
    @Published var splitPos = 0.5
    /// Full-screen image preview (F): hides every panel.
    @Published private(set) var presenting = false
    private var enteredFullScreen = false
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
    @Published var showEffectBrowser = false { didSet { if !showEffectBrowser { previewLayers = nil }; if showEffectBrowser { showPresetBrowser = false } } }
    @Published var showPresetBrowser = false { didSet { if !showPresetBrowser { previewLayers = nil }; if showPresetBrowser { showEffectBrowser = false } } }
    @Published var showCommandPalette = false
    @Published var showExport = false
    @Published var isExporting = false
    @Published var effectThumbs: [String: CGImage] = [:]
    @Published var toast: String?

    // Undo
    private var undoStack: [DocumentState] = []
    private var redoStack: [DocumentState] = []
    private var lastCoalesceKey: String?
    private var lastCoalesceTime = Date.distantPast
    private var suppressUndo = false
    private var browsing = false
    private var browseUndoPushed = false
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
        previewLayers = nil
        if !restoringSession && AppSettings.shared.reopenLastSession { writeSessionOriginal(img) }
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
        scheduleSessionSave()
    }

    // MARK: - Session (reopened on next launch)

    private struct SessionFile: Codable {
        var fileName: String
        var originalFile: String
        var state: DocumentState
        var projectPath: String?
        var dirty: Bool
    }
    private var restoringSession = false
    private var sessionWork: DispatchWorkItem?
    private let sessionQueue = DispatchQueue(label: "effector.session", qos: .utility)

    private var sessionDir: URL {
        let base = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0]
        let d = base.appendingPathComponent("EFFECTOR/Session", isDirectory: true)
        try? FileManager.default.createDirectory(at: d, withIntermediateDirectories: true)
        return d
    }

    private func writeSessionOriginal(_ src: SourceImage) {
        let dir = sessionDir
        let ext = (src.fileName as NSString).pathExtension.isEmpty ? "png" : (src.fileName as NSString).pathExtension.lowercased()
        let data = src.data
        sessionQueue.async {
            let fm = FileManager.default
            for f in (try? fm.contentsOfDirectory(at: dir, includingPropertiesForKeys: nil)) ?? [] { try? fm.removeItem(at: f) }
            try? data.write(to: dir.appendingPathComponent("original." + ext), options: .atomic)
        }
    }

    func scheduleSessionSave() {
        guard !restoringSession, AppSettings.shared.reopenLastSession else { return }
        sessionWork?.cancel()
        let w = DispatchWorkItem { [weak self] in self?.saveSession() }
        sessionWork = w
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.8, execute: w)
    }

    func saveSession(sync: Bool = false) {
        guard let src = source else { return }
        let ext = (src.fileName as NSString).pathExtension.isEmpty ? "png" : (src.fileName as NSString).pathExtension.lowercased()
        let f = SessionFile(fileName: src.fileName, originalFile: "original." + ext, state: state,
                            projectPath: projectURL?.path, dirty: isDirty)
        let url = sessionDir.appendingPathComponent("session.json")
        let work = { if let d = try? JSONEncoder().encode(f) { try? d.write(to: url, options: .atomic) } }
        if sync { sessionQueue.sync(execute: work) } else { sessionQueue.async(execute: work) }
    }

    /// Reopens the image and layers that were open when the app last quit.
    @discardableResult
    func restoreSession() -> Bool {
        guard AppSettings.shared.reopenLastSession, !hasImage else { return false }
        let dir = sessionDir
        guard let d = try? Data(contentsOf: dir.appendingPathComponent("session.json")),
              let f = try? JSONDecoder().decode(SessionFile.self, from: d),
              let data = try? Data(contentsOf: dir.appendingPathComponent(f.originalFile)),
              let img = try? SourceImage.load(data: data, fileName: f.fileName,
                                              typeHint: UTType(filenameExtension: (f.originalFile as NSString).pathExtension)) else { return false }
        restoringSession = true
        openImage(img)
        suppressUndo = true
        state = f.state
        suppressUndo = false
        projectURL = f.projectPath.map { URL(fileURLWithPath: $0) }
        isDirty = f.dirty
        sourceTexCache = nil
        updatePreviewScale(force: true)
        regenerateThumbnails()
        restoringSession = false
        return true
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
        var merge = coalesce != nil && coalesce == lastCoalesceKey && now.timeIntervalSince(lastCoalesceTime) < 1.0
        if browsing {
            // all changes made while browsing a picker form one undo step
            merge = browseUndoPushed
            browseUndoPushed = true
            interactiveUntil = now.addingTimeInterval(0.2)
        }
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

    /// Picker browsing session: every change until `endBrowse` is a single undo step.
    func beginBrowse() { browsing = true; browseUndoPushed = false }
    func endBrowse() { browsing = false; browseUndoPushed = false; lastCoalesceKey = nil }

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
        scheduleSessionSave()
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

    func addLUTLayer(_ l: LUTData) {
        var p = Params()
        p["lut"] = .lut(l)
        var node = LayerNode.effect("lut", params: p)
        node.name = l.name
        insertAboveSelection(node)
        flash(tr("LUT added: %@", l.name))
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

    // MARK: Selection

    var isMultiSelection: Bool { selectedIDs.count > 1 }

    /// Click on a layer row: plain = select, ⇧ = range, ⌘ = toggle.
    func click(_ id: UUID) {
        let f = NSEvent.modifierFlags
        if f.contains(.shift), let anchor = selectionAnchor ?? selectedID {
            let order = state.layers.flattened().map(\.node.id)
            if let a = order.firstIndex(of: anchor), let b = order.firstIndex(of: id) {
                extendingSelection = true
                selectedIDs = Set(order[min(a, b)...max(a, b)])
                selectedID = id
                extendingSelection = false
                return
            }
        }
        if f.contains(.command) {
            extendingSelection = true
            if selectedIDs.contains(id) {
                selectedIDs.remove(id)
                if selectedID == id { selectedID = order(selectedIDs).first }
            } else {
                selectedIDs.insert(id)
                selectedID = id
            }
            extendingSelection = false
            selectionAnchor = id
            return
        }
        selectedID = id
        selectionAnchor = id
    }

    func selectAllLayers() {
        let all = state.layers.flattened().map(\.node.id)
        extendingSelection = true
        selectedIDs = Set(all)
        selectedID = all.first
        extendingSelection = false
    }

    /// Every node id, depth-first, top-first.
    private func allIDs(_ nodes: [LayerNode]) -> [UUID] { nodes.flatMap { [$0.id] + allIDs($0.children) } }
    private func order(_ ids: Set<UUID>) -> [UUID] { allIDs(state.layers).filter { ids.contains($0) } }

    /// Selected nodes that are not inside another selected node, top-first.
    var selectionRoots: [UUID] {
        let sel = selectedIDs.isEmpty ? Set(selectedID.map { [$0] } ?? []) : selectedIDs
        return order(sel).filter { id in !sel.contains { other in other != id && state.layers.contains(id, inside: other) } }
    }

    func deleteSelected() {
        let roots = selectionRoots
        guard let first = roots.first else { return }
        let loc = state.layers.location(of: first)
        edit { s in for id in roots { _ = s.layers.remove(id) } }
        if let s = soloID, roots.contains(s) || roots.contains(where: { state.layers.contains(s, inside: $0) }) { soloID = nil }
        if let loc {
            let siblings = loc.parent.flatMap { state.layers.find($0)?.children } ?? state.layers
            if siblings.isEmpty { selectedID = loc.parent } else { selectedID = siblings[min(loc.index, siblings.count - 1)].id }
        } else { selectedID = nil }
    }

    func duplicateSelected() {
        let roots = selectionRoots
        guard !roots.isEmpty else { return }
        var copies: [UUID] = []
        edit { s in
            for id in roots {
                guard let n = s.layers.find(id), let loc = s.layers.location(of: id) else { continue }
                var copy = n.reidentified()
                copy.name = n.name + " " + tr("copy")
                s.layers.insert(copy, parent: loc.parent, index: loc.index)
                copies.append(copy.id)
            }
        }
        extendingSelection = true
        selectedIDs = Set(copies)
        selectedID = copies.first
        extendingSelection = false
    }

    /// Groups the selection (all selected layers go into one new group, keeping their order).
    func groupSelected() {
        guard hasImage else { return }
        let roots = selectionRoots
        guard let first = roots.first, let loc = state.layers.location(of: first) else {
            let g = LayerNode.group(tr("Group"), [])
            edit { $0.layers.insert(g, at: 0) }
            selectedID = g.id
            return
        }
        let nodes = roots.compactMap { state.layers.find($0) }
        let g = LayerNode.group(tr("Group"), nodes)
        edit { s in
            for id in roots { _ = s.layers.remove(id) }
            let parentOK = loc.parent.map { s.layers.find($0) != nil } ?? true
            if parentOK {
                let count = loc.parent.flatMap { s.layers.find($0)?.children.count } ?? s.layers.count
                s.layers.insert(g, parent: loc.parent, index: min(loc.index, count))
            } else {
                s.layers.insert(g, at: 0)
            }
        }
        selectedID = g.id
    }

    /// Applies a change to every selected layer as one undo step.
    func updateSelected(coalesce: String? = nil, _ body: (inout LayerNode) -> Void) {
        let ids = selectionRoots
        edit(coalesce) { s in for id in ids { s.layers.update(id, body) } }
    }

    func toggleSelectedVisibility() {
        let ids = selectionRoots
        guard let first = ids.first.flatMap({ state.layers.find($0) }) else { return }
        let on = !first.enabled
        edit { s in for id in ids { s.layers.update(id) { $0.enabled = on } } }
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

    /// Resets the layer to the effect defaults and applies a look's values.
    func applyLook(_ id: UUID, _ look: EffectLook) {
        guard let n = state.layers.find(id), !n.isGroup else { return }
        var p = LayerNode.effect(n.effectID).params
        for (k, v) in look.values { if let pv = paramValue(v) { p[k] = pv } }
        // keep the seed so browsing looks doesn't reshuffle noise
        if let seed = n.params["seed"] { p["seed"] = seed }
        p["_look"] = .text(look.id)
        updateLayer(id) { $0.params = p }
    }

    enum DropPosition { case above, below, inside }

    /// Moves `id` relative to `target`.
    func move(_ id: UUID, to target: UUID, _ pos: DropPosition) { move([id], to: target, pos) }

    /// Moves several layers (kept in order) relative to `target`.
    func move(_ ids: [UUID], to target: UUID, _ pos: DropPosition) {
        let ids = ids.filter { $0 != target && !state.layers.contains(target, inside: $0) }
        guard !ids.isEmpty else { return }
        edit { s in
            let nodes = ids.compactMap { s.layers.remove($0) }
            guard let loc = s.layers.location(of: target) else { for n in nodes.reversed() { s.layers.insert(n, at: 0) }; return }
            for (i, n) in nodes.enumerated() {
                switch pos {
                case .above: s.layers.insert(n, parent: loc.parent, index: loc.index + i)
                case .below: s.layers.insert(n, parent: loc.parent, index: loc.index + 1 + i)
                case .inside: s.layers.insert(n, parent: target, index: i)
                }
            }
        }
        if pos == .inside { updateLayerSilently(target) { $0.expanded = true } }
    }

    func moveToRootBottom(_ ids: [UUID]) {
        edit { s in
            let nodes = ids.compactMap { s.layers.remove($0) }
            s.layers.append(contentsOf: nodes)
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

    /// Saves the whole layer stack (order, groups, parameters, blend, opacity) as a user preset.
    func saveStackAsPreset() {
        guard hasImage, !state.layers.isEmpty else { return }
        guard let n = promptText(tr("Save as Preset"), tr("Name"), tr("My Preset")) else { return }
        PresetStore.shared.add(name: n, layers: state.layers, thumbnail: renderThumbnail(layers: state.layers, size: 240))
        flash(tr("Preset saved: %@", n))
    }

    private var hoverWork: DispatchWorkItem?
    /// Temporary preview while hovering / browsing (nil clears it, with a short delay so moving
    /// between cards doesn't flicker back to the original).
    func hoverPreview(_ layers: [LayerNode]?) {
        guard AppSettings.shared.hoverPreview || layers == nil else { return }
        hoverWork?.cancel()
        let w = DispatchWorkItem { [weak self] in self?.previewLayers = layers }
        hoverWork = w
        DispatchQueue.main.asyncAfter(deadline: .now() + (layers == nil ? 0.18 : 0.06), execute: w)
    }

    func presetPreviewLayers(_ p: Preset) -> [LayerNode] {
        var g = LayerNode.group(p.displayName, p.layers)
        g.expanded = false
        return [g]
    }

    func applyPreset(_ preset: Preset) {
        guard hasImage else { return }
        var g = LayerNode.group(preset.displayName, preset.layers.map { $0.reidentified() })
        g.expanded = false
        edit { $0.layers.insert(g, at: 0) }
        selectedID = g.id
        flash(tr("Preset applied: %@", preset.displayName))
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
        if let pv = previewLayers { layers = pv + layers }
        let crop = state.crop
        let scale = currentScale()
        renderQueue.async { [weak self] in
            guard let self else { return }
            let srcTex = self.sourceTexture(src, crop: crop, scale: scale)
            var h = Hasher(); h.combine(src.id); h.combine(crop)
            let out = self.renderer.render(layers: layers, source: srcTex, sourceKey: h.finalize())
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

    // MARK: Randomize / copy & paste layers

    /// Rolls new random values for a layer's sliders, menus and seed (one undo step).
    func randomizeParams(_ id: UUID) {
        guard let n = state.layers.find(id), !n.isGroup, let eff = EffectRegistry.effect(n.effectID) else { return }
        var p = n.params
        for d in eff.params {
            switch d.kind {
            case .slider(let mn, let mx, _, let step, _, let curve):
                var v = mn + (mx - mn) * pow(Double.random(in: 0...1), curve)
                if step > 0 { v = (v / step).rounded() * step }
                p[d.key] = .number(min(mx, max(mn, v)))
            case .choice(let o, _), .segmented(let o, _):
                if let r = o.randomElement() { p[d.key] = .choice(r.id) }
            case .seed:
                p[d.key] = .number(Double(Int.random(in: 1...9999)))
            default: break
            }
        }
        p["_look"] = nil
        updateLayer(id) { $0.params = p }
    }

    static let layersPasteboardType = NSPasteboard.PasteboardType("com.davimelo.effector.layers")

    func copyLayers() {
        let nodes = selectionRoots.compactMap { state.layers.find($0) }
        guard !nodes.isEmpty, let d = try? JSONEncoder().encode(nodes) else { return }
        NSPasteboard.general.clearContents()
        NSPasteboard.general.setData(d, forType: AppState.layersPasteboardType)
        flash(tr("%d layers copied", nodes.count))
    }

    func pasteLayers() {
        guard hasImage, let d = NSPasteboard.general.data(forType: AppState.layersPasteboardType),
              let nodes = try? JSONDecoder().decode([LayerNode].self, from: d) else { flash(tr("No layers to paste")); return }
        let fresh = nodes.map { $0.reidentified() }
        edit { s in
            var index = 0
            var parent: UUID? = nil
            if let sel = selectedID, let loc = s.layers.location(of: sel) { parent = loc.parent; index = loc.index }
            for (i, n) in fresh.enumerated() { s.layers.insert(n, parent: parent, index: index + i) }
        }
        extendingSelection = true
        selectedIDs = Set(fresh.map(\.id))
        selectedID = fresh.first?.id
        extendingSelection = false
        flash(tr("%d layers pasted", fresh.count))
    }

    func togglePresentation() {
        guard hasImage || presenting else { return }
        showEffectBrowser = false; showPresetBrowser = false; showCommandPalette = false
        presenting.toggle()
        let w = NSApp.windows.first { $0.identifier?.rawValue.hasPrefix("main") == true } ?? NSApp.mainWindow
        if presenting {
            zoom = nil; pan = .zero
            if let w, !w.styleMask.contains(.fullScreen) { w.toggleFullScreen(nil); enteredFullScreen = true }
            flash(tr("Full screen preview · F or ⎋ to go back"))
        } else {
            if enteredFullScreen, let w {
                // the enter animation may still be running: retry until the window is really full screen
                func leave(_ tries: Int) {
                    guard !presenting else { return }
                    if w.styleMask.contains(.fullScreen) { w.toggleFullScreen(nil) }
                    else if tries > 0 { DispatchQueue.main.asyncAfter(deadline: .now() + 0.4) { leave(tries - 1) } }
                }
                leave(4)
            }
            enteredFullScreen = false
            zoom = nil
        }
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
                let out = r.render(layers: [node], source: tex, sourceKey: 0)
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
        let out = Renderer(useCache: false).render(layers: layers, source: tex, sourceKey: 0)
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
        return Renderer(useCache: false).render(layers: state.layers, source: tex, sourceKey: 0, isExport: true)
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
            try writeProject(to: url)
            projectURL = url
            isDirty = false
            NSDocumentController.shared.noteNewRecentDocumentURL(url)
            scheduleSessionSave()
            flash(tr("Saved"))
        } catch {
            alert(tr("Could not save"), error.localizedDescription)
        }
    }

    /// Writes the .effector package (document.json + original image + thumbnail).
    func writeProject(to url: URL) throws {
        guard let src = source else { return }
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
        if FileManager.default.fileExists(atPath: url.path) { try? FileManager.default.removeItem(at: url) }
        let wrapper = FileWrapper(directoryWithFileWrappers: files)
        try wrapper.write(to: url, options: .atomic, originalContentsURL: nil)
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

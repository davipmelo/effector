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

    // Documents
    @Published private(set) var docs: [DocSlot] = []
    @Published private(set) var activeID: UUID?
    /// Images selected in the filmstrip (for sync / batch export).
    @Published var stripSelection: Set<UUID> = []
    @Published var showSync = false
    @Published var showFilmstrip = UserDefaults.standard.object(forKey: "showFilmstrip") as? Bool ?? true {
        didSet { UserDefaults.standard.set(showFilmstrip, forKey: "showFilmstrip") }
    }

    // Active document (working copy)
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
    @Published var output: MTLTexture?
    @Published var original: MTLTexture?
    @Published private(set) var isRendering = false
    @Published var isCropping = false
    @Published var cropDraft = CropRect.full
    @Published var cropAspect: String = "free"
    @Published var showEffectBrowser = false { didSet { if !showEffectBrowser { previewLayers = nil } } }
    /// Presets docked in the lower half of the left sidebar (stays open until closed; remembered).
    @Published var showPresetBrowser = UserDefaults.standard.bool(forKey: "showPresetsDock") {
        didSet {
            if !showPresetBrowser { previewLayers = nil }
            if showPresetBrowser { showEffectBrowser = false }
            UserDefaults.standard.set(showPresetBrowser, forKey: "showPresetsDock")
        }
    }
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

    // MARK: - Documents (several images, one active)

    var hasImage: Bool { source != nil }
    var activeDoc: DocSlot? { docs.first { $0.id == activeID } }
    var activeIndex: Int? { docs.firstIndex { $0.id == activeID } }

    /// Adds an image as a new document and makes it active.
    @discardableResult
    func openImage(_ img: SourceImage, url: URL? = nil, state initial: DocumentState = DocumentState(),
                   projectURL purl: URL? = nil, activate: Bool = true) -> DocSlot {
        let slot = DocSlot(source: img, fileURL: url)
        slot.state = initial
        slot.projectURL = purl
        docs.append(slot)
        if !restoringSession && sessionEnabled { writeSessionOriginal(slot) }
        if activate { self.activate(slot.id) }
        makeStripThumb(slot)
        scheduleSessionSave()
        return slot
    }

    /// Saves the working copy of the active document back into its slot.
    private func stash() {
        guard let s = activeDoc else { return }
        s.state = state; s.undo = undoStack; s.redo = redoStack
        s.projectURL = projectURL; s.isDirty = isDirty; s.selectedID = selectedID
        s.zoom = zoom; s.pan = pan
    }

    /// Switches the editor to another image.
    func activate(_ id: UUID) {
        guard let slot = docs.first(where: { $0.id == id }) else { return }
        if id == activeID && source != nil { return }
        if isCropping { cancelCrop() }
        stash()
        let src: SourceImage
        do { src = try slot.loadSource() } catch { alert(tr("Could not open file"), error.localizedDescription); return }
        activeID = id
        slot.lastUsed = Date()
        loadingDoc = true
        source = src
        previewLayers = nil
        soloID = nil
        suppressUndo = true
        state = slot.state
        suppressUndo = false
        undoStack = slot.undo; redoStack = slot.redo
        lastCoalesceKey = nil
        projectURL = slot.projectURL
        isDirty = slot.isDirty
        selectedID = slot.selectedID.flatMap { state.layers.find($0) == nil ? nil : $0 }
        zoom = slot.zoom; pan = slot.pan
        loadingDoc = false
        sourceTexCache = nil
        renderer.clearCache()
        updatePreviewScale(force: true)
        requestRender()
        regenerateThumbnails()
        unloadInactiveSources()
        if !stripSelection.contains(id) { stripSelection = [id] }
        objectWillChange.send()
        scheduleSessionSave()
    }

    /// Keeps decoded pixels only for the active image and the two most recently used ones.
    private func unloadInactiveSources() {
        let keep = Set(docs.filter { $0.source != nil }.sorted { $0.lastUsed > $1.lastUsed }.prefix(3).map(\.id))
        for d in docs where d.id != activeID && !keep.contains(d.id) { d.source = nil }
    }

    func activateNext(_ delta: Int) {
        guard let i = activeIndex, !docs.isEmpty else { return }
        let n = min(docs.count - 1, max(0, i + delta))
        if n != i { activate(docs[n].id) }
    }

    /// Closes images (their edits are lost unless saved as a project).
    func closeDocuments(_ ids: Set<UUID>) {
        guard !ids.isEmpty else { return }
        let edited = docs.filter { ids.contains($0.id) && ($0.id == activeID ? !state.layers.isEmpty : !$0.state.layers.isEmpty) }
        if !edited.isEmpty {
            let a = NSAlert()
            a.messageText = ids.count == 1 ? tr("Close this image?") : tr("Close %d images?", ids.count)
            a.informativeText = tr("Their effects will be removed from the workspace. Saved projects are not affected.")
            a.addButton(withTitle: tr("Close"))
            a.addButton(withTitle: tr("Cancel"))
            if a.runModal() != .alertFirstButtonReturn { return }
        }
        let wasActive = activeID.map { ids.contains($0) } ?? false
        let oldIndex = activeIndex ?? 0
        for d in docs where ids.contains(d.id) { removeSessionOriginal(d) }
        docs.removeAll { ids.contains($0.id) }
        stripSelection.subtract(ids)
        if wasActive {
            activeID = nil
            if docs.isEmpty {
                source = nil
                loadingDoc = true
                suppressUndo = true; state = DocumentState(); suppressUndo = false
                loadingDoc = false
                undoStack = []; redoStack = []; projectURL = nil; isDirty = false; selectedID = nil
                output = nil; original = nil
            } else {
                activate(docs[min(oldIndex, docs.count - 1)].id)
            }
        }
        scheduleSessionSave()
    }

    // MARK: Filmstrip helpers

    /// Thumbnail of an image with its own effects (for the filmstrip).
    func makeStripThumb(_ slot: DocSlot) {
        let st = slot.id == activeID ? state : slot.state
        let src = slot.source
        let data = slot.data, name = slot.fileName, hint = slot.typeHint
        DispatchQueue.global(qos: .utility).async {
            guard let s = src ?? (try? SourceImage.load(data: data, fileName: name, typeHint: hint)) else { return }
            let (cw, ch) = s.croppedSize(st.crop)
            let scale = 180.0 / Double(max(cw, ch))
            let tex = s.makeTexture(crop: st.crop, scale: scale)
            let out = Renderer(useCache: false).render(layers: st.layers, source: tex, sourceKey: 0, canvas: st.activeCanvas)
            let cg = ImageExport.cgImage(out)
            DispatchQueue.main.async { slot.thumb = cg }
        }
    }

    private var thumbWork: DispatchWorkItem?
    private func scheduleActiveThumb() {
        thumbWork?.cancel()
        let w = DispatchWorkItem { [weak self] in
            guard let self, let s = self.activeDoc else { return }
            self.stash()
            self.makeStripThumb(s)
        }
        thumbWork = w
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.7, execute: w)
    }

    /// Filmstrip click: plain = open, ⌘ = toggle in selection, ⇧ = range.
    func stripClick(_ id: UUID) {
        let f = NSEvent.modifierFlags
        if f.contains(.command) {
            if stripSelection.contains(id) && stripSelection.count > 1 { stripSelection.remove(id) } else { stripSelection.insert(id) }
            return
        }
        if f.contains(.shift), let a = activeIndex, let b = docs.firstIndex(where: { $0.id == id }) {
            stripSelection = Set(docs[min(a, b)...max(a, b)].map(\.id))
            return
        }
        stripSelection = [id]
        activate(id)
    }

    /// Which parts of the active image's settings to copy to other images.
    struct SyncOptions: OptionSet {
        let rawValue: Int
        static let layers = SyncOptions(rawValue: 1)
        static let crop = SyncOptions(rawValue: 2)
        static let canvas = SyncOptions(rawValue: 4)
        static let addOnTop = SyncOptions(rawValue: 8)
    }

    /// Copies the active image's settings to other images (each gets its own undo step).
    func syncSettings(to ids: Set<UUID>, _ opt: SyncOptions) {
        stash()
        let targets = docs.filter { ids.contains($0.id) && $0.id != activeID }
        guard !targets.isEmpty else { return }
        for t in targets {
            t.undo.append(t.state)
            t.redo.removeAll()
            if opt.contains(.layers) {
                let copy = state.layers.map { $0.reidentified() }
                t.state.layers = opt.contains(.addOnTop) ? copy + t.state.layers : copy
            }
            if opt.contains(.crop) { t.state.crop = state.crop }
            if opt.contains(.canvas) { t.state.canvas = state.canvas }
            t.isDirty = true
            makeStripThumb(t)
        }
        flash(tr("Settings synced to %d images", targets.count))
        scheduleSessionSave()
    }

    @Published private var copiedSettings: DocumentState?
    var hasCopiedSettings: Bool { copiedSettings != nil }

    func copySettings() {
        stash()
        copiedSettings = state
        flash(tr("Settings copied"))
    }

    /// Pastes copied settings (layers, crop and canvas) onto the selected images.
    func pasteSettings(to ids: Set<UUID>) {
        guard let c = copiedSettings else { return }
        stash()
        for t in docs where ids.contains(t.id) {
            if t.id == activeID {
                edit { s in s.layers = c.layers.map { $0.reidentified() }; s.crop = c.crop; s.canvas = c.canvas }
            } else {
                t.undo.append(t.state); t.redo.removeAll()
                t.state.layers = c.layers.map { $0.reidentified() }
                t.state.crop = c.crop; t.state.canvas = c.canvas
                t.isDirty = true
                makeStripThumb(t)
            }
        }
        flash(tr("Settings pasted to %d images", ids.count))
        scheduleSessionSave()
    }

    func resetSettings(of ids: Set<UUID>) {
        stash()
        for t in docs where ids.contains(t.id) {
            if t.id == activeID { edit { $0 = DocumentState() } }
            else { t.undo.append(t.state); t.redo.removeAll(); t.state = DocumentState(); t.isDirty = true; makeStripThumb(t) }
        }
        scheduleSessionSave()
    }

    // MARK: - Session (reopened on next launch)

    private struct SessionDoc: Codable {
        var fileName: String
        var originalFile: String
        var state: DocumentState
        var projectPath: String?
        var dirty: Bool
        var filePath: String?
    }
    private struct SessionFile: Codable {
        var version = 2
        var active: Int
        var docs: [SessionDoc]
    }
    /// Version 1 (single image) format, still readable.
    private struct SessionFileV1: Codable {
        var fileName: String
        var originalFile: String
        var state: DocumentState
        var projectPath: String?
        var dirty: Bool
    }
    private var restoringSession = false
    private var loadingDoc = false
    /// Headless tests must never touch the user's real session.
    static var headless = false
    private var sessionEnabled: Bool { !AppState.headless && AppSettings.shared.reopenLastSession }
    private var sessionWork: DispatchWorkItem?
    private let sessionQueue = DispatchQueue(label: "effector.session", qos: .utility)

    private var sessionDir: URL {
        let base = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0]
        let d = base.appendingPathComponent("EFFECTOR/Session", isDirectory: true)
        try? FileManager.default.createDirectory(at: d, withIntermediateDirectories: true)
        return d
    }

    private func writeSessionOriginal(_ slot: DocSlot) {
        let ext = (slot.fileName as NSString).pathExtension.isEmpty ? "png" : (slot.fileName as NSString).pathExtension.lowercased()
        let name = "img-" + slot.id.uuidString + "." + ext
        slot.sessionFile = name
        let url = sessionDir.appendingPathComponent(name)
        let data = slot.data
        sessionQueue.async { try? data.write(to: url, options: .atomic) }
    }

    private func removeSessionOriginal(_ slot: DocSlot) {
        guard let f = slot.sessionFile else { return }
        let url = sessionDir.appendingPathComponent(f)
        sessionQueue.async { try? FileManager.default.removeItem(at: url) }
    }

    func scheduleSessionSave() {
        guard !restoringSession, sessionEnabled else { return }
        sessionWork?.cancel()
        let w = DispatchWorkItem { [weak self] in self?.saveSession() }
        sessionWork = w
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.8, execute: w)
    }

    func saveSession(sync: Bool = false) {
        guard !restoringSession, sessionEnabled else { return }
        stash()
        let list = docs.compactMap { d -> SessionDoc? in
            guard let f = d.sessionFile else { return nil }
            return SessionDoc(fileName: d.fileName, originalFile: f, state: d.state, projectPath: d.projectURL?.path,
                              dirty: d.isDirty, filePath: d.fileURL?.path)
        }
        let file = SessionFile(active: activeIndex ?? 0, docs: list)
        let dir = sessionDir
        let keep = Set(list.map(\.originalFile) + ["session.json"])
        let work = {
            if let d = try? JSONEncoder().encode(file) { try? d.write(to: dir.appendingPathComponent("session.json"), options: .atomic) }
            // remove images that are no longer open
            for f in (try? FileManager.default.contentsOfDirectory(atPath: dir.path)) ?? [] where !keep.contains(f) {
                try? FileManager.default.removeItem(at: dir.appendingPathComponent(f))
            }
        }
        if sync { sessionQueue.sync(execute: work) } else { sessionQueue.async(execute: work) }
    }

    /// Reopens the images and layers that were open when the app last quit.
    @discardableResult
    func restoreSession() -> Bool {
        guard sessionEnabled, !hasImage else { return false }
        let dir = sessionDir
        guard let d = try? Data(contentsOf: dir.appendingPathComponent("session.json")) else { return false }
        var entries: [SessionDoc] = []
        var active = 0
        if let f = try? JSONDecoder().decode(SessionFile.self, from: d) {
            entries = f.docs; active = f.active
        } else if let f = try? JSONDecoder().decode(SessionFileV1.self, from: d) {
            entries = [SessionDoc(fileName: f.fileName, originalFile: f.originalFile, state: f.state, projectPath: f.projectPath, dirty: f.dirty)]
        }
        guard !entries.isEmpty else { return false }
        restoringSession = true
        for (i, e) in entries.enumerated() {
            guard let data = try? Data(contentsOf: dir.appendingPathComponent(e.originalFile)) else { continue }
            let hint = UTType(filenameExtension: (e.originalFile as NSString).pathExtension)
            let slot = DocSlot(data: data, fileName: e.fileName, typeHint: hint, fileURL: e.filePath.map { URL(fileURLWithPath: $0) })
            slot.state = e.state
            slot.projectURL = e.projectPath.map { URL(fileURLWithPath: $0) }
            slot.isDirty = e.dirty
            slot.sessionFile = e.originalFile
            if i == active { _ = try? slot.loadSource() }
            docs.append(slot)
        }
        let target = docs.indices.contains(active) ? docs[active] : docs.first
        if let t = target { activate(t.id) }
        for s in docs { makeStripThumb(s) }
        restoringSession = false
        // a v1 session used a different file name: rewrite in the new format
        saveSession()
        return hasImage
    }

    func open(url: URL) { open(urls: [url]) }

    /// Opens images and/or projects; each becomes a new image in the filmstrip.
    func open(urls: [URL]) {
        var failed: [String] = []
        var first: DocSlot?
        for url in urls {
            do {
                if url.pathExtension.lowercased() == "effector" {
                    let slot = try openProject(url, activate: first == nil)
                    first = first ?? slot
                } else {
                    let slot = openImage(try SourceImage.load(url: url), url: url, activate: first == nil)
                    first = first ?? slot
                }
                NSDocumentController.shared.noteNewRecentDocumentURL(url)
            } catch {
                failed.append(url.lastPathComponent)
            }
        }
        if !failed.isEmpty { alert(tr("Could not open file"), failed.joined(separator: "\n")) }
        if urls.count > 1, first != nil { flash(tr("%d images opened", urls.count - failed.count)) }
    }

    func openPanel() {
        let p = NSOpenPanel()
        p.allowedContentTypes = [.image, .rawImage, .effectorProject]
        p.allowsMultipleSelection = true
        p.treatsFilePackagesAsDirectories = false
        if p.runModal() == .OK { open(urls: p.urls) }
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
        if !loadingDoc { isDirty = true; scheduleActiveThumb() }
        scheduleSessionSave()
        if old.crop != state.crop { sourceTexCache = nil; updatePreviewScale(force: true) }
        if old.canvas != state.canvas { updatePreviewScale(force: true) }
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
        let canvas = state.activeCanvas
        let scale = currentScale()
        renderQueue.async { [weak self] in
            guard let self else { return }
            let srcTex = self.sourceTexture(src, crop: crop, scale: scale)
            var h = Hasher(); h.combine(src.id); h.combine(crop)
            let out = self.renderer.render(layers: layers, source: srcTex, sourceKey: h.finalize(), canvas: canvas)
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
    /// Size of the final picture in original pixels (the canvas when one is set).
    func outputSize() -> (Int, Int) {
        guard let src = source else { return (0, 0) }
        let (w, h) = src.croppedSize(state.crop)
        guard let c = state.activeCanvas else { return (w, h) }
        let s = c.layout(Double(w), Double(h)).size
        return (Int(s.width), Int(s.height))
    }

    func updatePreviewScale(force: Bool = false) {
        guard source != nil, viewportSize.width > 10 else { return }
        let (cw, ch) = outputSize()
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
        showEffectBrowser = false; showCommandPalette = false
        presenting.toggle()
        let w = NSApp.windows.first { $0.identifier?.rawValue.hasPrefix("main") == true } ?? NSApp.mainWindow
        if let w {
            // remove the title bar / toolbar strip so the image really covers the screen
            w.toolbar?.isVisible = !presenting
            w.titlebarAppearsTransparent = presenting
            w.titleVisibility = .hidden
            if presenting { w.styleMask.insert(.fullSizeContentView) } else { w.styleMask.remove(.fullSizeContentView) }
            w.standardWindowButton(.closeButton)?.superview?.isHidden = presenting
        }
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
        guard source != nil else { return (0, 0) }
        let (w, h) = outputSize()
        return (w * scale, h * scale)
    }

    func renderFull(scale: Int) -> MTLTexture? {
        guard let src = source else { return nil }
        let tex = src.makeTexture(crop: state.crop, scale: Double(scale))
        return Renderer(useCache: false).render(layers: state.layers, source: tex, sourceKey: 0, isExport: true, canvas: state.activeCanvas)
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

    @Published var exportProgress: String?

    /// Exports several images (each with its own effects) into a folder.
    func exportBatch(_ ids: Set<UUID>, format: ImageExport.Format, quality: Double, scale: Int, suffix: String) {
        stash()
        let panel = NSOpenPanel()
        panel.canChooseDirectories = true
        panel.canChooseFiles = false
        panel.canCreateDirectories = true
        panel.prompt = tr("Export Here")
        panel.message = tr("Choose a folder for %d images", ids.count)
        guard panel.runModal() == .OK, let dir = panel.url else { return }
        let jobs = docs.filter { ids.contains($0.id) }.map { ($0, $0.state) }
        isExporting = true
        exportProgress = "0 / \(jobs.count)"
        DispatchQueue.global(qos: .userInitiated).async {
            var done = 0
            var failed: [String] = []
            for (slot, st) in jobs {
                autoreleasepool {
                    guard let src = slot.source ?? (try? SourceImage.load(data: slot.data, fileName: slot.fileName, typeHint: slot.typeHint)) else {
                        failed.append(slot.fileName); return
                    }
                    let tex = src.makeTexture(crop: st.crop, scale: Double(scale))
                    let out = Renderer(useCache: false).render(layers: st.layers, source: tex, sourceKey: 0, isExport: true, canvas: st.activeCanvas)
                    let base = (slot.fileName as NSString).deletingPathExtension + suffix + (scale > 1 ? "@\(scale)x" : "")
                    var url = dir.appendingPathComponent(base + "." + format.rawValue)
                    var n = 2
                    while FileManager.default.fileExists(atPath: url.path) {
                        url = dir.appendingPathComponent(base + "-\(n)." + format.rawValue); n += 1
                    }
                    do { try ImageExport.write(out, to: url, format: format, quality: quality, keepAlpha: src.hasAlpha && st.activeCanvas == nil) }
                    catch { failed.append(slot.fileName) }
                }
                done += 1
                let d = done
                DispatchQueue.main.async { self.exportProgress = "\(d) / \(jobs.count)" }
            }
            DispatchQueue.main.async {
                self.isExporting = false
                self.exportProgress = nil
                self.showExport = false
                if !failed.isEmpty { self.alert(tr("Export failed"), failed.joined(separator: "\n")) }
                self.flash(tr("Exported %d images", jobs.count - failed.count))
                NSWorkspace.shared.activateFileViewerSelecting([dir])
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

    @discardableResult
    func openProject(_ url: URL, activate: Bool = true) throws -> DocSlot {
        let w = try FileWrapper(url: url, options: .immediate)
        guard let files = w.fileWrappers,
              let json = files["document.json"]?.regularFileContents,
              let (name, orig) = files.first(where: { $0.key.hasPrefix("original.") }),
              let data = orig.regularFileContents else { throw SourceImage.LoadError.unreadable }
        let manifest = try JSONDecoder().decode(ProjectManifest.self, from: json)
        let img = try SourceImage.load(data: data, fileName: manifest.originalFileName,
                                       typeHint: UTType(filenameExtension: (name as NSString).pathExtension))
        return openImage(img, state: manifest.state, projectURL: url, activate: activate)
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


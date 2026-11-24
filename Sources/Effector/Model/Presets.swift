import Foundation
import AppKit

struct Preset: Codable, Identifiable, Hashable {
    var id = UUID()
    var name: String
    var layers: [LayerNode]
    var isFactory = false
    var thumbnail: Data? = nil

    static let fileVersion = 1
}

private struct PresetFile: Codable {
    var version = Preset.fileVersion
    var app = "EFFECTOR"
    var presets: [Preset]
}

/// Stores user presets on disk and provides factory presets.
final class PresetStore: ObservableObject {
    static let shared = PresetStore()

    @Published private(set) var user: [Preset] = []
    @Published var factoryVisible = true
    @Published var thumbs: [UUID: CGImage] = [:]

    var all: [Preset] { (factoryVisible ? FactoryPresets.all : []) + user }

    private var dir: URL {
        let base = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0]
        let d = base.appendingPathComponent("EFFECTOR", isDirectory: true)
        try? FileManager.default.createDirectory(at: d, withIntermediateDirectories: true)
        return d
    }
    private var file: URL { dir.appendingPathComponent("presets.json") }

    private init() {
        factoryVisible = UserDefaults.standard.object(forKey: "factoryPresetsVisible") as? Bool ?? true
        load()
    }

    private func load() {
        guard let d = try? Data(contentsOf: file), let f = try? JSONDecoder().decode(PresetFile.self, from: d) else { return }
        user = f.presets
    }

    private func persist() {
        let enc = JSONEncoder()
        if let d = try? enc.encode(PresetFile(presets: user)) { try? d.write(to: file, options: .atomic) }
    }

    func setFactoryVisible(_ v: Bool) {
        factoryVisible = v
        UserDefaults.standard.set(v, forKey: "factoryPresetsVisible")
    }

    func add(name: String, layers: [LayerNode], thumbnail: CGImage?) {
        var p = Preset(name: name, layers: layers.map { $0.reidentified() })
        if let t = thumbnail { p.thumbnail = NSBitmapImageRep(cgImage: t).representation(using: .jpeg, properties: [.compressionFactor: 0.8]) }
        user.insert(p, at: 0)
        persist()
    }

    func rename(_ id: UUID, _ name: String) {
        if let i = user.firstIndex(where: { $0.id == id }) { user[i].name = name; persist() }
    }

    func delete(_ id: UUID) {
        user.removeAll { $0.id == id }
        persist()
    }

    // MARK: Import / export

    func export(_ presets: [Preset], to url: URL) throws {
        let enc = JSONEncoder()
        enc.outputFormatting = [.prettyPrinted]
        let ps = presets.map { p -> Preset in var p = p; p.isFactory = false; return p }
        try enc.encode(PresetFile(presets: ps)).write(to: url, options: .atomic)
    }

    @discardableResult
    func importFile(_ url: URL) throws -> Int {
        let d = try Data(contentsOf: url)
        let f = try JSONDecoder().decode(PresetFile.self, from: d)
        let imported = f.presets.map { p -> Preset in
            var p = p; p.id = UUID(); p.isFactory = false
            p.layers = p.layers.map { $0.reidentified() }
            return p
        }
        user.insert(contentsOf: imported, at: 0)
        persist()
        return imported.count
    }

    func exportPanel(_ presets: [Preset]) {
        let p = NSSavePanel()
        p.allowedContentTypes = [.effectorPreset]
        p.nameFieldStringValue = (presets.count == 1 ? presets[0].name : "EFFECTOR Presets") + ".effectorpreset"
        guard p.runModal() == .OK, let url = p.url else { return }
        do { try export(presets, to: url) } catch { AppState.shared.alert(tr("Export failed"), error.localizedDescription) }
    }

    func importPanel() {
        let p = NSOpenPanel()
        p.allowedContentTypes = [.effectorPreset, .json]
        p.allowsMultipleSelection = true
        guard p.runModal() == .OK else { return }
        var n = 0
        for u in p.urls {
            do { n += try importFile(u) } catch { AppState.shared.alert(tr("Could not import presets"), error.localizedDescription) }
        }
        if n > 0 { AppState.shared.flash(tr("%d presets imported", n)) }
    }

    /// Renders thumbnails of all presets on the current image.
    func regenerateThumbs() {
        let app = AppState.shared
        guard app.hasImage else { return }
        let list = all
        DispatchQueue.global(qos: .utility).async {
            var out: [UUID: CGImage] = [:]
            for p in list {
                if let cg = app.renderThumbnail(layers: p.layers, size: 180) { out[p.id] = cg }
            }
            DispatchQueue.main.async { self.thumbs = out }
        }
    }
}

enum FactoryPresets {
    static var all: [Preset] { [] }
}

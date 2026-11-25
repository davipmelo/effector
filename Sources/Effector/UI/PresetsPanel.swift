import SwiftUI

/// Presets panel docked in the lower half of the left sidebar. Stays open after applying;
/// arrow keys browse it only while the pointer is over it or its search field has focus.
struct PresetBrowser: View {
    @ObservedObject var app = AppState.shared
    @ObservedObject var store = PresetStore.shared
    @State private var query = ""
    @State private var highlight: UUID?
    @AppStorage("presetBrowserList") private var listMode = false
    @FocusState private var focused: Bool
    @StateObject private var keys = BrowserKeyRouter()
    /// Mouse position at the last keyboard navigation: hovers are ignored until the mouse really moves
    /// (scrolling the list under a still pointer must not steal the highlight).
    @State private var keyNavMouse: CGPoint?
    @State private var hovering = false

    private var list: [Preset] {
        let q = query.lowercased()
        return store.all.filter { q.isEmpty || $0.displayName.lowercased().contains(q) || tr($0.category ?? "").lowercased().contains(q) }
    }

    private func sections(_ l: [Preset]) -> [(String, [Preset])] {
        var out: [(String, [Preset])] = []
        let mine = l.filter { !$0.isFactory }
        if !mine.isEmpty { out.append(("My Presets", mine)) }
        for c in FactoryPresets.categories {
            let items = l.filter { $0.isFactory && $0.category == c }
            if !items.isEmpty { out.append((c, items)) }
        }
        return out
    }

    var body: some View {
        let secs = sections(list)
        let flat = secs.flatMap { $0.1 }
        let _ = (keys.handler = { code in handleKey(code, flat) })
        VStack(spacing: 0) {
            HStack(spacing: 8) {
                SearchField(placeholder: tr("Search presets"), text: $query, focused: $focused)
                    .onSubmit { if let p = flat.first(where: { $0.id == highlight }) ?? flat.first { apply(p) } }
                ViewModeToggle(listMode: $listMode)
                Menu {
                    Button(tr("Save Current Stack as Preset…")) { app.saveStackAsPreset() }.disabled(app.state.layers.isEmpty)
                    Divider()
                    Button(tr("Import Presets…")) { store.importPanel() }
                    Button(tr("Export My Presets…")) { store.exportPanel(store.user) }.disabled(store.user.isEmpty)
                    Divider()
                    Toggle(tr("Show Built-in Presets"), isOn: Binding(get: { store.factoryVisible }, set: { store.setFactoryVisible($0) }))
                } label: {
                    Image(systemName: "ellipsis.circle").font(.system(size: 15))
                }
                .menuStyle(.borderlessButton)
                .menuIndicator(.hidden)
                .fixedSize()
                .help(tr("More preset options"))
                IconButton(symbol: "xmark", help: "Close presets (P)") { app.showPresetBrowser = false }
            }
            .padding(.horizontal, 10).padding(.vertical, 8)
            ScrollViewReader { proxy in
                ScrollView {
                    LazyVStack(alignment: .leading, spacing: 6, pinnedViews: [.sectionHeaders]) {
                        ForEach(secs, id: \.0) { title, items in
                            Section {
                                Group {
                                    if listMode {
                                        VStack(spacing: 2) {
                                            ForEach(items) { p in
                                                PresetRow(preset: p, highlighted: highlight == p.id, hover: { hover(p) }) { apply(p) }.id(p.id)
                                            }
                                        }
                                    } else {
                                        LazyVGrid(columns: [GridItem(.adaptive(minimum: 105), spacing: 8)], spacing: 10) {
                                            ForEach(items) { p in
                                                PresetCard(preset: p, highlighted: highlight == p.id, hover: { hover(p) }) { apply(p) }.id(p.id)
                                            }
                                        }
                                    }
                                }
                                .padding(.horizontal, 12)
                                .padding(.bottom, 10)
                            } header: {
                                HStack {
                                    Text(tr(title).uppercased()).font(Theme.sectionFont).tracking(0.6).foregroundStyle(Theme.textDim)
                                    Spacer()
                                    Text("\(items.count)").font(.system(size: 10).monospacedDigit()).foregroundStyle(Theme.textFaint)
                                }
                                .padding(.horizontal, 14).padding(.vertical, 7)
                                .background(Theme.panelBG)
                            }
                        }
                    }
                    if flat.isEmpty {
                        Text(tr("No presets yet. Build a stack of effects and save it from the ⋯ menu."))
                            .font(.system(size: 11.5)).foregroundStyle(Theme.textFaint)
                            .multilineTextAlignment(.center).padding(20)
                    }
                }
                .onChange(of: highlight) { _, h in if let h { withAnimation(.linear(duration: 0.1)) { proxy.scrollTo(h) } } }
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .background(Theme.panelBG)
        .help(tr("Hover or ↑↓ to preview · click or ↩ to add on top as a group · ⎋ close"))
        .onHover { inside in
            hovering = inside
            if !inside && !focused { highlight = nil; app.hoverPreview(nil) }
        }
        .onAppear { keys.install(); store.regenerateThumbs() }
        .onDisappear { keys.remove(); app.previewLayers = nil }
        .onChange(of: app.source?.id) { _, _ in store.regenerateThumbs() }
        .onChange(of: store.user.count) { _, _ in store.regenerateThumbs() }
        .onChange(of: query) { _, _ in highlight = nil; app.hoverPreview(nil) }
    }

    private func hover(_ p: Preset, fromKeys: Bool = false) {
        if !fromKeys, let m = keyNavMouse { if m == NSEvent.mouseLocation { return }; keyNavMouse = nil }
        highlight = p.id
        app.hoverPreview(app.presetPreviewLayers(p))
    }

    private func handleKey(_ code: UInt16, _ flat: [Preset]) -> Bool {
        // docked panel: only take keys while it is the thing being used
        guard !app.showEffectBrowser, hovering || focused else { return false }
        switch code {
        case KeyCode.escape:
            if highlight != nil || focused { highlight = nil; focused = false; app.hoverPreview(nil) }
            else { app.showPresetBrowser = false }
            return true
        case KeyCode.up, KeyCode.down, KeyCode.left, KeyCode.right:
            guard !flat.isEmpty else { return true }
            let d = (code == KeyCode.up || code == KeyCode.left) ? -1 : 1
            let i = flat.firstIndex { $0.id == highlight } ?? (d > 0 ? -1 : flat.count)
            let n = min(flat.count - 1, max(0, i + d))
            keyNavMouse = NSEvent.mouseLocation
            hover(flat[n], fromKeys: true)
            return true
        case KeyCode.returnKey, KeyCode.enter:
            if let p = flat.first(where: { $0.id == highlight }) ?? flat.first { apply(p) }
            return true
        default: return false
        }
    }

    private func apply(_ p: Preset) {
        app.previewLayers = nil
        app.applyPreset(p)
        highlight = nil
    }
}

private struct PresetMenu: View {
    let preset: Preset
    @ObservedObject var app = AppState.shared
    @ObservedObject var store = PresetStore.shared
    var body: some View {
        Button(tr("Apply")) { app.applyPreset(preset) }
        Button(tr("Export…")) { store.exportPanel([preset]) }
        if !preset.isFactory {
            Button(tr("Rename…")) {
                if let n = promptText(tr("Rename Preset"), tr("Name"), preset.name) { store.rename(preset.id, n) }
            }
            Divider()
            Button(tr("Delete"), role: .destructive) { store.delete(preset.id) }
        }
    }
}

struct PresetThumb: View {
    let preset: Preset
    var highlighted = false
    var height: CGFloat = 78
    @ObservedObject var store = PresetStore.shared
    var body: some View {
        ZStack {
            RoundedRectangle(cornerRadius: 7).fill(Theme.field)
            if let cg = store.thumbs[preset.id] {
                Image(decorative: cg, scale: 1).resizable().aspectRatio(contentMode: .fill)
            } else {
                Image(systemName: "wand.and.stars").foregroundStyle(Theme.textFaint)
            }
        }
        .frame(height: height)
        .clipShape(RoundedRectangle(cornerRadius: 7))
        .overlay(RoundedRectangle(cornerRadius: 7).stroke(highlighted ? Theme.accent : Theme.separator, lineWidth: highlighted ? 1.5 : 1))
        .overlay(alignment: .topTrailing) {
            if preset.isFactory {
                Image(systemName: "seal.fill").font(.system(size: 9)).foregroundStyle(Theme.accent).padding(5)
                    .help(tr("Built-in preset"))
            }
        }
    }
}

struct PresetCard: View {
    let preset: Preset
    var highlighted = false
    var hover: () -> Void = {}
    let action: () -> Void
    @State private var isHover = false

    var body: some View {
        VStack(alignment: .leading, spacing: 5) {
            PresetThumb(preset: preset, highlighted: highlighted || isHover)
            Text(preset.displayName).font(.system(size: 11.5, weight: .medium)).foregroundStyle(Theme.text).lineLimit(1)
        }
        .contentShape(Rectangle())
        .onHover { h in isHover = h; if h { hover() } }
        .onTapGesture { action() }
        .help(tr("Click to add on top as a group"))
        .contextMenu { PresetMenu(preset: preset) }
    }
}

struct PresetRow: View {
    let preset: Preset
    var highlighted = false
    var hover: () -> Void = {}
    let action: () -> Void
    @State private var isHover = false

    var body: some View {
        HStack(spacing: 10) {
            PresetThumb(preset: preset, highlighted: highlighted || isHover, height: 42).frame(width: 64)
            VStack(alignment: .leading, spacing: 2) {
                Text(preset.displayName).font(.system(size: 12.5, weight: .medium)).foregroundStyle(Theme.text)
                Text(preset.layers.allEffectIDs.compactMap { EffectRegistry.effect($0).map { tr($0.name) } }.joined(separator: " · "))
                    .font(.system(size: 11)).foregroundStyle(Theme.textDim).lineLimit(2)
            }
            Spacer(minLength: 0)
        }
        .padding(6)
        .background(RoundedRectangle(cornerRadius: 8).fill(highlighted ? Theme.rowSelected : (isHover ? Theme.rowHover : .clear)))
        .contentShape(Rectangle())
        .onHover { h in isHover = h; if h { hover() } }
        .onTapGesture { action() }
        .contextMenu { PresetMenu(preset: preset) }
    }
}

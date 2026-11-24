import SwiftUI

/// Lightroom-style strip of open images under the canvas.
struct Filmstrip: View {
    @ObservedObject var app = AppState.shared
    @State private var showSyncPopover = false
    @State private var targeted = false

    var body: some View {
        HStack(spacing: 0) {
            VStack(spacing: 4) {
                IconButton(symbol: "plus", help: "Add images… (⌘O)") { app.openPanel() }
                IconButton(symbol: "arrow.triangle.2.circlepath", help: "Sync settings from the active image to the selected ones (⌥⌘S)") {
                    showSyncPopover = true
                }
                .disabled(app.stripSelection.subtracting([app.activeID].compactMap { $0 }).isEmpty)
                .popover(isPresented: $showSyncPopover, arrowEdge: .top) { SyncPanel { showSyncPopover = false } }
            }
            .padding(.horizontal, 6)
            Rectangle().fill(Theme.separator).frame(width: 1)
            ScrollViewReader { proxy in
                ScrollView(.horizontal, showsIndicators: false) {
                    LazyHStack(spacing: 6) {
                        ForEach(Array(app.docs.enumerated()), id: \.element.id) { i, d in
                            FilmCell(doc: d, index: i + 1, active: d.id == app.activeID, selected: app.stripSelection.contains(d.id))
                                .id(d.id)
                        }
                    }
                    .padding(.horizontal, 8)
                    .padding(.vertical, 6)
                }
                .onChange(of: app.activeID) { _, id in if let id { withAnimation(.snappy(duration: 0.2)) { proxy.scrollTo(id, anchor: .center) } } }
            }
            .overlay {
                if targeted { RoundedRectangle(cornerRadius: 6).stroke(Theme.accent, lineWidth: 1.5).padding(2) }
            }
        }
        .frame(height: 84)
        .background(Theme.panelBG)
        .overlay(alignment: .top) { Rectangle().fill(Theme.separator).frame(height: 1) }
        .onDrop(of: [.fileURL], isTargeted: $targeted) { dropFiles($0) }
        .onChange(of: app.showSync) { _, v in if v { showSyncPopover = true; app.showSync = false } }
    }
}

private struct FilmCell: View {
    @ObservedObject var doc: DocSlot
    let index: Int
    let active: Bool
    let selected: Bool
    @ObservedObject var app = AppState.shared
    @State private var hover = false

    var body: some View {
        ZStack(alignment: .topLeading) {
            RoundedRectangle(cornerRadius: 5).fill(Theme.field)
            if let t = doc.thumb {
                Image(decorative: t, scale: 1).resizable().interpolation(.medium).aspectRatio(contentMode: .fit)
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
            } else {
                ProgressView().controlSize(.small).frame(maxWidth: .infinity, maxHeight: .infinity)
            }
            Text("\(index)")
                .font(.system(size: 9, weight: .bold).monospacedDigit())
                .padding(.horizontal, 4).padding(.vertical, 1)
                .background(Capsule().fill(.black.opacity(0.55)))
                .foregroundStyle(.white)
                .padding(3)
            if hasEdits {
                Image(systemName: "wand.and.stars").font(.system(size: 8, weight: .bold))
                    .padding(3).background(Circle().fill(.black.opacity(0.55))).foregroundStyle(Theme.accent)
                    .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .bottomTrailing)
                    .padding(3)
                    .help(tr("Has effects"))
            }
        }
        .frame(width: 104, height: 70)
        .clipShape(RoundedRectangle(cornerRadius: 5))
        .overlay(RoundedRectangle(cornerRadius: 5)
            .stroke(active ? Theme.accent : (selected ? Theme.accent.opacity(0.55) : (hover ? Theme.textFaint : Theme.separator)),
                    lineWidth: active ? 2 : (selected ? 1.5 : 1)))
        .opacity(active || selected || hover ? 1 : 0.82)
        .contentShape(Rectangle())
        .onHover { hover = $0 }
        .onTapGesture { app.stripClick(doc.id) }
        .help(doc.fileName + " · " + tr("click to open · ⌘/⇧-click to select several"))
        .contextMenu {
            let targets = app.stripSelection.contains(doc.id) ? app.stripSelection : [doc.id]
            Button(tr("Open")) { app.activate(doc.id) }
            Divider()
            if doc.id != app.activeID {
                Button(tr("Sync Settings from Active Image")) { app.syncSettings(to: targets, [.layers, .canvas]) }
            }
            Button(tr("Copy Settings")) { app.activate(doc.id); app.copySettings() }
            Button(tr("Paste Settings")) { app.pasteSettings(to: targets) }.disabled(!app.hasCopiedSettings)
            Button(tr("Reset Settings")) { app.resetSettings(of: targets) }
            Divider()
            if let u = doc.fileURL {
                Button(tr("Show in Finder")) { NSWorkspace.shared.activateFileViewerSelecting([u]) }
            }
            Button(tr("Select All Images")) { app.stripSelection = Set(app.docs.map(\.id)) }
            Divider()
            Button(targets.count > 1 ? tr("Close %d Images", targets.count) : tr("Close Image"), role: .destructive) {
                app.closeDocuments(targets)
            }
        }
    }

    private var hasEdits: Bool { active ? !app.state.layers.isEmpty || app.state.activeCanvas != nil : !doc.state.layers.isEmpty || doc.state.activeCanvas != nil }
}

/// Choose what to copy from the active image to the selected ones.
struct SyncPanel: View {
    let done: () -> Void
    @ObservedObject var app = AppState.shared
    @AppStorage("syncLayers") private var layers = true
    @AppStorage("syncAddOnTop") private var addOnTop = false
    @AppStorage("syncCrop") private var crop = false
    @AppStorage("syncCanvas") private var canvas = true

    var body: some View {
        let targets = app.stripSelection.subtracting([app.activeID].compactMap { $0 })
        VStack(alignment: .leading, spacing: 12) {
            Text(tr("Sync Settings")).font(.system(size: 13, weight: .semibold))
            Text(tr("Copy from the active image to %d selected images.", targets.count))
                .font(.system(size: 11)).foregroundStyle(Theme.textDim)
            ToggleRow(label: "Layers & Effects", isOn: layers) { layers = $0 }
            if layers {
                SegmentedRow(label: "", options: opts([("replace", "Replace Their Layers"), ("add", "Add on Top")]),
                             selected: addOnTop ? "add" : "replace") { addOnTop = $0 == "add" }
            }
            ToggleRow(label: "Crop", isOn: crop) { crop = $0 }
            ToggleRow(label: "Canvas & Borders", isOn: canvas) { canvas = $0 }
            HStack {
                Spacer()
                Button(tr("Cancel")) { done() }.buttonStyle(PillButtonStyle())
                Button(tr("Sync to %d Images", targets.count)) {
                    var o: AppState.SyncOptions = []
                    if layers { o.insert(.layers) }
                    if addOnTop { o.insert(.addOnTop) }
                    if crop { o.insert(.crop) }
                    if canvas { o.insert(.canvas) }
                    app.syncSettings(to: targets, o)
                    done()
                }
                .buttonStyle(PillButtonStyle(prominent: true))
                .disabled(targets.isEmpty || (!layers && !crop && !canvas))
            }
        }
        .padding(16)
        .frame(width: 300)
        .background(Theme.panelBG)
    }
}

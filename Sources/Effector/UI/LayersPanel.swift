import SwiftUI

/// In-progress reorder drag (handled entirely by the panel, so the drop indicator always clears).
private struct LayerDrag {
    var ids: [UUID]
    var target: UUID?
    var pos: AppState.DropPosition = .above
    var toBottom = false
}

private let rowH: CGFloat = 32
private let rowGap: CGFloat = 1
private let listTop: CGFloat = 6

struct LayersPanel: View {
    @ObservedObject var app = AppState.shared
    @State private var drag: LayerDrag?

    var body: some View {
        let rows = app.state.layers.flattened()
        VStack(spacing: 0) {
            header
            if app.isMultiSelection { selectionBar }
            ScrollView {
                VStack(spacing: rowGap) {
                    ForEach(rows, id: \.node.id) { row in
                        LayerRow(node: row.node, depth: row.depth,
                                 selected: app.selectedIDs.contains(row.node.id),
                                 primary: app.selectedID == row.node.id,
                                 dimmed: drag?.ids.contains(row.node.id) == true,
                                 dropIndicator: drag?.target == row.node.id && drag?.toBottom == false ? drag?.pos : nil)
                            .frame(height: rowH)
                            .gesture(DragGesture(minimumDistance: 4, coordinateSpace: .named("layerList"))
                                .onChanged { g in dragChanged(row.node.id, y: g.location.y, rows: rows) }
                                .onEnded { _ in dragEnded() })
                    }
                    // empty space below the list: click to deselect
                    Color.clear.frame(height: 60)
                        .contentShape(Rectangle())
                        .onTapGesture { app.selectedID = nil }
                        .overlay(alignment: .top) {
                            if drag?.toBottom == true { Capsule().fill(Theme.accent).frame(height: 2).padding(.horizontal, 10) }
                        }
                }
                .padding(.top, listTop)
                .coordinateSpace(name: "layerList")
            }
            .overlay {
                if app.state.layers.isEmpty {
                    VStack(spacing: 8) {
                        Image(systemName: "square.3.layers.3d").font(.system(size: 26)).foregroundStyle(Theme.textFaint)
                        Text(app.hasImage ? tr("No effects yet") : tr("Open an image to start"))
                            .font(.system(size: 12, weight: .medium)).foregroundStyle(Theme.textDim)
                        if app.hasImage {
                            Text(tr("Add an effect or apply a preset.\nThe top layer is applied last."))
                                .font(.system(size: 11)).foregroundStyle(Theme.textFaint).multilineTextAlignment(.center)
                        }
                    }
                    .padding(.horizontal, 16)
                    .allowsHitTesting(false)
                }
            }
        }
    }

    // MARK: Header

    private var header: some View {
        VStack(spacing: 8) {
            HStack(spacing: 6) {
                Button { app.showEffectBrowser.toggle() } label: {
                    HStack(spacing: 6) {
                        Image(systemName: "plus").font(.system(size: 11, weight: .bold))
                        Text(tr("Add Effect")).font(.system(size: 12, weight: .semibold)).lineLimit(1)
                    }
                    .foregroundStyle(Color.black)
                    .frame(maxWidth: .infinity)
                    .padding(.vertical, 6)
                    .background(RoundedRectangle(cornerRadius: 7).fill(Theme.accent))
                }
                .buttonStyle(.plain)
                .help(tr("Add Effect…") + "  ⇧⌘A")
                Button { app.showPresetBrowser.toggle() } label: {
                    HStack(spacing: 6) {
                        Image(systemName: "wand.and.stars").font(.system(size: 11, weight: .semibold))
                        Text(tr("Presets")).font(.system(size: 12, weight: .semibold)).lineLimit(1)
                    }
                    .foregroundStyle(app.showPresetBrowser ? Color.black : Theme.text)
                    .frame(maxWidth: .infinity)
                    .padding(.vertical, 6)
                    .background(RoundedRectangle(cornerRadius: 7).fill(app.showPresetBrowser ? Theme.accent : Theme.fieldStrong))
                }
                .buttonStyle(.plain)
                .help(tr("Browse presets") + "  P")
            }
            .disabled(!app.hasImage)
            .opacity(app.hasImage ? 1 : 0.4)
            HStack(spacing: 2) {
                Text(tr("Layers").uppercased()).font(Theme.sectionFont).tracking(0.6).foregroundStyle(Theme.textDim)
                Spacer()
                IconButton(symbol: "folder.badge.plus", help: "Group selected layers (⌘G)") { app.groupSelected() }
                IconButton(symbol: "plus.square.on.square", help: "Duplicate (⌘D)") { app.duplicateSelected() }
                IconButton(symbol: "square.and.arrow.down.on.square", help: "Save all layers as a preset") { app.saveStackAsPreset() }
                    .disabled(app.state.layers.isEmpty)
            }
            .disabled(!app.hasImage)
        }
        .padding(.horizontal, 10)
        .padding(.top, 10)
        .padding(.bottom, 6)
        .overlay(alignment: .bottom) { Rectangle().fill(Theme.separator).frame(height: 1) }
    }

    private var selectionBar: some View {
        HStack(spacing: 2) {
            Text(tr("%d selected", app.selectedIDs.count)).font(.system(size: 11.5, weight: .semibold)).foregroundStyle(Theme.accentText)
            Spacer()
            IconButton(symbol: "folder.badge.plus", help: "Group selected layers (⌘G)") { app.groupSelected() }
            IconButton(symbol: "plus.square.on.square", help: "Duplicate (⌘D)") { app.duplicateSelected() }
            IconButton(symbol: "eye.slash", help: "Show / Hide (H)") { app.toggleSelectedVisibility() }
            IconButton(symbol: "trash", help: "Delete (⌘⌫)") { app.deleteSelected() }
            IconButton(symbol: "xmark", help: "Clear selection") { app.selectedID = nil }
        }
        .padding(.horizontal, 10)
        .frame(height: 34)
        .background(Theme.rowSelected.opacity(0.6))
        .overlay(alignment: .bottom) { Rectangle().fill(Theme.separator).frame(height: 1) }
    }

    // MARK: Drag to reorder

    private func dragChanged(_ id: UUID, y: CGFloat, rows: [(node: LayerNode, depth: Int)]) {
        if drag == nil {
            if !app.selectedIDs.contains(id) { app.selectedID = id }
            drag = LayerDrag(ids: app.selectionRoots.isEmpty ? [id] : app.selectionRoots)
        }
        guard var d = drag else { return }
        let f = (y - listTop) / (rowH + rowGap)
        let idx = Int(floor(f))
        if idx >= rows.count {
            d.toBottom = true; d.target = nil
        } else {
            let i = max(0, idx)
            let node = rows[i].node
            let frac = idx < 0 ? 0 : f - CGFloat(idx)
            d.toBottom = false
            let invalid = d.ids.contains(node.id) || d.ids.contains { app.state.layers.contains(node.id, inside: $0) }
            if invalid {
                d.target = nil
            } else {
                d.target = node.id
                if node.isGroup {
                    d.pos = frac < 0.3 ? .above : (frac > 0.7 && !(node.expanded && !node.children.isEmpty) ? .below : .inside)
                } else {
                    d.pos = frac < 0.5 ? .above : .below
                }
            }
        }
        drag = d
    }

    private func dragEnded() {
        defer { drag = nil }
        guard let d = drag else { return }
        if d.toBottom { app.moveToRootBottom(d.ids) }
        else if let t = d.target { app.move(d.ids, to: t, d.pos) }
    }
}

struct LayerRow: View {
    let node: LayerNode
    let depth: Int
    let selected: Bool
    var primary = true
    var dimmed = false
    let dropIndicator: AppState.DropPosition?
    @ObservedObject var app = AppState.shared
    @State private var hover = false
    @State private var renaming = false
    @State private var name = ""
    @FocusState private var focused: Bool

    /// Actions from a row act on the whole selection when the row is part of it.
    private func select() { if !app.selectedIDs.contains(node.id) { app.selectedID = node.id } }

    var body: some View {
        HStack(spacing: 6) {
            Color.clear.frame(width: CGFloat(depth) * 14)
            if node.isGroup {
                Button {
                    withAnimation(.snappy(duration: 0.18)) { app.updateLayerSilently(node.id) { $0.expanded.toggle() } }
                } label: {
                    Image(systemName: "chevron.right")
                        .font(.system(size: 9, weight: .bold))
                        .rotationEffect(.degrees(node.expanded ? 90 : 0))
                        .foregroundStyle(Theme.textDim)
                        .frame(width: 16, height: 24)
                        .contentShape(Rectangle())
                }
                .buttonStyle(.plain)
                .help(node.expanded ? tr("Collapse group") : tr("Expand group"))
            } else {
                Color.clear.frame(width: 16)
            }
            Image(systemName: node.isGroup ? (node.expanded ? "folder" : "folder.fill") : (EffectRegistry.effect(node.effectID)?.symbol ?? "square"))
                .font(.system(size: 11.5))
                .foregroundStyle(selected ? Theme.accentText : Theme.textDim)
                .frame(width: 18)
            if renaming {
                TextField("", text: $name)
                    .textFieldStyle(.plain)
                    .font(.system(size: 12.5))
                    .focused($focused)
                    .onSubmit { finishRename() }
                    .onChange(of: focused) { _, f in if !f { finishRename() } }
            } else {
                Text(node.name)
                    .font(.system(size: 12.5, weight: selected ? .medium : .regular))
                    .foregroundStyle(node.enabled ? Theme.text : Theme.textFaint)
                    .lineLimit(1)
            }
            Spacer(minLength: 4)
            if node.blend != .normal {
                Text(tr(node.blend.label)).font(.system(size: 9.5)).foregroundStyle(Theme.textFaint).lineLimit(1)
                    .help(tr("Blend mode"))
            }
            if node.opacity < 0.999 {
                Text("\(Int((node.opacity * 100).rounded()))%").font(.system(size: 10).monospacedDigit()).foregroundStyle(Theme.textFaint)
                    .help(tr("Opacity"))
            }
            if hover || app.soloID == node.id {
                Image(systemName: "s.circle\(app.soloID == node.id ? ".fill" : "")")
                    .font(.system(size: 12))
                    .foregroundStyle(app.soloID == node.id ? Theme.accentText : Theme.textFaint)
                    .onTapGesture { app.soloID = app.soloID == node.id ? nil : node.id }
                    .help(tr("Solo: show only this layer"))
            }
            Image(systemName: "trash")
                .font(.system(size: 10.5))
                .foregroundStyle(hover ? Theme.textDim : Theme.textFaint.opacity(0.7))
                .frame(width: 18)
                .contentShape(Rectangle())
                .onTapGesture { select(); app.deleteSelected() }
                .help(tr(app.selectedIDs.contains(node.id) && app.isMultiSelection ? "Delete selected layers" : "Delete") + " (⌘⌫)")
            Image(systemName: node.enabled ? "eye" : "eye.slash")
                .font(.system(size: 11.5))
                .foregroundStyle(node.enabled ? (hover ? Theme.text : Theme.textDim) : Theme.textFaint)
                .frame(width: 20)
                .contentShape(Rectangle())
                .onTapGesture {
                    if app.isMultiSelection && app.selectedIDs.contains(node.id) { app.toggleSelectedVisibility() }
                    else { app.toggleVisibility(node.id) }
                }
                .help(tr("Show / Hide") + " (H)")
        }
        .padding(.horizontal, 10)
        .frame(height: 32)
        .background(
            RoundedRectangle(cornerRadius: 7)
                .fill(selected ? (primary ? Theme.rowSelected : Theme.rowSelected.opacity(0.6)) : (hover ? Theme.rowHover : .clear))
                .padding(.horizontal, 6)
        )
        .opacity(dimmed ? 0.45 : 1)
        .overlay(alignment: .top) {
            if dropIndicator == .above { Capsule().fill(Theme.accent).frame(height: 2).padding(.leading, CGFloat(depth) * 14 + 10).padding(.trailing, 8).offset(y: -1) }
        }
        .overlay(alignment: .bottom) {
            if dropIndicator == .below { Capsule().fill(Theme.accent).frame(height: 2).padding(.leading, CGFloat(depth) * 14 + 10).padding(.trailing, 8).offset(y: 1) }
        }
        .overlay {
            if dropIndicator == .inside {
                RoundedRectangle(cornerRadius: 7).stroke(Theme.accent, lineWidth: 1.5).padding(.horizontal, 6)
            }
        }
        .contentShape(Rectangle())
        .onHover { hover = $0 }
        .onTapGesture(count: 2) { startRename() }
        .onTapGesture { app.click(node.id) }
        .help(tr("Click to select · ⇧/⌘-click for several · drag to reorder · double-click to rename"))
        .contextMenu {
            Button(tr("Rename")) { startRename() }
            Button(tr("Duplicate")) { select(); app.duplicateSelected() }
            Button(node.enabled ? tr("Hide") : tr("Show")) { select(); app.toggleSelectedVisibility() }
            Button(app.soloID == node.id ? tr("Unsolo") : tr("Solo")) { app.soloID = app.soloID == node.id ? nil : node.id }
            Divider()
            Button(tr("Group Layer")) { select(); app.groupSelected() }
            if node.isGroup { Button(tr("Ungroup")) { app.selectedID = node.id; app.ungroupSelected() } }
            Button(tr("Save as Preset…")) { savePreset() }
            Divider()
            Button(tr("Delete"), role: .destructive) { select(); app.deleteSelected() }
        }
    }

    private func startRename() {
        name = node.name
        renaming = true
        DispatchQueue.main.async { focused = true }
    }

    private func finishRename() {
        guard renaming else { return }
        renaming = false
        let n = name.trimmingCharacters(in: .whitespaces)
        if !n.isEmpty && n != node.name { app.updateLayer(node.id) { $0.name = n } }
    }

    private func savePreset() {
        guard let n = promptText(tr("Save as Preset"), tr("Name"), node.name) else { return }
        let roots = app.selectedIDs.contains(node.id) && app.isMultiSelection ? app.selectionRoots.compactMap { app.state.layers.find($0) }
            : (node.isGroup ? node.children : [node])
        PresetStore.shared.add(name: n, layers: roots, thumbnail: app.renderThumbnail(layers: roots, size: 240))
        app.flash(tr("Preset saved"))
    }
}

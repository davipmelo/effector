import SwiftUI
import UniformTypeIdentifiers

struct LayersPanel: View {
    @ObservedObject var app = AppState.shared
    @State private var dropTarget: (id: UUID, pos: AppState.DropPosition)?
    @State private var draggingID: UUID?

    var body: some View {
        VStack(spacing: 0) {
            HStack(spacing: 4) {
                Button { app.showEffectBrowser = true } label: {
                    HStack(spacing: 6) {
                        Image(systemName: "plus").font(.system(size: 11, weight: .bold))
                        Text(tr("Add Effect")).font(.system(size: 12, weight: .semibold))
                    }
                    .foregroundStyle(Color.black)
                    .padding(.horizontal, 10).padding(.vertical, 6)
                    .background(RoundedRectangle(cornerRadius: 7).fill(Theme.accent))
                }
                .buttonStyle(.plain)
                .disabled(!app.hasImage)
                .opacity(app.hasImage ? 1 : 0.4)
                Spacer()
                IconButton(symbol: "folder.badge.plus", help: "Group (⌘G)") { app.groupSelected() }
                IconButton(symbol: "plus.square.on.square", help: "Duplicate (⌘D)") { app.duplicateSelected() }
                IconButton(symbol: "trash", help: "Delete (⌘⌫)") { app.deleteSelected() }
            }
            .padding(.horizontal, 10)
            .frame(height: 44)
            .overlay(alignment: .bottom) { Rectangle().fill(Theme.separator).frame(height: 1) }

            ScrollView {
                LazyVStack(spacing: 1) {
                    let rows = app.state.layers.flattened()
                    ForEach(rows, id: \.node.id) { row in
                        LayerRow(node: row.node, depth: row.depth,
                                 selected: app.selectedID == row.node.id,
                                 dropIndicator: dropTarget?.id == row.node.id ? dropTarget?.pos : nil)
                            .onDrag {
                                draggingID = row.node.id
                                return NSItemProvider(object: row.node.id.uuidString as NSString)
                            }
                            .onDrop(of: [.text], delegate: LayerDropDelegate(target: row.node, app: app,
                                                                             dropTarget: $dropTarget, draggingID: $draggingID))
                    }
                    // bottom drop zone: move to bottom of root
                    Color.clear.frame(height: 40)
                        .contentShape(Rectangle())
                        .onTapGesture { app.selectedID = nil }
                        .onDrop(of: [.text], isTargeted: nil) { _ in
                            if let d = draggingID { app.moveToRootBottom(d) }
                            draggingID = nil; dropTarget = nil
                            return true
                        }
                }
                .padding(.vertical, 6)
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
                    .allowsHitTesting(false)
                }
            }
            // Source image row (always at the bottom)
            if let src = app.source {
                HStack(spacing: 8) {
                    Image(systemName: "photo").font(.system(size: 11)).foregroundStyle(Theme.textFaint).frame(width: 18)
                    Text(src.fileName).font(.system(size: 12)).foregroundStyle(Theme.textDim).lineLimit(1).truncationMode(.middle)
                    Spacer()
                    Image(systemName: "lock.fill").font(.system(size: 9)).foregroundStyle(Theme.textFaint)
                }
                .padding(.horizontal, 14)
                .frame(height: 34)
                .overlay(alignment: .top) { Rectangle().fill(Theme.separator).frame(height: 1) }
            }
        }
    }
}

struct LayerDropDelegate: DropDelegate {
    let target: LayerNode
    let app: AppState
    @Binding var dropTarget: (id: UUID, pos: AppState.DropPosition)?
    @Binding var draggingID: UUID?

    private func position(_ info: DropInfo) -> AppState.DropPosition {
        let y = info.location.y
        let h: CGFloat = 32
        if target.isGroup {
            if y < h * 0.3 { return .above }
            if y > h * 0.7 { return target.expanded && !target.children.isEmpty ? .inside : .below }
            return .inside
        }
        return y < h / 2 ? .above : .below
    }

    func dropUpdated(info: DropInfo) -> DropProposal? {
        dropTarget = (target.id, position(info))
        return DropProposal(operation: .move)
    }

    func dropExited(info: DropInfo) {
        if dropTarget?.id == target.id { dropTarget = nil }
    }

    func performDrop(info: DropInfo) -> Bool {
        let pos = position(info)
        dropTarget = nil
        if let d = draggingID {
            app.move(d, to: target.id, pos)
            app.selectedID = d
        }
        draggingID = nil
        return true
    }
}

struct LayerRow: View {
    let node: LayerNode
    let depth: Int
    let selected: Bool
    let dropIndicator: AppState.DropPosition?
    @ObservedObject var app = AppState.shared
    @State private var hover = false
    @State private var renaming = false
    @State private var name = ""
    @FocusState private var focused: Bool

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
            }
            if node.opacity < 0.999 {
                Text("\(Int((node.opacity * 100).rounded()))%").font(.system(size: 10).monospacedDigit()).foregroundStyle(Theme.textFaint)
            }
            if hover || app.soloID == node.id {
                Image(systemName: "s.circle\(app.soloID == node.id ? ".fill" : "")")
                    .font(.system(size: 12))
                    .foregroundStyle(app.soloID == node.id ? Theme.accentText : Theme.textFaint)
                    .onTapGesture { app.soloID = app.soloID == node.id ? nil : node.id }
                    .help(tr("Solo: show only this layer"))
            }
            Image(systemName: node.enabled ? "eye" : "eye.slash")
                .font(.system(size: 11.5))
                .foregroundStyle(node.enabled ? (hover ? Theme.text : Theme.textDim) : Theme.textFaint)
                .frame(width: 20)
                .contentShape(Rectangle())
                .onTapGesture { app.toggleVisibility(node.id) }
        }
        .padding(.horizontal, 10)
        .frame(height: 32)
        .background(
            RoundedRectangle(cornerRadius: 7)
                .fill(selected ? Theme.rowSelected : (hover ? Theme.rowHover : .clear))
                .padding(.horizontal, 6)
        )
        .overlay(alignment: .top) {
            if dropIndicator == .above { Capsule().fill(Theme.accent).frame(height: 2).padding(.leading, CGFloat(depth) * 14 + 10).padding(.trailing, 8) }
        }
        .overlay(alignment: .bottom) {
            if dropIndicator == .below { Capsule().fill(Theme.accent).frame(height: 2).padding(.leading, CGFloat(depth) * 14 + 10).padding(.trailing, 8) }
        }
        .overlay {
            if dropIndicator == .inside {
                RoundedRectangle(cornerRadius: 7).stroke(Theme.accent, lineWidth: 1.5).padding(.horizontal, 6)
            }
        }
        .contentShape(Rectangle())
        .onHover { hover = $0 }
        .onTapGesture(count: 2) { startRename() }
        .onTapGesture { app.selectedID = node.id }
        .contextMenu {
            Button(tr("Rename")) { startRename() }
            Button(tr("Duplicate")) { app.selectedID = node.id; app.duplicateSelected() }
            Button(node.enabled ? tr("Hide") : tr("Show")) { app.toggleVisibility(node.id) }
            Button(app.soloID == node.id ? tr("Unsolo") : tr("Solo")) { app.soloID = app.soloID == node.id ? nil : node.id }
            Divider()
            Button(tr("Group Layer")) { app.selectedID = node.id; app.groupSelected() }
            if node.isGroup { Button(tr("Ungroup")) { app.selectedID = node.id; app.ungroupSelected() } }
            Button(tr("Save as Preset…")) { savePreset() }
            Divider()
            Button(tr("Delete"), role: .destructive) { app.selectedID = node.id; app.deleteSelected() }
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
        let layers = node.isGroup ? node.children : [node]
        PresetStore.shared.add(name: n, layers: layers, thumbnail: app.renderThumbnail(layers: layers, size: 240))
        app.flash(tr("Preset saved"))
    }
}

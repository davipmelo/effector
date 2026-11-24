import SwiftUI

struct MainView: View {
    @ObservedObject var app = AppState.shared
    @ObservedObject var settings = AppSettings.shared
    @ObservedObject var shortcuts = ShortcutStore.shared

    var body: some View {
        let _ = settings.languageRevision
        HStack(spacing: 0) {
            sidebar
                .frame(width: 270)
            Rectangle().fill(Theme.separator).frame(width: 1)
            CanvasContainer()
                .frame(minWidth: 400, maxWidth: .infinity, maxHeight: .infinity)
            Rectangle().fill(Theme.separator).frame(width: 1)
            InspectorView()
                .frame(width: 300)
        }
        .background(Theme.panelBG)
        .overlay {
            if app.showCommandPalette {
                ZStack(alignment: .top) {
                    Color.black.opacity(0.25).ignoresSafeArea()
                        .onTapGesture { app.showCommandPalette = false }
                    CommandPalette().padding(.top, 80)
                }
            }
        }
        .sheet(isPresented: $app.showExport) { ExportSheet() }
        .toolbar { toolbarContent }
        .navigationTitle(title)
        .navigationSubtitle(subtitle)
        .preferredColorScheme(settings.theme.colorScheme)
        .tint(Theme.accent)
    }

    private var title: String {
        if let u = app.projectURL { return u.deletingPathExtension().lastPathComponent }
        return app.source?.fileName ?? "EFFECTOR"
    }

    private var subtitle: String {
        guard let s = app.source else { return "" }
        let (w, h) = s.croppedSize(app.state.crop)
        return "\(w) × \(h)" + (app.isDirty ? " · " + tr("Edited") : "")
    }

    private var sidebar: some View {
        VStack(spacing: 0) {
            HStack(spacing: 2) {
                tabButton(tr("Layers"), "square.3.layers.3d", 0)
                tabButton(tr("Presets"), "wand.and.stars", 1)
            }
            .padding(3)
            .background(RoundedRectangle(cornerRadius: 8).fill(Theme.field))
            .padding(.horizontal, 10)
            .padding(.vertical, 8)
            Rectangle().fill(Theme.separator).frame(height: 1)
            if app.sidebarTab == 0 { LayersPanel() } else { PresetsPanel() }
        }
        .background(Theme.panelBG)
    }

    private func tabButton(_ title: String, _ symbol: String, _ tag: Int) -> some View {
        HStack(spacing: 6) {
            Image(systemName: symbol).font(.system(size: 11))
            Text(title).font(.system(size: 12, weight: app.sidebarTab == tag ? .semibold : .regular))
        }
        .foregroundStyle(app.sidebarTab == tag ? Theme.text : Theme.textDim)
        .frame(maxWidth: .infinity)
        .padding(.vertical, 5)
        .background(RoundedRectangle(cornerRadius: 6).fill(app.sidebarTab == tag ? Theme.fieldStrong : .clear))
        .contentShape(Rectangle())
        .onTapGesture { app.sidebarTab = tag }
    }

    @ToolbarContentBuilder
    private var toolbarContent: some ToolbarContent {
        ToolbarItemGroup(placement: .navigation) {
            Button { Actions.perform(.open) } label: { Label(tr("Open"), systemImage: "folder") }
                .help(tr("Open…") + "  ⌘O")
            Button { app.save() } label: { Label(tr("Save"), systemImage: "square.and.arrow.down") }
                .help(tr("Save Project") + "  ⌘S")
                .disabled(!app.hasImage)
        }
        ToolbarItemGroup(placement: .principal) {
            HStack(spacing: 2) {
                Button { app.zoomOut() } label: { Image(systemName: "minus") }.buttonStyle(.borderless)
                Menu {
                    Button(tr("Fit to Window")) { app.setZoom(nil) }
                    Button("50%") { app.setZoom(0.5 / app.backingScale) }
                    Button("100%") { app.zoomActual() }
                    Button("200%") { app.setZoom(2 / app.backingScale) }
                    Button("400%") { app.setZoom(4 / app.backingScale) }
                } label: {
                    Text(app.hasImage ? "\(Int((app.displayZoom * app.backingScale * 100).rounded()))%" : "—")
                        .font(.system(size: 12).monospacedDigit())
                        .frame(width: 46)
                }
                .menuStyle(.borderlessButton)
                .menuIndicator(.hidden)
                .fixedSize()
                Button { app.zoomIn() } label: { Image(systemName: "plus") }.buttonStyle(.borderless)
            }
            .disabled(!app.hasImage)
        }
        ToolbarItemGroup(placement: .primaryAction) {
            Button { app.undo() } label: { Label(tr("Undo"), systemImage: "arrow.uturn.backward") }
                .disabled(!app.canUndo).help(tr("Undo") + "  ⌘Z")
            Button { app.redo() } label: { Label(tr("Redo"), systemImage: "arrow.uturn.forward") }
                .disabled(!app.canRedo).help(tr("Redo") + "  ⇧⌘Z")
            Button { Actions.perform(.crop) } label: { Label(tr("Crop"), systemImage: "crop") }
                .disabled(!app.hasImage).help(tr("Crop") + "  C")
            Toggle(isOn: $app.showOriginal) { Label(tr("Before / After"), systemImage: "square.split.2x1") }
                .disabled(!app.hasImage)
                .help(tr("Toggle Before / After") + "  " + (shortcuts.combo(.beforeAfter)?.display ?? ""))
            Button { app.showExport = true } label: { Label(tr("Export"), systemImage: "square.and.arrow.up") }
                .disabled(!app.hasImage).help(tr("Export Image…") + "  ⌘E")
        }
    }
}

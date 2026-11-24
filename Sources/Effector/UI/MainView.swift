import SwiftUI

struct MainView: View {
    @ObservedObject var app = AppState.shared
    @ObservedObject var settings = AppSettings.shared
    @ObservedObject var shortcuts = ShortcutStore.shared
    @AppStorage("leftWidth") private var leftWidth = 270.0
    @AppStorage("rightWidth") private var rightWidth = 300.0

    var body: some View {
        let _ = settings.languageRevision
        GeometryReader { geo in
            let total = geo.size.width
            // keep at least 420 pt for the canvas
            let maxLeft = max(200, min(480, total - rightWidth - 420))
            let maxRight = max(240, min(520, total - leftWidth - 420))
            let lw = min(max(leftWidth, 200), maxLeft)
            let rw = min(max(rightWidth, 240), maxRight)
            HStack(spacing: 0) {
                sidebar
                    .frame(width: lw)
                ResizeHandle(width: $leftWidth, range: 200...maxLeft, sign: 1)
                CanvasContainer()
                    .frame(minWidth: 300, maxWidth: .infinity, maxHeight: .infinity)
                ResizeHandle(width: $rightWidth, range: 240...maxRight, sign: -1)
                InspectorView()
                    .frame(width: rw)
            }
            .overlay(alignment: .topLeading) { panels(left: lw) }
        }
        .id(settings.languageRevision)
        .background(Theme.panelBG)
        .animation(.snappy(duration: 0.18), value: app.showEffectBrowser)
        .animation(.snappy(duration: 0.18), value: app.showPresetBrowser)
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

    @ViewBuilder
    private func panels(left: Double) -> some View {
        if app.showEffectBrowser || app.showPresetBrowser {
            ZStack(alignment: .topLeading) {
                Color.black.opacity(0.18).ignoresSafeArea()
                    .padding(.leading, left)
                    .onTapGesture { app.showEffectBrowser = false; app.showPresetBrowser = false }
                Group {
                    if app.showEffectBrowser { EffectBrowser() } else { PresetBrowser() }
                }
                .frame(maxHeight: .infinity)
                .clipShape(RoundedRectangle(cornerRadius: 14))
                .overlay(RoundedRectangle(cornerRadius: 14).stroke(Theme.separator))
                .shadow(color: .black.opacity(0.45), radius: 28, y: 10)
                .padding(.leading, left + 8)
                .padding(.vertical, 8)
                .transition(.opacity.combined(with: .scale(scale: 0.97, anchor: .topLeading)))
            }
        }
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
            AppHeader()
            LayersPanel()
        }
        .background(Theme.panelBG)
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
                    .help(tr("Zoom Out") + "  ⌘-")
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
                .help(tr("Zoom level · Z fits the image"))
                Button { app.zoomIn() } label: { Image(systemName: "plus") }.buttonStyle(.borderless)
                    .help(tr("Zoom In") + "  ⌘=")
            }
            .padding(.horizontal, 8)
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
            SettingsLink { Label(tr("Settings"), systemImage: "gearshape") }
                .help(tr("Settings") + "  ⌘,")
        }
    }
}

/// App identity at the top of the sidebar: icon, name and version.
struct AppHeader: View {
    private var version: String {
        (Bundle.main.infoDictionary?["CFBundleShortVersionString"] as? String) ?? "dev"
    }
    var body: some View {
        HStack(spacing: 9) {
            Image(nsImage: NSApp.applicationIconImage)
                .resizable()
                .interpolation(.high)
                .frame(width: 26, height: 26)
            Text("EFFECTOR")
                .font(.system(size: 13, weight: .heavy))
                .tracking(1.6)
                .foregroundStyle(Theme.text)
            Spacer()
            Text("v" + version)
                .font(.system(size: 10.5, weight: .medium, design: .monospaced))
                .foregroundStyle(Theme.textFaint)
                .padding(.horizontal, 6).padding(.vertical, 2)
                .background(Capsule().fill(Theme.field))
        }
        .padding(.horizontal, 12)
        .frame(height: 44)
        .overlay(alignment: .bottom) { Rectangle().fill(Theme.separator).frame(height: 1) }
    }
}

/// Draggable divider between the panels and the canvas.
struct ResizeHandle: View {
    @Binding var width: Double
    let range: ClosedRange<Double>
    let sign: Double
    @State private var start: Double?
    @State private var hover = false

    var body: some View {
        Rectangle().fill(hover || start != nil ? Theme.accent.opacity(0.6) : Theme.separator)
            .frame(width: 1)
            .overlay(
                Color.clear
                    .frame(width: 9)
                    .contentShape(Rectangle())
                    .onHover { h in
                        hover = h
                        if h { NSCursor.resizeLeftRight.push() } else { NSCursor.pop() }
                    }
                    .gesture(DragGesture(minimumDistance: 1, coordinateSpace: .global)
                        .onChanged { g in
                            if start == nil { start = min(max(width, range.lowerBound), range.upperBound) }
                            width = min(max((start ?? width) + g.translation.width * sign, range.lowerBound), range.upperBound)
                        }
                        .onEnded { _ in start = nil })
                    .onTapGesture(count: 2) { width = sign > 0 ? 270 : 300 }
                    .help(tr("Drag to resize · double-click to reset"))
            )
            .zIndex(5)
    }
}

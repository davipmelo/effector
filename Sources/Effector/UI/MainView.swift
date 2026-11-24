import SwiftUI

struct MainView: View {
    @ObservedObject var app = AppState.shared
    @ObservedObject var settings = AppSettings.shared
    @ObservedObject var shortcuts = ShortcutStore.shared
    @AppStorage("leftWidth") private var leftWidth = 270.0
    @State private var showCanvas = false
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
                if !app.presenting {
                    sidebar
                        .frame(width: lw)
                    ResizeHandle(width: $leftWidth, range: 200...maxLeft, sign: 1)
                }
                VStack(spacing: 0) {
                    CanvasContainer()
                        .frame(minWidth: 300, maxWidth: .infinity, maxHeight: .infinity)
                    if !app.presenting && app.showFilmstrip && !app.docs.isEmpty { Filmstrip() }
                    if !app.presenting { InfoBar() }
                }
                if !app.presenting {
                    ResizeHandle(width: $rightWidth, range: 240...maxRight, sign: -1)
                    InspectorView()
                        .frame(width: rw)
                }
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
        .toolbar(app.presenting ? .hidden : .visible, for: .windowToolbar)
        .ignoresSafeArea(edges: app.presenting ? .all : [])
        .background(app.presenting ? Color.black : Color.clear)
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
        ToolbarItem(placement: .principal) {
            VStack(spacing: 2) {
                Text("EFFECTOR")
                    .font(.system(size: 12.5, weight: .semibold))
                    .tracking(0.6)
                    .foregroundStyle(Theme.text)
                    .padding(.horizontal, 16).padding(.vertical, 2)
                    .background(Capsule().fill(Theme.field))
                Text("v" + AppInfo.version)
                    .font(.system(size: 8.5, weight: .regular, design: .monospaced))
                    .foregroundStyle(Theme.textFaint.opacity(0.8))
            }
            .frame(maxHeight: .infinity, alignment: .center)
            .help("EFFECTOR " + AppInfo.version)
        }
        .sharedBackgroundVisibility(.hidden)
        ToolbarItemGroup(placement: .primaryAction) {
            Button { app.undo() } label: { Label(tr("Undo"), systemImage: "arrow.uturn.backward") }
                .disabled(!app.canUndo).help(tr("Undo") + "  ⌘Z")
            Button { app.redo() } label: { Label(tr("Redo"), systemImage: "arrow.uturn.forward") }
                .disabled(!app.canRedo).help(tr("Redo") + "  ⇧⌘Z")
            Button { showCanvas.toggle() } label: { Label(tr("Canvas"), systemImage: "aspectratio") }
                .disabled(!app.hasImage)
                .help(tr("Canvas & Borders: fit the image in another aspect ratio (Instagram etc.)"))
                .popover(isPresented: $showCanvas, arrowEdge: .bottom) { CanvasPopover() }
            Button { Actions.perform(.crop) } label: { Label(tr("Crop"), systemImage: "crop") }
                .disabled(!app.hasImage).help(tr("Crop") + "  C")
            Toggle(isOn: $app.compareSplit) { Label(tr("Split Before / After"), systemImage: "rectangle.split.2x1") }
                .disabled(!app.hasImage)
                .help(tr("Split Before / After") + "  /")
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

enum AppInfo {
    static var version: String { (Bundle.main.infoDictionary?["CFBundleShortVersionString"] as? String) ?? "dev" }
}

/// Thin status bar under the canvas: image details on the left, zoom on the right.
struct InfoBar: View {
    @ObservedObject var app = AppState.shared

    var body: some View {
        HStack(spacing: 10) {
            if let src = app.source {
                let (w, h) = src.croppedSize(app.state.crop)
                if app.docs.count > 1, let i = app.activeIndex {
                    Text("\(i + 1) / \(app.docs.count)").font(font.monospacedDigit()).foregroundStyle(Theme.textDim)
                        .help(tr("Image %d of %d · ← → to switch", i + 1, app.docs.count))
                    sep
                }
                item("photo", src.fileName, help: "File")
                sep
                item("aspectratio", "\(w) × \(h) px", help: "Size")
                sep
                Text(aspect(w, h) + " · " + megapixels(w, h)).font(font).foregroundStyle(Theme.textFaint)
                    .help(tr("Aspect ratio and megapixels"))
                if !app.state.crop.isFull {
                    sep
                    item("crop", tr("Cropped from %@", "\(src.width) × \(src.height)"), help: "Crop")
                }
                if app.state.activeCanvas != nil {
                    let (cw, ch) = app.outputSize()
                    sep
                    item("aspectratio", tr("Canvas %@", "\(cw) × \(ch)"), help: "Canvas & Borders")
                }
                sep
                Text((src.fileName as NSString).pathExtension.uppercased() + (src.hasAlpha ? " · " + tr("Alpha") : ""))
                    .font(font).foregroundStyle(Theme.textFaint).help(tr("File type"))
                sep
                item("square.3.layers.3d", tr("%d layers", app.state.layers.flattened().filter { !$0.node.isGroup }.count), help: "Layers")
                if app.isDirty {
                    sep
                    HStack(spacing: 4) {
                        Circle().fill(Theme.accent).frame(width: 6, height: 6)
                        Text(app.projectURL == nil ? tr("Not saved") : tr("Edited")).font(font).foregroundStyle(Theme.textDim)
                    }
                    .help(tr("Unsaved changes are kept automatically and come back when you reopen the app"))
                }
            } else {
                Text(tr("No image open")).font(font).foregroundStyle(Theme.textFaint)
            }
            Spacer(minLength: 8)
            if app.isRendering { ProgressView().controlSize(.mini).help(tr("Rendering…")) }
            Button { app.showFilmstrip.toggle() } label: {
                Image(systemName: "photo.stack").font(.system(size: 11)).foregroundStyle(app.showFilmstrip ? Theme.accentText : Theme.textDim)
            }
            .buttonStyle(.borderless)
            .padding(.trailing, 6)
            .help(tr("Show / Hide Filmstrip") + "  B")
            zoomControl
        }
        .padding(.horizontal, 12)
        .frame(height: 28)
        .background(Theme.panelBG)
        .overlay(alignment: .top) { Rectangle().fill(Theme.separator).frame(height: 1) }
    }

    private let font = Font.system(size: 11)
    private var sep: some View { Rectangle().fill(Theme.separator).frame(width: 1, height: 12) }

    private func item(_ symbol: String, _ text: String, help: String) -> some View {
        HStack(spacing: 5) {
            Image(systemName: symbol).font(.system(size: 10)).foregroundStyle(Theme.textFaint)
            Text(text).font(font).foregroundStyle(Theme.textDim).lineLimit(1).truncationMode(.middle)
        }
        .help(tr(help))
    }

    private func aspect(_ w: Int, _ h: Int) -> String {
        func gcd(_ a: Int, _ b: Int) -> Int { b == 0 ? a : gcd(b, a % b) }
        let g = gcd(w, h)
        let (a, b) = (w / max(1, g), h / max(1, g))
        return a <= 32 && b <= 32 ? "\(a):\(b)" : String(format: "%.2f:1", Double(w) / Double(max(1, h)))
    }
    private func megapixels(_ w: Int, _ h: Int) -> String { String(format: "%.1f MP", Double(w * h) / 1_000_000) }

    private var zoomControl: some View {
        HStack(spacing: 10) {
            Button { app.zoomOut() } label: { Image(systemName: "minus").font(.system(size: 10, weight: .semibold)) }
                .buttonStyle(.borderless)
                .help(tr("Zoom Out") + "  ⌘-")
            Menu {
                Button(tr("Fit to Window") + "  Z") { app.setZoom(nil) }
                Divider()
                Button("50%") { app.setZoom(0.5 / app.backingScale) }
                Button("100%") { app.zoomActual() }
                Button("200%") { app.setZoom(2 / app.backingScale) }
                Button("400%") { app.setZoom(4 / app.backingScale) }
            } label: {
                Text(app.hasImage ? "\(Int((app.displayZoom * app.backingScale * 100).rounded()))%" : "—")
                    .font(.system(size: 11).monospacedDigit())
                    .frame(width: 44)
            }
            .menuStyle(.borderlessButton)
            .menuIndicator(.hidden)
            .fixedSize()
            .help(tr("Zoom level · Z fits the image"))
            Button { app.zoomIn() } label: { Image(systemName: "plus").font(.system(size: 10, weight: .semibold)) }
                .buttonStyle(.borderless)
                .help(tr("Zoom In") + "  ⌘=")
            Rectangle().fill(Theme.separator).frame(width: 1, height: 12)
            Button { app.setZoom(nil) } label: { Image(systemName: "arrow.up.left.and.arrow.down.right").font(.system(size: 10, weight: .semibold)) }
                .buttonStyle(.borderless)
                .help(tr("Fit to Window") + "  Z")
            Button { app.togglePresentation() } label: { Image(systemName: "rectangle.inset.filled").font(.system(size: 10, weight: .semibold)) }
                .buttonStyle(.borderless)
                .help(tr("Full Screen Preview") + "  F")
        }
        .padding(.horizontal, 12).padding(.vertical, 3)
        .background(Capsule().fill(Theme.field))
        .disabled(!app.hasImage)
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

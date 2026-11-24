import SwiftUI

struct PresetsPanel: View {
    @ObservedObject var app = AppState.shared
    @ObservedObject var store = PresetStore.shared
    @State private var query = ""

    private var list: [Preset] {
        let q = query.lowercased()
        return store.all.filter { q.isEmpty || $0.name.lowercased().contains(q) }
    }

    var body: some View {
        VStack(spacing: 0) {
            HStack(spacing: 4) {
                HStack(spacing: 6) {
                    Image(systemName: "magnifyingglass").font(.system(size: 11)).foregroundStyle(Theme.textFaint)
                    TextField(tr("Search presets"), text: $query).textFieldStyle(.plain).font(.system(size: 12))
                }
                .padding(.horizontal, 8).padding(.vertical, 6)
                .background(RoundedRectangle(cornerRadius: 7).fill(Theme.field))
                Menu {
                    Button(tr("Save Current Stack as Preset…")) { saveStack() }.disabled(app.state.layers.isEmpty)
                    Divider()
                    Button(tr("Import Presets…")) { store.importPanel() }
                    Button(tr("Export My Presets…")) { store.exportPanel(store.user) }.disabled(store.user.isEmpty)
                    Divider()
                    Toggle(tr("Show Built-in Presets"), isOn: Binding(get: { store.factoryVisible }, set: { store.setFactoryVisible($0) }))
                } label: {
                    Image(systemName: "ellipsis.circle").font(.system(size: 14))
                }
                .menuStyle(.borderlessButton)
                .menuIndicator(.hidden)
                .fixedSize()
                .padding(.leading, 4)
            }
            .padding(.horizontal, 10)
            .frame(height: 44)
            .overlay(alignment: .bottom) { Rectangle().fill(Theme.separator).frame(height: 1) }

            ScrollView {
                LazyVGrid(columns: [GridItem(.flexible(), spacing: 10), GridItem(.flexible(), spacing: 10)], spacing: 12) {
                    ForEach(list) { p in PresetCard(preset: p) }
                }
                .padding(10)
                if list.isEmpty {
                    Text(tr("No presets yet. Build a stack of effects and save it from the ⋯ menu."))
                        .font(.system(size: 11.5)).foregroundStyle(Theme.textFaint)
                        .multilineTextAlignment(.center).padding(20)
                }
            }
        }
        .onAppear { store.regenerateThumbs() }
        .onChange(of: app.source?.id) { _, _ in store.regenerateThumbs() }
        .onChange(of: store.user.count) { _, _ in store.regenerateThumbs() }
    }

    private func saveStack() {
        guard let n = promptText(tr("Save as Preset"), tr("Name"), tr("My Preset")) else { return }
        store.add(name: n, layers: app.state.layers, thumbnail: app.renderThumbnail(layers: app.state.layers, size: 240))
        app.flash(tr("Preset saved"))
    }
}

struct PresetCard: View {
    let preset: Preset
    @ObservedObject var app = AppState.shared
    @ObservedObject var store = PresetStore.shared
    @State private var hover = false

    var body: some View {
        VStack(alignment: .leading, spacing: 5) {
            ZStack {
                RoundedRectangle(cornerRadius: 7).fill(Theme.field)
                if let cg = store.thumbs[preset.id] {
                    Image(decorative: cg, scale: 1).resizable().aspectRatio(contentMode: .fill)
                } else {
                    Image(systemName: "wand.and.stars").foregroundStyle(Theme.textFaint)
                }
            }
            .frame(height: 78)
            .clipShape(RoundedRectangle(cornerRadius: 7))
            .overlay(RoundedRectangle(cornerRadius: 7).stroke(hover ? Theme.accent : Theme.separator, lineWidth: hover ? 1.5 : 1))
            .overlay(alignment: .topTrailing) {
                if preset.isFactory {
                    Image(systemName: "seal.fill").font(.system(size: 9)).foregroundStyle(Theme.accent).padding(5)
                }
            }
            Text(preset.name).font(.system(size: 11.5, weight: .medium)).foregroundStyle(Theme.text).lineLimit(1)
        }
        .contentShape(Rectangle())
        .onHover { hover = $0 }
        .onTapGesture { app.applyPreset(preset) }
        .help(tr("Click to add on top as a group"))
        .contextMenu {
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
}

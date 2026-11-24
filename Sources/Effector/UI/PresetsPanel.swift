import SwiftUI

struct PresetsPanel: View {
    @ObservedObject var app = AppState.shared
    @ObservedObject var store = PresetStore.shared
    @State private var query = ""

    private var list: [Preset] {
        let q = query.lowercased()
        return store.all.filter { q.isEmpty || $0.displayName.lowercased().contains(q) || tr($0.category ?? "").lowercased().contains(q) }
    }

    private var sections: [(String, [Preset])] {
        let l = list
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
        VStack(spacing: 0) {
            HStack(spacing: 4) {
                Button { saveStack() } label: { Image(systemName: "plus").font(.system(size: 12, weight: .semibold)) }
                    .buttonStyle(.plain)
                    .foregroundStyle(app.state.layers.isEmpty ? Theme.textFaint : Theme.text)
                    .frame(width: 26, height: 26)
                    .background(RoundedRectangle(cornerRadius: 7).fill(Theme.field))
                    .disabled(app.state.layers.isEmpty)
                    .help(tr("Save all layers as a preset"))
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
                LazyVStack(alignment: .leading, spacing: 6, pinnedViews: [.sectionHeaders]) {
                    ForEach(sections, id: \.0) { title, items in
                        Section {
                            LazyVGrid(columns: [GridItem(.flexible(), spacing: 10), GridItem(.flexible(), spacing: 10)], spacing: 12) {
                                ForEach(items) { p in PresetCard(preset: p) }
                            }
                            .padding(.horizontal, 10)
                            .padding(.bottom, 10)
                        } header: {
                            HStack {
                                Text(tr(title).uppercased()).font(Theme.sectionFont).tracking(0.6).foregroundStyle(Theme.textDim)
                                Spacer()
                                Text("\(items.count)").font(.system(size: 10).monospacedDigit()).foregroundStyle(Theme.textFaint)
                            }
                            .padding(.horizontal, 12).padding(.vertical, 7)
                            .background(Theme.panelBG)
                        }
                    }
                }
                .padding(.vertical, 4)
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

    private func saveStack() { app.saveStackAsPreset() }
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
            Text(preset.displayName).font(.system(size: 11.5, weight: .medium)).foregroundStyle(Theme.text).lineLimit(1)
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

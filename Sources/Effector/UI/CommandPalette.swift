import SwiftUI

struct PaletteItem: Identifiable {
    let id: String
    let title: String
    let subtitle: String
    let symbol: String
    let run: () -> Void
}

struct CommandPalette: View {
    @ObservedObject var app = AppState.shared
    @ObservedObject var presets = PresetStore.shared
    @State private var query = ""
    @State private var index = 0
    @FocusState private var focused: Bool

    private var items: [PaletteItem] {
        var all: [PaletteItem] = []
        for e in EffectRegistry.all {
            all.append(PaletteItem(id: "e-\(e.id)", title: tr(e.name), subtitle: tr("Add effect") + " · " + tr(e.category.label), symbol: e.symbol) {
                app.addEffect(e.id)
            })
        }
        for p in presets.all {
            all.append(PaletteItem(id: "p-\(p.id)", title: p.name, subtitle: tr("Apply preset"), symbol: "wand.and.stars") { app.applyPreset(p) })
        }
        for a in ShortcutAction.allCases where a != .commandPalette {
            all.append(PaletteItem(id: "a-\(a.rawValue)", title: tr(a.label), subtitle: ShortcutStore.shared.combo(a)?.display ?? "", symbol: "command") {
                Actions.perform(a)
            })
        }
        let q = query.lowercased().trimmingCharacters(in: .whitespaces)
        if q.isEmpty { return all }
        return all.filter { $0.title.lowercased().contains(q) || $0.subtitle.lowercased().contains(q) }
            .sorted { ($0.title.lowercased().hasPrefix(q) ? 0 : 1) < ($1.title.lowercased().hasPrefix(q) ? 0 : 1) }
    }

    var body: some View {
        let list = items
        VStack(spacing: 0) {
            HStack(spacing: 10) {
                Image(systemName: "sparkle.magnifyingglass").foregroundStyle(Theme.accentText)
                TextField(tr("Search effects, presets and commands"), text: $query)
                    .textFieldStyle(.plain)
                    .font(.system(size: 15))
                    .focused($focused)
                    .onSubmit { run(list) }
                    .onKeyPress(.downArrow) { index = min(index + 1, list.count - 1); return .handled }
                    .onKeyPress(.upArrow) { index = max(index - 1, 0); return .handled }
                    .onKeyPress(.escape) { app.showCommandPalette = false; return .handled }
            }
            .padding(14)
            Rectangle().fill(Theme.separator).frame(height: 1)
            ScrollViewReader { proxy in
                ScrollView {
                    LazyVStack(spacing: 1) {
                        ForEach(Array(list.prefix(80).enumerated()), id: \.element.id) { i, item in
                            HStack(spacing: 10) {
                                Image(systemName: item.symbol).frame(width: 20).foregroundStyle(i == index ? Theme.accentText : Theme.textDim)
                                Text(item.title).font(.system(size: 13)).foregroundStyle(Theme.text)
                                Spacer()
                                Text(item.subtitle).font(.system(size: 11)).foregroundStyle(Theme.textFaint)
                            }
                            .padding(.horizontal, 12).padding(.vertical, 7)
                            .background(RoundedRectangle(cornerRadius: 7).fill(i == index ? Theme.rowSelected : .clear))
                            .contentShape(Rectangle())
                            .onTapGesture { index = i; run(list) }
                            .id(i)
                        }
                    }
                    .padding(6)
                }
                .onChange(of: index) { _, i in proxy.scrollTo(i) }
            }
            .frame(height: 320)
        }
        .frame(width: 520)
        .background(Theme.panelBG)
        .clipShape(RoundedRectangle(cornerRadius: 14))
        .overlay(RoundedRectangle(cornerRadius: 14).stroke(Theme.separator))
        .shadow(color: .black.opacity(0.4), radius: 30, y: 12)
        .onAppear { focused = true; index = 0 }
        .onChange(of: query) { _, _ in index = 0 }
    }

    private func run(_ list: [PaletteItem]) {
        guard index < list.count else { return }
        let item = list[index]
        app.showCommandPalette = false
        DispatchQueue.main.async { item.run() }
    }
}

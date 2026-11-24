import SwiftUI

struct EffectBrowser: View {
    @ObservedObject var app = AppState.shared
    @State private var query = ""
    @FocusState private var focused: Bool

    private var filtered: [Effect] {
        let q = query.trimmingCharacters(in: .whitespaces).lowercased()
        guard !q.isEmpty else { return EffectRegistry.all }
        return EffectRegistry.all.filter { tr($0.name).lowercased().contains(q) || $0.name.lowercased().contains(q) || tr($0.category.label).lowercased().contains(q) }
    }

    var body: some View {
        VStack(spacing: 0) {
            HStack(spacing: 8) {
                Image(systemName: "magnifyingglass").foregroundStyle(Theme.textFaint)
                TextField(tr("Search effects"), text: $query)
                    .textFieldStyle(.plain)
                    .font(.system(size: 13))
                    .focused($focused)
                    .onSubmit { if let f = filtered.first { add(f) } }
                    .onKeyPress(.escape) { app.showEffectBrowser = false; return .handled }
            }
            .padding(10)
            .background(RoundedRectangle(cornerRadius: 8).fill(Theme.field))
            .padding(12)
            ScrollView {
                VStack(alignment: .leading, spacing: 16) {
                    ForEach(EffectCategory.allCases, id: \.self) { cat in
                        let items = filtered.filter { $0.category == cat }
                        if !items.isEmpty {
                            VStack(alignment: .leading, spacing: 8) {
                                Text(tr(cat.label).uppercased())
                                    .font(Theme.sectionFont).tracking(0.6).foregroundStyle(Theme.textDim)
                                LazyVGrid(columns: [GridItem(.adaptive(minimum: 104), spacing: 10)], spacing: 10) {
                                    ForEach(items, id: \.id) { e in EffectCard(effect: e) { add(e) } }
                                }
                            }
                        }
                    }
                }
                .padding(.horizontal, 12)
                .padding(.bottom, 14)
            }
        }
        .frame(width: 400, height: 560)
        .background(Theme.panelBG)
        .onAppear { focused = true }
    }

    private func add(_ e: Effect) {
        app.addEffect(e.id)
        app.showEffectBrowser = false
    }
}

struct EffectCard: View {
    let effect: Effect
    let action: () -> Void
    @ObservedObject var app = AppState.shared
    @State private var hover = false

    var body: some View {
        Button(action: action) {
            VStack(alignment: .leading, spacing: 6) {
                ZStack {
                    RoundedRectangle(cornerRadius: 7).fill(Theme.field)
                    if let cg = app.effectThumbs[effect.id] {
                        Image(decorative: cg, scale: 1)
                            .resizable()
                            .interpolation(.medium)
                            .aspectRatio(contentMode: .fill)
                    } else {
                        Image(systemName: effect.symbol).font(.system(size: 20)).foregroundStyle(Theme.textFaint)
                    }
                }
                .frame(height: 70)
                .clipShape(RoundedRectangle(cornerRadius: 7))
                .overlay(RoundedRectangle(cornerRadius: 7).stroke(hover ? Theme.accent : Theme.separator, lineWidth: hover ? 1.5 : 1))
                HStack(spacing: 5) {
                    Image(systemName: effect.symbol).font(.system(size: 9.5)).foregroundStyle(Theme.textFaint)
                    Text(tr(effect.name)).font(.system(size: 11.5, weight: .medium)).foregroundStyle(Theme.text).lineLimit(1)
                }
            }
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .onHover { hover = $0 }
    }
}

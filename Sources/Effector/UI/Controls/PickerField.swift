import SwiftUI
import AppKit

/// One entry of a picker list.
struct PickerItem: Identifiable, Hashable {
    let id: String
    let label: String
    var group: String? = nil
    /// Palette preview (small squares).
    var swatches: [RGBA]? = nil
    /// Gradient preview (evenly spaced colours).
    var gradient: [RGBA]? = nil
}

/// Extra command shown at the bottom of a picker list (save, import…).
struct PickerAction: Identifiable {
    let id = UUID()
    let label: String
    var symbol: String? = nil
    var destructive = false
    let action: () -> Void
}

/// Favourite items per picker scope (e.g. "film.stock", "gradient", "blend").
final class FavoritesStore: ObservableObject {
    static let shared = FavoritesStore()
    @Published private var map: [String: [String]] = [:]

    private init() {
        if let d = UserDefaults.standard.data(forKey: "favorites"),
           let m = try? JSONDecoder().decode([String: [String]].self, from: d) { map = m }
    }

    func list(_ scope: String) -> [String] { map[scope] ?? [] }
    func contains(_ scope: String, _ id: String) -> Bool { map[scope]?.contains(id) ?? false }

    func toggle(_ scope: String, _ id: String) {
        var l = map[scope] ?? []
        if let i = l.firstIndex(of: id) { l.remove(at: i) } else { l.append(id) }
        map[scope] = l
        if let d = try? JSONEncoder().encode(map) { UserDefaults.standard.set(d, forKey: "favorites") }
    }
}

/// Dropdown replacement: click to open a list, browse with ↑/↓ (applies live),
/// ↩ keeps, ⎋ reverts. While the field is focused, ↑/↓ also step through the options.
/// Items can be starred to appear first.
struct PickerField: View {
    let scope: String
    let items: [PickerItem]
    let selected: String
    var title: String? = nil
    var actions: [PickerAction] = []
    var expand = false
    let onSelect: (String) -> Void

    @State private var open = false
    @State private var hover = false
    @FocusState private var focused: Bool
    @ObservedObject private var fav = FavoritesStore.shared

    private var current: PickerItem? { items.first { $0.id == selected } }

    /// Navigation order: favourites first, then the list.
    private var order: [String] {
        let favs = fav.list(scope).filter { id in items.contains { $0.id == id } }
        return favs + items.map(\.id).filter { !favs.contains($0) }
    }

    var body: some View {
        HStack(spacing: 5) {
            if let g = current?.gradient { GradientStrip(colors: g).frame(width: 26, height: 10) }
            else if let s = current?.swatches { SwatchStrip(colors: s).frame(width: 26, height: 10) }
            Text(title ?? current?.label ?? selected)
                .font(Theme.labelFont)
                .foregroundStyle(Theme.text)
                .lineLimit(1)
                .truncationMode(.tail)
            if expand { Spacer(minLength: 2) }
            Image(systemName: "chevron.up.chevron.down")
                .font(.system(size: 8.5, weight: .semibold))
                .foregroundStyle(Theme.textDim)
        }
        .padding(.horizontal, 7)
        .padding(.vertical, 3.5)
        .frame(maxWidth: expand ? .infinity : 190, alignment: .leading)
        .fixedSize(horizontal: !expand, vertical: false)
        .background(RoundedRectangle(cornerRadius: 6).fill(open || hover ? Theme.fieldStrong : Theme.field))
        .overlay(RoundedRectangle(cornerRadius: 6).stroke(focused && !open ? Theme.accent.opacity(0.7) : .clear, lineWidth: 1))
        .contentShape(Rectangle())
        .onHover { hover = $0 }
        .focusable()
        .focusEffectDisabled()
        .focused($focused)
        .onTapGesture { focused = true; open.toggle() }
        .onKeyPress(phases: [.down, .repeat]) { press in
            switch press.key {
            case .upArrow: step(-1); return .handled
            case .downArrow: step(1); return .handled
            case .space, .return: open = true; return .handled
            default:
                switch press.characters.lowercased() {
                case "w": step(-1); return .handled
                case "s": step(1); return .handled
                default: return .ignored
                }
            }
        }
        .help(tr("Click to choose · ↑↓ or W/S to browse"))
        .popover(isPresented: $open, arrowEdge: .bottom) {
            PickerList(scope: scope, items: items, initial: selected, actions: actions, onSelect: onSelect) {
                open = false
            }
        }
        .onChange(of: open) { _, o in
            if o { AppState.shared.beginBrowse() } else { AppState.shared.endBrowse(); focused = true }
        }
    }

    private func step(_ d: Int) {
        let o = order
        guard !o.isEmpty else { return }
        let i = o.firstIndex(of: selected) ?? -1
        let n = min(o.count - 1, max(0, i + d))
        if o[n] != selected { onSelect(o[n]) }
    }
}

private struct PickerRow: Identifiable {
    let id: String          // unique row key
    let item: PickerItem
    let header: String?
}

struct PickerList: View {
    let scope: String
    let items: [PickerItem]
    let initial: String
    let actions: [PickerAction]
    let onSelect: (String) -> Void
    let close: () -> Void

    @ObservedObject private var fav = FavoritesStore.shared
    @State private var current = ""
    @State private var highlight = ""
    @State private var typed = ""
    @State private var typedAt = Date.distantPast
    @State private var escMonitor: Any?
    @FocusState private var focused: Bool

    private var rows: [PickerRow] {
        var out: [PickerRow] = []
        let favs = fav.list(scope).compactMap { id in items.first { $0.id == id } }
        for (i, it) in favs.enumerated() {
            out.append(PickerRow(id: "★" + it.id, item: it, header: i == 0 ? "★ " + tr("Favorites") : nil))
        }
        var last: String? = "\u{0}"
        for (i, it) in items.enumerated() {
            var header: String? = nil
            if it.group != last {
                header = it.group ?? (i == 0 && !favs.isEmpty ? tr("All") : nil)
                last = it.group
            } else if i == 0 && !favs.isEmpty {
                header = tr("All")
            }
            out.append(PickerRow(id: it.id, item: it, header: header))
        }
        return out
    }

    var body: some View {
        let rows = self.rows
        VStack(spacing: 0) {
            ScrollViewReader { proxy in
                ScrollView {
                    LazyVStack(alignment: .leading, spacing: 0) {
                        ForEach(rows) { r in
                            if let h = r.header {
                                Text(h.uppercased())
                                    .font(.system(size: 9.5, weight: .semibold)).tracking(0.5)
                                    .foregroundStyle(Theme.textFaint)
                                    .padding(.horizontal, 10).padding(.top, 8).padding(.bottom, 3)
                            }
                            row(r).id(r.id)
                        }
                    }
                    .padding(.vertical, 5)
                }
                .frame(maxHeight: 440)
                .onAppear {
                    current = initial
                    highlight = rows.first { $0.item.id == initial && !$0.id.hasPrefix("★") }?.id ?? rows.first?.id ?? ""
                    focused = true
                    DispatchQueue.main.async { proxy.scrollTo(highlight, anchor: .center) }
                }
                .onChange(of: highlight) { _, h in withAnimation(.linear(duration: 0.08)) { proxy.scrollTo(h) } }
            }
            if !actions.isEmpty {
                Rectangle().fill(Theme.separator).frame(height: 1)
                VStack(alignment: .leading, spacing: 0) {
                    ForEach(actions) { a in
                        ActionRow(action: a) { close(); DispatchQueue.main.async { a.action() } }
                    }
                }
                .padding(.vertical, 4)
            }
            Rectangle().fill(Theme.separator).frame(height: 1)
            Text(tr("↑↓ / W S browse · ↩ keep · ⎋ revert · ★ favorite"))
                .font(.system(size: 9.5)).foregroundStyle(Theme.textFaint)
                .frame(maxWidth: .infinity)
                .padding(.vertical, 5)
        }
        .frame(width: 270)
        .background(Theme.panelBG)
        .focusable()
        .focusEffectDisabled()
        .focused($focused)
        .onKeyPress(phases: [.down, .repeat]) { press in handle(press, rows) }
        // NSPopover swallows ⎋ before SwiftUI sees it: catch it first to revert the live preview.
        .onAppear {
            escMonitor = NSEvent.addLocalMonitorForEvents(matching: .keyDown) { e in
                if e.keyCode == 53 { revertAndClose(); return nil }
                return e
            }
        }
        .onDisappear {
            if let m = escMonitor { NSEvent.removeMonitor(m); escMonitor = nil }
        }
    }

    private func revertAndClose() {
        if current != initial { onSelect(initial); current = initial }
        close()
    }

    private func handle(_ press: KeyPress, _ rows: [PickerRow]) -> KeyPress.Result {
        switch press.key {
        case .upArrow: move(-1, rows); return .handled
        case .downArrow: move(1, rows); return .handled
        case .return: close(); return .handled
        case .escape:
            revertAndClose(); return .handled
        default:
            let ch = press.characters.lowercased()
            if ch == "w" { move(-1, rows); return .handled }
            if ch == "s" { move(1, rows); return .handled }
            guard ch.count == 1, let c = ch.first, c.isLetter || c.isNumber else { return .ignored }
            if Date().timeIntervalSince(typedAt) > 0.9 { typed = "" }
            typed += ch; typedAt = Date()
            if let r = rows.first(where: { !$0.id.hasPrefix("★") && $0.item.label.lowercased().hasPrefix(typed) }) {
                choose(r)
            }
            return .handled
        }
    }

    private func move(_ d: Int, _ rows: [PickerRow]) {
        guard !rows.isEmpty else { return }
        let i = rows.firstIndex { $0.id == highlight } ?? -1
        let n = min(rows.count - 1, max(0, i + d))
        choose(rows[n])
    }

    private func choose(_ r: PickerRow) {
        highlight = r.id
        if r.item.id != current {
            current = r.item.id
            onSelect(r.item.id)
        }
    }

    @ViewBuilder
    private func row(_ r: PickerRow) -> some View {
        PickerRowView(item: r.item, highlighted: r.id == highlight, checked: r.item.id == current,
                      favorite: fav.contains(scope, r.item.id),
                      toggleFav: { fav.toggle(scope, r.item.id) }) {
            choose(r)
            close()
        }
    }
}

private struct PickerRowView: View {
    let item: PickerItem
    let highlighted: Bool
    let checked: Bool
    let favorite: Bool
    let toggleFav: () -> Void
    let tap: () -> Void
    @State private var hover = false

    var body: some View {
        HStack(spacing: 7) {
            Image(systemName: "checkmark")
                .font(.system(size: 9, weight: .bold))
                .foregroundStyle(Theme.accentText)
                .opacity(checked ? 1 : 0)
                .frame(width: 10)
            if let g = item.gradient { GradientStrip(colors: g).frame(width: 46, height: 12) }
            else if let s = item.swatches { SwatchStrip(colors: s).frame(width: 46, height: 12) }
            Text(item.label)
                .font(.system(size: 12))
                .foregroundStyle(highlighted ? Theme.text : Theme.text.opacity(0.88))
                .lineLimit(1)
            Spacer(minLength: 4)
            Image(systemName: favorite ? "star.fill" : "star")
                .font(.system(size: 10))
                .foregroundStyle(favorite ? Theme.accent : Theme.textFaint)
                .opacity(favorite || hover ? 1 : 0.25)
                .frame(width: 18, height: 18)
                .contentShape(Rectangle())
                .onTapGesture { toggleFav() }
                .help(tr("Favorite"))
        }
        .padding(.horizontal, 8)
        .frame(height: 24)
        .background(RoundedRectangle(cornerRadius: 5)
            .fill(highlighted ? Theme.rowSelected : (hover ? Theme.rowHover : .clear))
            .padding(.horizontal, 4))
        .contentShape(Rectangle())
        .onHover { hover = $0 }
        .onTapGesture { tap() }
    }
}

private struct ActionRow: View {
    let action: PickerAction
    let tap: () -> Void
    @State private var hover = false
    var body: some View {
        HStack(spacing: 7) {
            Image(systemName: action.symbol ?? "circle").font(.system(size: 10.5)).frame(width: 14)
                .opacity(action.symbol == nil ? 0 : 1)
            Text(action.label).font(.system(size: 12))
            Spacer()
        }
        .foregroundStyle(action.destructive ? Color.red : Theme.text)
        .padding(.horizontal, 10)
        .frame(height: 24)
        .background(RoundedRectangle(cornerRadius: 5).fill(hover ? Theme.rowHover : .clear).padding(.horizontal, 4))
        .contentShape(Rectangle())
        .onHover { hover = $0 }
        .onTapGesture { tap() }
    }
}

struct GradientStrip: View {
    let colors: [RGBA]
    var body: some View {
        RoundedRectangle(cornerRadius: 3)
            .fill(LinearGradient(colors: colors.map(\.color), startPoint: .leading, endPoint: .trailing))
            .overlay(RoundedRectangle(cornerRadius: 3).stroke(Theme.separator, lineWidth: 0.5))
    }
}

struct SwatchStrip: View {
    let colors: [RGBA]
    var body: some View {
        HStack(spacing: 0) {
            ForEach(Array(colors.prefix(16).enumerated()), id: \.offset) { _, c in Rectangle().fill(c.color) }
        }
        .clipShape(RoundedRectangle(cornerRadius: 3))
        .overlay(RoundedRectangle(cornerRadius: 3).stroke(Theme.separator, lineWidth: 0.5))
    }
}

extension GradientSpec {
    /// Evenly sampled colours for previews.
    var previewColors: [RGBA] { (0..<12).map { sample(Double($0) / 11) } }
}

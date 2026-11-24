import SwiftUI
import AppKit

/// Routes arrow / return / escape keys to the open browser panel (installed as a local monitor so it
/// works while the search field has focus).
final class BrowserKeyRouter: ObservableObject {
    var handler: ((UInt16) -> Bool)?
    private var monitor: Any?

    func install() {
        guard monitor == nil else { return }
        monitor = NSEvent.addLocalMonitorForEvents(matching: .keyDown) { [weak self] e in
            guard let self, NSApp.keyWindow?.attachedSheet == nil,
                  e.modifierFlags.intersection([.command, .control, .option]).isEmpty else { return e }
            return self.handler?(e.keyCode) == true ? nil : e
        }
    }
    func remove() {
        if let m = monitor { NSEvent.removeMonitor(m); monitor = nil }
    }
}

enum KeyCode {
    static let left: UInt16 = 123, right: UInt16 = 124, down: UInt16 = 125, up: UInt16 = 126
    static let returnKey: UInt16 = 36, enter: UInt16 = 76, escape: UInt16 = 53
}

/// Grid / list toggle used by both browser panels.
struct ViewModeToggle: View {
    @Binding var listMode: Bool
    var body: some View {
        HStack(spacing: 2) {
            button("square.grid.2x2", false, "Grid")
            button("list.bullet", true, "List")
        }
        .padding(2)
        .background(RoundedRectangle(cornerRadius: 7).fill(Theme.field))
    }
    private func button(_ symbol: String, _ list: Bool, _ help: String) -> some View {
        Image(systemName: symbol)
            .font(.system(size: 11, weight: .medium))
            .foregroundStyle(listMode == list ? Theme.text : Theme.textDim)
            .frame(width: 28, height: 26)
            .background(RoundedRectangle(cornerRadius: 5).fill(listMode == list ? Theme.fieldStrong : .clear))
            .contentShape(Rectangle())
            .onTapGesture { listMode = list }
            .help(tr(help))
    }
}

struct SearchField: View {
    let placeholder: String
    @Binding var text: String
    var focused: FocusState<Bool>.Binding
    var body: some View {
        HStack(spacing: 8) {
            Image(systemName: "magnifyingglass").foregroundStyle(Theme.textFaint)
            TextField(placeholder, text: $text)
                .textFieldStyle(.plain)
                .font(.system(size: 13))
                .focused(focused)
            if !text.isEmpty {
                Image(systemName: "xmark.circle.fill").foregroundStyle(Theme.textFaint)
                    .onTapGesture { text = "" }
                    .help(tr("Clear search"))
            }
        }
        .padding(10)
        .background(RoundedRectangle(cornerRadius: 8).fill(Theme.field))
    }
}

struct BrowserFooter: View {
    let text: String
    var body: some View {
        Text(text)
            .font(.system(size: 10)).foregroundStyle(Theme.textFaint)
            .frame(maxWidth: .infinity)
            .padding(.vertical, 6)
            .overlay(alignment: .top) { Rectangle().fill(Theme.separator).frame(height: 1) }
    }
}

struct EffectBrowser: View {
    @ObservedObject var app = AppState.shared
    @State private var query = ""
    @State private var highlight: String?
    @AppStorage("effectBrowserList") private var listMode = false
    @FocusState private var focused: Bool
    @StateObject private var keys = BrowserKeyRouter()
    /// Mouse position at the last keyboard navigation: hovers are ignored until the mouse really moves
    /// (scrolling the list under a still pointer must not steal the highlight).
    @State private var keyNavMouse: CGPoint?

    private var filtered: [Effect] {
        let q = query.trimmingCharacters(in: .whitespaces).lowercased()
        let all = EffectCategory.allCases.flatMap { c in EffectRegistry.all.filter { $0.category == c } }
        guard !q.isEmpty else { return all }
        return all.filter {
            tr($0.name).lowercased().contains(q) || $0.name.lowercased().contains(q)
                || tr($0.category.label).lowercased().contains(q) || tr($0.summary).lowercased().contains(q)
        }
    }

    var body: some View {
        let list = filtered
        let _ = (keys.handler = { code in handleKey(code, list) })
        VStack(spacing: 0) {
            HStack(spacing: 8) {
                SearchField(placeholder: tr("Search effects"), text: $query, focused: $focused)
                    .onSubmit { if let e = list.first(where: { $0.id == highlight }) ?? list.first { add(e) } }
                ViewModeToggle(listMode: $listMode)
            }
            .padding(12)
            ScrollViewReader { proxy in
                ScrollView {
                    VStack(alignment: .leading, spacing: 16) {
                        ForEach(EffectCategory.allCases, id: \.self) { cat in
                            let items = list.filter { $0.category == cat }
                            if !items.isEmpty {
                                VStack(alignment: .leading, spacing: 8) {
                                    Text(tr(cat.label).uppercased())
                                        .font(Theme.sectionFont).tracking(0.6).foregroundStyle(Theme.textDim)
                                    if listMode {
                                        VStack(spacing: 2) {
                                            ForEach(items, id: \.id) { e in
                                                EffectListRow(effect: e, highlighted: highlight == e.id, hover: { hover(e) }) { add(e) }.id(e.id)
                                            }
                                        }
                                    } else {
                                        LazyVGrid(columns: [GridItem(.adaptive(minimum: 116), spacing: 10)], spacing: 10) {
                                            ForEach(items, id: \.id) { e in
                                                EffectCard(effect: e, highlighted: highlight == e.id, hover: { hover(e) }) { add(e) }.id(e.id)
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    .padding(.horizontal, 12)
                    .padding(.bottom, 14)
                }
                .onChange(of: highlight) { _, h in if let h { withAnimation(.linear(duration: 0.1)) { proxy.scrollTo(h) } } }
            }
            BrowserFooter(text: tr("Hover or ↑↓ to preview · click or ↩ to add · ⎋ close"))
        }
        .frame(width: 440)
        .frame(maxHeight: .infinity)
        .background(Theme.panelBG)
        .onAppear { DispatchQueue.main.asyncAfter(deadline: .now() + 0.05) { focused = true }; keys.install() }
        .onDisappear { keys.remove(); app.previewLayers = nil }
        .onChange(of: query) { _, _ in highlight = nil; app.hoverPreview(nil) }
    }

    private func preview(_ e: Effect) { app.hoverPreview([LayerNode.effect(e.id)]) }

    private func hover(_ e: Effect) {
        if let m = keyNavMouse { if m == NSEvent.mouseLocation { return }; keyNavMouse = nil }
        highlight = e.id
        preview(e)
    }

    private func handleKey(_ code: UInt16, _ list: [Effect]) -> Bool {
        switch code {
        case KeyCode.escape: app.showEffectBrowser = false; return true
        case KeyCode.up, KeyCode.down, KeyCode.left, KeyCode.right:
            guard !list.isEmpty else { return true }
            let d = (code == KeyCode.up || code == KeyCode.left) ? -1 : 1
            let i = list.firstIndex { $0.id == highlight } ?? (d > 0 ? -1 : list.count)
            let n = min(list.count - 1, max(0, i + d))
            keyNavMouse = NSEvent.mouseLocation
            highlight = list[n].id
            preview(list[n])
            return true
        case KeyCode.returnKey, KeyCode.enter:
            if let e = list.first(where: { $0.id == highlight }) ?? list.first { add(e) }
            return true
        default: return false
        }
    }

    private func add(_ e: Effect) {
        app.previewLayers = nil
        app.addEffect(e.id)
        app.showEffectBrowser = false
    }
}

struct EffectCard: View {
    let effect: Effect
    var highlighted = false
    var hover: () -> Void = {}
    let action: () -> Void
    @ObservedObject var app = AppState.shared
    @State private var isHover = false

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
                .overlay(RoundedRectangle(cornerRadius: 7).stroke(highlighted || isHover ? Theme.accent : Theme.separator,
                                                                  lineWidth: highlighted || isHover ? 1.5 : 1))
                HStack(spacing: 5) {
                    Image(systemName: effect.symbol).font(.system(size: 9.5)).foregroundStyle(Theme.textFaint)
                    Text(tr(effect.name)).font(.system(size: 11.5, weight: .medium)).foregroundStyle(Theme.text).lineLimit(1)
                }
            }
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .onHover { h in isHover = h; if h { hover() } }
        .help(tr(effect.summary))
    }
}

struct EffectListRow: View {
    let effect: Effect
    var highlighted = false
    var hover: () -> Void = {}
    let action: () -> Void
    @ObservedObject var app = AppState.shared
    @State private var isHover = false

    var body: some View {
        HStack(spacing: 10) {
            ZStack {
                RoundedRectangle(cornerRadius: 6).fill(Theme.field)
                if let cg = app.effectThumbs[effect.id] {
                    Image(decorative: cg, scale: 1).resizable().interpolation(.medium).aspectRatio(contentMode: .fill)
                } else {
                    Image(systemName: effect.symbol).font(.system(size: 14)).foregroundStyle(Theme.textFaint)
                }
            }
            .frame(width: 64, height: 42)
            .clipShape(RoundedRectangle(cornerRadius: 6))
            .overlay(RoundedRectangle(cornerRadius: 6).stroke(highlighted || isHover ? Theme.accent : Theme.separator,
                                                              lineWidth: highlighted || isHover ? 1.5 : 1))
            VStack(alignment: .leading, spacing: 2) {
                HStack(spacing: 5) {
                    Image(systemName: effect.symbol).font(.system(size: 10)).foregroundStyle(Theme.textFaint)
                    Text(tr(effect.name)).font(.system(size: 12.5, weight: .medium)).foregroundStyle(Theme.text)
                }
                if !effect.summary.isEmpty {
                    Text(tr(effect.summary)).font(.system(size: 11)).foregroundStyle(Theme.textDim).lineLimit(2)
                }
            }
            Spacer(minLength: 0)
        }
        .padding(6)
        .background(RoundedRectangle(cornerRadius: 8).fill(highlighted ? Theme.rowSelected : (isHover ? Theme.rowHover : .clear)))
        .contentShape(Rectangle())
        .onHover { h in isHover = h; if h { hover() } }
        .onTapGesture { action() }
    }
}

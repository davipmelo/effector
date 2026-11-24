import Foundation
import AppKit
import SwiftUI

struct KeyCombo: Codable, Hashable {
    var key: String            // lowercased character or special name
    var cmd = false, shift = false, opt = false, ctrl = false

    var display: String {
        (ctrl ? "⌃" : "") + (opt ? "⌥" : "") + (shift ? "⇧" : "") + (cmd ? "⌘" : "") + KeyCombo.keyLabel(key)
    }

    static func keyLabel(_ k: String) -> String {
        switch k {
        case "delete": return "⌫"
        case "fdelete": return "⌦"
        case "left": return "←"
        case "right": return "→"
        case "up": return "↑"
        case "down": return "↓"
        case "space": return "Space"
        case "return": return "↩"
        case "tab": return "⇥"
        case "escape": return "⎋"
        default: return k.uppercased()
        }
    }

    static func from(_ e: NSEvent) -> KeyCombo? {
        let special: [UInt16: String] = [51: "delete", 117: "fdelete", 123: "left", 124: "right", 126: "up", 125: "down",
                                         49: "space", 36: "return", 48: "tab", 53: "escape"]
        let k: String
        if let s = special[e.keyCode] { k = s }
        else if let c = e.charactersIgnoringModifiers?.lowercased(), !c.isEmpty { k = c }
        else { return nil }
        let f = e.modifierFlags
        var combo = KeyCombo(key: k, cmd: f.contains(.command), shift: f.contains(.shift), opt: f.contains(.option), ctrl: f.contains(.control))
        // ⇧ changes characters (e.g. ⇧= is +); normalize common cases
        if combo.shift, k == "+" { combo.key = "=" }
        return combo
    }

    var keyboardShortcut: KeyboardShortcut? {
        let ke: KeyEquivalent
        switch key {
        case "delete": ke = .delete
        case "fdelete": ke = .deleteForward
        case "left": ke = .leftArrow
        case "right": ke = .rightArrow
        case "up": ke = .upArrow
        case "down": ke = .downArrow
        case "space": ke = .space
        case "return": ke = .return
        case "tab": ke = .tab
        case "escape": ke = .escape
        default:
            guard let c = key.first, key.count == 1 else { return nil }
            ke = KeyEquivalent(c)
        }
        var m: EventModifiers = []
        if cmd { m.insert(.command) }
        if shift { m.insert(.shift) }
        if opt { m.insert(.option) }
        if ctrl { m.insert(.control) }
        return KeyboardShortcut(ke, modifiers: m)
    }

    var hasModifier: Bool { cmd || ctrl }
}

enum ShortcutAction: String, CaseIterable, Codable {
    case open, save, saveAs, export, copyResult, paste, undo, redo
    case commandPalette, addEffect, beforeAfter, crop
    case group, ungroup, duplicate, delete, toggleVisibility, moveUp, moveDown
    case zoomIn, zoomOut, zoomFit, zoom100, showLayers, showPresets

    var label: String {
        switch self {
        case .open: return "Open…"
        case .save: return "Save Project"
        case .saveAs: return "Save Project As…"
        case .export: return "Export Image…"
        case .copyResult: return "Copy Result"
        case .paste: return "Paste Image"
        case .undo: return "Undo"
        case .redo: return "Redo"
        case .commandPalette: return "Command Palette"
        case .addEffect: return "Add Effect…"
        case .beforeAfter: return "Toggle Before / After"
        case .crop: return "Crop"
        case .group: return "Group Layer"
        case .ungroup: return "Ungroup"
        case .duplicate: return "Duplicate Layer"
        case .delete: return "Delete Layer"
        case .toggleVisibility: return "Show / Hide Layer"
        case .moveUp: return "Move Layer Up"
        case .moveDown: return "Move Layer Down"
        case .zoomIn: return "Zoom In"
        case .zoomOut: return "Zoom Out"
        case .zoomFit: return "Fit to Window"
        case .zoom100: return "Actual Size (100%)"
        case .showLayers: return "Show Layers"
        case .showPresets: return "Show Presets"
        }
    }

    var defaultCombo: KeyCombo? {
        switch self {
        case .open: return KeyCombo(key: "o", cmd: true)
        case .save: return KeyCombo(key: "s", cmd: true)
        case .saveAs: return KeyCombo(key: "s", cmd: true, shift: true)
        case .export: return KeyCombo(key: "e", cmd: true)
        case .copyResult: return KeyCombo(key: "c", cmd: true, shift: true)
        case .paste: return KeyCombo(key: "v", cmd: true)
        case .undo: return KeyCombo(key: "z", cmd: true)
        case .redo: return KeyCombo(key: "z", cmd: true, shift: true)
        case .commandPalette: return KeyCombo(key: "k", cmd: true)
        case .addEffect: return KeyCombo(key: "a", cmd: true, shift: true)
        case .beforeAfter: return KeyCombo(key: "\\")
        case .crop: return KeyCombo(key: "c")
        case .group: return KeyCombo(key: "g", cmd: true)
        case .ungroup: return KeyCombo(key: "g", cmd: true, shift: true)
        case .duplicate: return KeyCombo(key: "d", cmd: true)
        case .delete: return KeyCombo(key: "delete", cmd: true)
        case .toggleVisibility: return KeyCombo(key: "h")
        case .moveUp: return KeyCombo(key: "]", cmd: true)
        case .moveDown: return KeyCombo(key: "[", cmd: true)
        case .zoomIn: return KeyCombo(key: "=", cmd: true)
        case .zoomOut: return KeyCombo(key: "-", cmd: true)
        case .zoomFit: return KeyCombo(key: "z")
        case .zoom100: return KeyCombo(key: "1", cmd: true)
        case .showLayers: return KeyCombo(key: "l")
        case .showPresets: return KeyCombo(key: "p")
        }
    }

    /// Standard text-editing shortcuts that must reach text fields while typing.
    var isTextEditingStandard: Bool { [.undo, .redo, .paste, .copyResult, .delete].contains(self) }
}

final class ShortcutStore: ObservableObject {
    static let shared = ShortcutStore()
    @Published private(set) var custom: [ShortcutAction: KeyCombo?] = [:]
    @Published var recording: ShortcutAction?
    private var monitor: Any?

    private init() {
        if let d = UserDefaults.standard.data(forKey: "shortcuts"),
           let m = try? JSONDecoder().decode([String: KeyCombo?].self, from: d) {
            for (k, v) in m { if let a = ShortcutAction(rawValue: k) { custom[a] = v } }
        }
    }

    func combo(_ a: ShortcutAction) -> KeyCombo? {
        if let c = custom[a] { return c }
        return a.defaultCombo
    }

    func set(_ a: ShortcutAction, _ c: KeyCombo?) {
        // Remove the combo from any other action to avoid conflicts.
        if let c {
            for other in ShortcutAction.allCases where other != a && combo(other) == c { custom[other] = .some(nil) }
        }
        custom[a] = .some(c)
        persist()
    }

    func resetAll() { custom = [:]; persist() }

    private func persist() {
        var m: [String: KeyCombo?] = [:]
        for (k, v) in custom { m[k.rawValue] = v }
        if let d = try? JSONEncoder().encode(m) { UserDefaults.standard.set(d, forKey: "shortcuts") }
    }

    func shortcut(_ a: ShortcutAction) -> KeyboardShortcut? { combo(a)?.keyboardShortcut }

    // MARK: Event monitor

    func install() {
        guard monitor == nil else { return }
        monitor = NSEvent.addLocalMonitorForEvents(matching: .keyDown) { [weak self] e in
            guard let self else { return e }
            return self.handle(e) ? nil : e
        }
    }

    private func isEditingText(_ w: NSWindow?) -> Bool {
        guard let r = w?.firstResponder else { return false }
        return r is NSText || r is NSTextView
    }

    private func handle(_ e: NSEvent) -> Bool {
        guard let pressed = KeyCombo.from(e) else { return false }
        let combo = pressed
        if let rec = recording {
            if combo.key == "escape" && !combo.hasModifier { recording = nil; return true }
            if combo.key == "delete" && !combo.hasModifier { set(rec, nil); recording = nil; return true }
            set(rec, combo)
            recording = nil
            return true
        }
        guard let w = e.window, w.identifier?.rawValue.hasPrefix("main") == true || w == NSApp.mainWindow,
              w.attachedSheet == nil else { return false }
        // Ignore when a non-main panel (popover, settings) is key.
        if NSApp.keyWindow != nil && NSApp.keyWindow != w { return false }
        let editing = isEditingText(w)
        guard let action = ShortcutAction.allCases.first(where: { self.combo($0) == pressed }) else { return false }
        if editing && (!combo.hasModifier || action.isTextEditingStandard) { return false }
        DispatchQueue.main.async { Actions.perform(action) }
        return true
    }

    private func combo(for a: ShortcutAction) -> KeyCombo? { combo(a) }
}

enum Actions {
    static func perform(_ a: ShortcutAction) {
        let app = AppState.shared
        switch a {
        case .open: if app.confirmDiscardChanges() { app.openPanel() }
        case .save: app.save()
        case .saveAs: app.save(as: true)
        case .export: if app.hasImage { app.showExport = true }
        case .copyResult: app.copyResult()
        case .paste: if app.confirmDiscardChanges() { app.pasteImage() }
        case .undo: app.undo()
        case .redo: app.redo()
        case .commandPalette: if app.hasImage { app.showCommandPalette.toggle() }
        case .addEffect: if app.hasImage { app.sidebarTab = 0; app.showEffectBrowser = true }
        case .beforeAfter: if app.hasImage { app.showOriginal.toggle() }
        case .crop: if app.isCropping { app.commitCrop() } else { app.beginCrop() }
        case .group: app.groupSelected()
        case .ungroup: app.ungroupSelected()
        case .duplicate: app.duplicateSelected()
        case .delete: app.deleteSelected()
        case .toggleVisibility: if let id = app.selectedID { app.toggleVisibility(id) }
        case .moveUp: app.nudgeSelected(-1)
        case .moveDown: app.nudgeSelected(1)
        case .zoomIn: app.zoomIn()
        case .zoomOut: app.zoomOut()
        case .zoomFit: app.setZoom(nil)
        case .zoom100: app.zoomActual()
        case .showLayers: app.sidebarTab = 0
        case .showPresets: app.sidebarTab = 1
        }
    }
}

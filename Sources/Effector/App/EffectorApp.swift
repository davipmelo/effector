import SwiftUI
import AppKit

final class AppDelegate: NSObject, NSApplicationDelegate {
    func applicationDidFinishLaunching(_ notification: Notification) {
        AppSettings.shared.applyTheme()
        ShortcutStore.shared.install()
        NSApp.activate()
        // give a file opened from Finder a moment to arrive before restoring the previous session
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.35) { AppState.shared.restoreSession() }
    }

    func application(_ application: NSApplication, open urls: [URL]) {
        AppState.shared.open(urls: urls)
    }

    func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool { true }

    func applicationShouldTerminate(_ sender: NSApplication) -> NSApplication.TerminateReply {
        // With session restore on, unsaved work comes back on the next launch: no need to ask.
        if AppSettings.shared.reopenLastSession { AppState.shared.saveSession(sync: true); return .terminateNow }
        return AppState.shared.confirmDiscardChanges() ? .terminateNow : .terminateCancel
    }
}

@main
struct EffectorApp: App {
    @NSApplicationDelegateAdaptor(AppDelegate.self) var delegate
    @ObservedObject var settings = AppSettings.shared
    @ObservedObject var shortcuts = ShortcutStore.shared
    @ObservedObject var app = AppState.shared

    init() {
        if CommandLine.arguments.contains("--render-test") {
            RenderTest.run(CommandLine.arguments)
            exit(0)
        }
    }

    var body: some Scene {
        Window("EFFECTOR", id: "main") {
            MainView()
                .frame(minWidth: 1000, minHeight: 640)
        }
        .windowToolbarStyle(.unified(showsTitle: false))
        .defaultSize(width: 1440, height: 900)
        .commands { menus }

        Settings {
            SettingsView()
                .preferredColorScheme(settings.theme.colorScheme)
        }
    }

    private func item(_ a: ShortcutAction, disabled: Bool = false) -> some View {
        let b = Button(tr(a.label)) { Actions.perform(a) }.disabled(disabled)
        // Only show modifier shortcuts in menus; plain keys are handled by the key monitor.
        if let c = shortcuts.combo(a), c.hasModifier, let ks = c.keyboardShortcut {
            return AnyView(b.keyboardShortcut(ks))
        }
        return AnyView(b)
    }

    @CommandsBuilder
    private var menus: some Commands {
        let _ = settings.languageRevision
        CommandGroup(replacing: .newItem) {
            item(.open)
            Divider()
            item(.save, disabled: !app.hasImage)
            item(.saveAs, disabled: !app.hasImage)
            Divider()
            item(.closeImage, disabled: !app.hasImage)
            Divider()
            item(.export, disabled: !app.hasImage)
            item(.copyResult, disabled: !app.hasImage)
        }
        CommandGroup(replacing: .undoRedo) {
            item(.undo, disabled: !app.canUndo)
            item(.redo, disabled: !app.canRedo)
        }
        CommandGroup(replacing: .pasteboard) {
            Button(tr("Cut")) { NSApp.sendAction(#selector(NSText.cut(_:)), to: nil, from: nil) }.keyboardShortcut("x")
            Button(tr("Copy")) { NSApp.sendAction(#selector(NSText.copy(_:)), to: nil, from: nil) }.keyboardShortcut("c")
            item(.paste)
            Button(tr("Select All")) { NSApp.sendAction(#selector(NSText.selectAll(_:)), to: nil, from: nil) }.keyboardShortcut("a")
        }
        CommandMenu(tr("Image")) {
            item(.crop, disabled: !app.hasImage)
            item(.beforeAfter, disabled: !app.hasImage)
            item(.compare, disabled: !app.hasImage)
            Divider()
            item(.syncSettings, disabled: app.stripSelection.count < 2)
            Button(tr("Copy Settings")) { AppState.shared.copySettings() }.disabled(!app.hasImage)
            Button(tr("Paste Settings to Selected Images")) { AppState.shared.pasteSettings(to: AppState.shared.stripSelection) }
                .disabled(!app.hasCopiedSettings)
        }
        CommandMenu(tr("Layer")) {
            item(.commandPalette, disabled: !app.hasImage)
            item(.addEffect, disabled: !app.hasImage)
            Divider()
            item(.group, disabled: !app.hasImage)
            item(.ungroup, disabled: app.selectedLayer?.isGroup != true)
            item(.duplicate, disabled: app.selectedID == nil)
            item(.delete, disabled: app.selectedIDs.isEmpty)
            item(.toggleVisibility, disabled: app.selectedIDs.isEmpty)
            item(.randomize, disabled: app.selectedLayer == nil || app.selectedLayer?.isGroup == true)
            Divider()
            item(.moveUp, disabled: app.selectedID == nil)
            item(.moveDown, disabled: app.selectedID == nil)
        }
        CommandGroup(after: .pasteboard) {
            Divider()
            item(.copyLayers, disabled: app.selectedIDs.isEmpty)
            item(.pasteLayers, disabled: !app.hasImage)
            Button(tr("Select All Layers")) { AppState.shared.selectAllLayers() }
                .keyboardShortcut("a", modifiers: [.command, .option])
                .disabled(!app.hasImage)
        }
        CommandGroup(after: .toolbar) {
            item(.zoomIn); item(.zoomOut)
            // ⌘0 always fits; the single-key shortcut (Z by default) is handled by the key monitor
            Button(tr(ShortcutAction.zoomFit.label)) { Actions.perform(.zoomFit) }.keyboardShortcut("0", modifiers: .command)
            item(.zoom100)
            Button(tr(ShortcutAction.present.label)) { Actions.perform(.present) }.disabled(!app.hasImage)
            Divider()
            item(.filmstrip)
            item(.prevImage, disabled: app.docs.count < 2)
            item(.nextImage, disabled: app.docs.count < 2)
            Divider()
            item(.showLayers); item(.showPresets)
            Divider()
        }
    }
}

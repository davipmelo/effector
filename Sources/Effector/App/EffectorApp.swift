import SwiftUI
import AppKit

final class AppDelegate: NSObject, NSApplicationDelegate {
    func applicationDidFinishLaunching(_ notification: Notification) {
        AppSettings.shared.applyTheme()
        ShortcutStore.shared.install()
        NSApp.activate()
    }

    func application(_ application: NSApplication, open urls: [URL]) {
        guard let u = urls.first else { return }
        if AppState.shared.confirmDiscardChanges() { AppState.shared.open(url: u) }
    }

    func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool { true }

    func applicationShouldTerminate(_ sender: NSApplication) -> NSApplication.TerminateReply {
        AppState.shared.confirmDiscardChanges() ? .terminateNow : .terminateCancel
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
        .windowToolbarStyle(.unified(showsTitle: true))
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
        }
        CommandMenu(tr("Layer")) {
            item(.commandPalette, disabled: !app.hasImage)
            item(.addEffect, disabled: !app.hasImage)
            Divider()
            item(.group, disabled: !app.hasImage)
            item(.ungroup, disabled: app.selectedLayer?.isGroup != true)
            item(.duplicate, disabled: app.selectedID == nil)
            item(.delete, disabled: app.selectedID == nil)
            item(.toggleVisibility, disabled: app.selectedID == nil)
            Divider()
            item(.moveUp, disabled: app.selectedID == nil)
            item(.moveDown, disabled: app.selectedID == nil)
        }
        CommandGroup(after: .toolbar) {
            item(.zoomIn); item(.zoomOut)
            // ⌘0 always fits; the single-key shortcut (Z by default) is handled by the key monitor
            Button(tr(ShortcutAction.zoomFit.label)) { Actions.perform(.zoomFit) }.keyboardShortcut("0", modifiers: .command)
            item(.zoom100)
            Divider()
            item(.showLayers); item(.showPresets)
            Divider()
        }
    }
}

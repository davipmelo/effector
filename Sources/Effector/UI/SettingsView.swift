import SwiftUI

struct SettingsView: View {
    @ObservedObject var settings = AppSettings.shared
    @State private var tab = 0

    var body: some View {
        let _ = settings.languageRevision
        TabView(selection: $tab) {
            GeneralSettings().tabItem { Label(tr("General"), systemImage: "gearshape") }.tag(0)
            ShortcutSettings().tabItem { Label(tr("Shortcuts"), systemImage: "keyboard") }.tag(1)
            ExportSettings().tabItem { Label(tr("Export"), systemImage: "square.and.arrow.up") }.tag(2)
        }
        .frame(width: 520, height: 520)
        .tint(Theme.accent)
    }
}

struct GeneralSettings: View {
    @ObservedObject var settings = AppSettings.shared
    var body: some View {
        Form {
            Picker(tr("Language"), selection: $settings.language) {
                ForEach(AppLanguage.allCases, id: \.self) { Text($0.label).tag($0) }
            }
            Text(tr("System menus (File, Edit…) switch language after reopening the app."))
                .font(.system(size: 11)).foregroundStyle(.secondary)
            Picker(tr("Appearance"), selection: $settings.theme) {
                ForEach(AppTheme.allCases, id: \.self) { Text($0.label).tag($0) }
            }
            .pickerStyle(.segmented)
            Section {
                Toggle(tr("Reopen the last image and layers on launch"), isOn: $settings.reopenLastSession)
                Toggle(tr("Preview effects and presets on hover"), isOn: $settings.hoverPreview)
                Text(tr("Settings, shortcuts, favorites, presets, palettes, gradients and LUTs are always saved automatically."))
                    .font(.system(size: 11)).foregroundStyle(.secondary)
            }
            Section {
                Toggle(tr("Show built-in presets"), isOn: Binding(get: { PresetStore.shared.factoryVisible },
                                                                  set: { PresetStore.shared.setFactoryVisible($0) }))
            }
        }
        .formStyle(.grouped)
    }
}

struct ExportSettings: View {
    @ObservedObject var settings = AppSettings.shared
    var body: some View {
        Form {
            Picker(tr("Default format"), selection: $settings.exportFormat) {
                Text("PNG").tag(ImageExport.Format.png)
                Text("JPG").tag(ImageExport.Format.jpg)
            }
            .pickerStyle(.segmented)
            Picker(tr("Default scale"), selection: $settings.exportScale) {
                Text("1×").tag(1)
                Text("2×").tag(2)
            }
            .pickerStyle(.segmented)
            LabeledContent(tr("JPG quality")) {
                HStack {
                    Slider(value: $settings.exportQuality, in: 0.5...1)
                    Text("\(Int(settings.exportQuality * 100))%").monospacedDigit().frame(width: 40)
                }
            }
        }
        .formStyle(.grouped)
    }
}

struct ShortcutSettings: View {
    @ObservedObject var store = ShortcutStore.shared
    var body: some View {
        VStack(spacing: 0) {
            List {
                ForEach(ShortcutAction.allCases, id: \.self) { a in
                    HStack {
                        Text(tr(a.label))
                        Spacer()
                        Button {
                            store.recording = store.recording == a ? nil : a
                        } label: {
                            Text(store.recording == a ? tr("Type shortcut…") : (store.combo(a)?.display ?? "—"))
                                .font(.system(size: 12, design: .rounded).monospaced())
                                .frame(minWidth: 90)
                                .padding(.vertical, 3)
                                .background(RoundedRectangle(cornerRadius: 5).fill(store.recording == a ? Theme.accent.opacity(0.3) : Color.primary.opacity(0.07)))
                        }
                        .buttonStyle(.plain)
                    }
                }
            }
            HStack {
                Text(tr("Click a shortcut, then press the new keys. ⎋ cancels, ⌫ clears."))
                    .font(.system(size: 11)).foregroundStyle(.secondary)
                Spacer()
                Button(tr("Restore Defaults")) { store.resetAll() }
            }
            .padding(12)
        }
    }
}

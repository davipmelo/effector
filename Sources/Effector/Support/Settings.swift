import Foundation
import SwiftUI
import AppKit

enum AppLanguage: String, CaseIterable, Codable {
    case en, pt, system
    var label: String {
        switch self {
        case .en: return "English"
        case .pt: return "Português"
        case .system: return tr("Follow System")
        }
    }
}

enum AppTheme: String, CaseIterable, Codable {
    case dark, light, system
    var label: String {
        switch self {
        case .dark: return tr("Dark")
        case .light: return tr("Light")
        case .system: return tr("Follow System")
        }
    }
    var colorScheme: ColorScheme? {
        switch self {
        case .dark: return .dark
        case .light: return .light
        case .system: return nil
        }
    }
    var nsAppearance: NSAppearance? {
        switch self {
        case .dark: return NSAppearance(named: .darkAqua)
        case .light: return NSAppearance(named: .aqua)
        case .system: return nil
        }
    }
}

/// Current resolved language, readable from anywhere (used by `tr`).
enum L10nState {
    nonisolated(unsafe) static var isPortuguese = false
}

final class AppSettings: ObservableObject {
    static let shared = AppSettings()
    private let d = UserDefaults.standard

    @Published var language: AppLanguage { didSet { d.set(language.rawValue, forKey: "language"); applyLanguage() } }
    @Published var theme: AppTheme { didSet { d.set(theme.rawValue, forKey: "theme"); applyTheme() } }
    @Published var exportFormat: ImageExport.Format { didSet { d.set(exportFormat.rawValue, forKey: "exportFormat") } }
    @Published var exportQuality: Double { didSet { d.set(exportQuality, forKey: "exportQuality") } }
    @Published var exportScale: Int { didSet { d.set(exportScale, forKey: "exportScale") } }
    /// Reopen the last image and its layers when the app starts.
    @Published var reopenLastSession: Bool { didSet { d.set(reopenLastSession, forKey: "reopenLastSession") } }
    /// Hovering effects / presets previews them on the image.
    @Published var hoverPreview: Bool { didSet { d.set(hoverPreview, forKey: "hoverPreview") } }
    /// Bumped when the language changes so views re-render.
    @Published var languageRevision = 0

    private init() {
        language = AppLanguage(rawValue: d.string(forKey: "language") ?? "") ?? .en
        theme = AppTheme(rawValue: d.string(forKey: "theme") ?? "") ?? .dark
        exportFormat = ImageExport.Format(rawValue: d.string(forKey: "exportFormat") ?? "") ?? .png
        exportQuality = d.object(forKey: "exportQuality") as? Double ?? 0.92
        exportScale = d.object(forKey: "exportScale") as? Int ?? 1
        reopenLastSession = d.object(forKey: "reopenLastSession") as? Bool ?? true
        hoverPreview = d.object(forKey: "hoverPreview") as? Bool ?? true
        applyLanguage()
    }

    func applyLanguage() {
        switch language {
        case .en: L10nState.isPortuguese = false
        case .pt: L10nState.isPortuguese = true
        case .system: L10nState.isPortuguese = (Locale.preferredLanguages.first ?? "en").hasPrefix("pt")
        }
        // System menus (File, Edit, Window…) follow AppleLanguages; applied on next launch.
        switch language {
        case .en: d.set(["en"], forKey: "AppleLanguages")
        case .pt: d.set(["pt-BR"], forKey: "AppleLanguages")
        case .system: d.removeObject(forKey: "AppleLanguages")
        }
        languageRevision += 1
    }

    func applyTheme() {
        NSApp?.appearance = theme.nsAppearance
    }
}

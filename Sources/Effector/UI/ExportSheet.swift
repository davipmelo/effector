import SwiftUI

struct ExportSheet: View {
    @ObservedObject var app = AppState.shared
    @ObservedObject var settings = AppSettings.shared
    @State private var format: ImageExport.Format = .png
    @State private var scale = 1
    @State private var quality = 0.92
    @State private var scope = "this"
    @AppStorage("exportSuffix") private var suffix = "-edit"

    private var batchIDs: Set<UUID> {
        switch scope {
        case "selected": return app.stripSelection
        case "all": return Set(app.docs.map(\.id))
        default: return []
        }
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 18) {
            HStack(spacing: 10) {
                Image(systemName: "square.and.arrow.up").font(.system(size: 18)).foregroundStyle(Theme.accentText)
                Text(tr("Export Image")).font(.system(size: 16, weight: .semibold))
            }
            VStack(alignment: .leading, spacing: 14) {
                if app.docs.count > 1 {
                    SegmentedRow(label: "Images", options: opts([("this", tr("This Image")),
                                                                   ("selected", tr("Selected (%d)", app.stripSelection.count)),
                                                                   ("all", tr("All (%d)", app.docs.count))]), selected: scope) { scope = $0 }
                }
                SegmentedRow(label: "Format", options: opts([("png", "PNG"), ("jpg", "JPG")]), selected: format.rawValue) {
                    format = ImageExport.Format(rawValue: $0) ?? .png
                }
                SegmentedRow(label: "Size", options: opts([("1", "1×"), ("2", "2×")]), selected: "\(scale)") { scale = Int($0) ?? 1 }
                if format == .jpg {
                    ValueSlider(label: "Quality", value: quality * 100, range: 50...100, defaultValue: 92, step: 1, unit: "%") { quality = $0 / 100 }
                }
                if scope == "this" {
                    let (w, h) = app.exportSize(scale: scale)
                    HStack {
                        Text(tr("Output")).font(Theme.labelFont).foregroundStyle(Theme.textDim)
                        Spacer()
                        Text("\(w) × \(h) px").font(Theme.valueFont)
                    }
                } else {
                    TextParamRow(label: "File name suffix", value: suffix) { suffix = $0 }
                    Text(tr("Each image is exported with its own effects into a folder you choose."))
                        .font(.system(size: 11)).foregroundStyle(Theme.textFaint)
                }
                if format == .png && (app.source?.hasAlpha ?? false) {
                    Label(tr("Transparency will be preserved"), systemImage: "checkerboard.rectangle")
                        .font(.system(size: 11)).foregroundStyle(Theme.textFaint)
                }
            }
            HStack {
                Spacer()
                Button(tr("Cancel")) { app.showExport = false }.keyboardShortcut(.cancelAction).buttonStyle(PillButtonStyle())
                Button {
                    settings.exportFormat = format; settings.exportScale = scale; settings.exportQuality = quality
                    if scope == "this" { app.export(format: format, quality: quality, scale: scale) }
                    else { app.exportBatch(batchIDs, format: format, quality: quality, scale: scale, suffix: suffix) }
                } label: {
                    if app.isExporting {
                        HStack(spacing: 6) { ProgressView().controlSize(.small); Text(app.exportProgress ?? "") .font(Theme.valueFont) }
                    } else { Text(scope == "this" ? tr("Export…") : tr("Export %d Images…", batchIDs.count)) }
                }
                .keyboardShortcut(.defaultAction)
                .buttonStyle(PillButtonStyle(prominent: true))
                .disabled(app.isExporting)
            }
        }
        .padding(22)
        .frame(width: 360)
        .background(Theme.panelBG)
        .onAppear {
            format = settings.exportFormat; scale = settings.exportScale; quality = settings.exportQuality
        }
    }
}

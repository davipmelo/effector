import SwiftUI

struct ExportSheet: View {
    @ObservedObject var app = AppState.shared
    @ObservedObject var settings = AppSettings.shared
    @State private var format: ImageExport.Format = .png
    @State private var scale = 1
    @State private var quality = 0.92

    var body: some View {
        VStack(alignment: .leading, spacing: 18) {
            HStack(spacing: 10) {
                Image(systemName: "square.and.arrow.up").font(.system(size: 18)).foregroundStyle(Theme.accentText)
                Text(tr("Export Image")).font(.system(size: 16, weight: .semibold))
            }
            VStack(alignment: .leading, spacing: 14) {
                SegmentedRow(label: "Format", options: opts([("png", "PNG"), ("jpg", "JPG")]), selected: format.rawValue) {
                    format = ImageExport.Format(rawValue: $0) ?? .png
                }
                SegmentedRow(label: "Size", options: opts([("1", "1×"), ("2", "2×")]), selected: "\(scale)") { scale = Int($0) ?? 1 }
                if format == .jpg {
                    ValueSlider(label: "Quality", value: quality * 100, range: 50...100, defaultValue: 92, step: 1, unit: "%") { quality = $0 / 100 }
                }
                let (w, h) = app.exportSize(scale: scale)
                HStack {
                    Text(tr("Output")).font(Theme.labelFont).foregroundStyle(Theme.textDim)
                    Spacer()
                    Text("\(w) × \(h) px").font(Theme.valueFont)
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
                    app.export(format: format, quality: quality, scale: scale)
                } label: {
                    if app.isExporting { ProgressView().controlSize(.small).frame(width: 60) } else { Text(tr("Export…")) }
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

import SwiftUI

struct CropAspect {
    static let options: [(String, String, Double?)] = [
        ("free", "Free", nil), ("original", "Original", -1), ("1:1", "1:1", 1), ("4:5", "4:5", 4.0 / 5),
        ("5:4", "5:4", 5.0 / 4), ("3:2", "3:2", 3.0 / 2), ("2:3", "2:3", 2.0 / 3), ("4:3", "4:3", 4.0 / 3),
        ("3:4", "3:4", 3.0 / 4), ("16:9", "16:9", 16.0 / 9), ("9:16", "9:16", 9.0 / 16),
        ("1.85:1", "1.85:1 (Cinema)", 1.85), ("2:1", "2:1", 2), ("2.35:1", "2.35:1 (Scope)", 2.35), ("2.7:1", "2.7:1 (XPan)", 2.7),
        ("1:2.35", "1:2.35", 1 / 2.35),
    ]
}

struct CropOverlay: View {
    let viewSize: CGSize
    @ObservedObject var app = AppState.shared
    @State private var start: CropRect?

    private var imgSize: CGSize {
        guard let s = app.source else { return .zero }
        return CGSize(width: s.width, height: s.height)
    }

    private var imageRect: CGRect {
        let z = min((viewSize.width - 80) / imgSize.width, (viewSize.height - 130) / imgSize.height)
        let yOff: CGFloat = -25
        let w = imgSize.width * z, h = imgSize.height * z
        return CGRect(x: (viewSize.width - w) / 2, y: (viewSize.height - h) / 2 + yOff, width: w, height: h)
    }

    /// target aspect in normalized units (w/h in normalized space)
    private var normAspect: Double? {
        guard let entry = CropAspect.options.first(where: { $0.0 == app.cropAspect }), let a = entry.2 else { return nil }
        let pixelAspect = a < 0 ? imgSize.width / imgSize.height : a
        return pixelAspect * imgSize.height / imgSize.width
    }

    var body: some View {
        let ir = imageRect
        let c = app.cropDraft
        let cr = CGRect(x: ir.minX + c.x * ir.width, y: ir.minY + c.y * ir.height, width: c.w * ir.width, height: c.h * ir.height)
        ZStack(alignment: .topLeading) {
            // dim outside
            Path { p in
                p.addRect(CGRect(origin: .zero, size: viewSize))
                p.addRect(cr)
            }
            .fill(Color.black.opacity(0.55), style: FillStyle(eoFill: true))
            .allowsHitTesting(false)
            // thirds grid
            Path { p in
                for i in 1..<3 {
                    let f = CGFloat(i) / 3
                    p.move(to: CGPoint(x: cr.minX + cr.width * f, y: cr.minY)); p.addLine(to: CGPoint(x: cr.minX + cr.width * f, y: cr.maxY))
                    p.move(to: CGPoint(x: cr.minX, y: cr.minY + cr.height * f)); p.addLine(to: CGPoint(x: cr.maxX, y: cr.minY + cr.height * f))
                }
            }
            .stroke(Color.white.opacity(0.35), lineWidth: 0.5)
            .allowsHitTesting(false)
            Rectangle().stroke(Theme.accent, lineWidth: 1.5)
                .frame(width: cr.width, height: cr.height)
                .offset(x: cr.minX, y: cr.minY)
                .allowsHitTesting(false)
            // move area
            Rectangle().fill(Color.white.opacity(0.001))
                .frame(width: max(0, cr.width - 24), height: max(0, cr.height - 24))
                .offset(x: cr.minX + 12, y: cr.minY + 12)
                .gesture(DragGesture().onChanged { g in
                    if start == nil { start = app.cropDraft }
                    guard let s = start else { return }
                    var n = s
                    n.x = min(max(0, s.x + g.translation.width / ir.width), 1 - s.w)
                    n.y = min(max(0, s.y + g.translation.height / ir.height), 1 - s.h)
                    app.cropDraft = n
                }.onEnded { _ in start = nil })
            ForEach(handles, id: \.0) { h in
                let p = CGPoint(x: cr.minX + cr.width * h.1.x, y: cr.minY + cr.height * h.1.y)
                RoundedRectangle(cornerRadius: 2)
                    .fill(Color.white)
                    .overlay(RoundedRectangle(cornerRadius: 2).stroke(Color.black.opacity(0.4)))
                    .frame(width: 12, height: 12)
                    .position(p)
                    .gesture(DragGesture().onChanged { g in dragHandle(h.1, g, ir) }.onEnded { _ in start = nil })
            }
            VStack {
                Spacer()
                cropBar.padding(.bottom, 12)
            }
            .frame(width: viewSize.width, height: viewSize.height)
        }
        .frame(width: viewSize.width, height: viewSize.height)
        .onChange(of: app.cropAspect) { _, _ in applyAspect() }
    }

    private var handles: [(String, CGPoint)] {
        [("tl", CGPoint(x: 0, y: 0)), ("t", CGPoint(x: 0.5, y: 0)), ("tr", CGPoint(x: 1, y: 0)),
         ("l", CGPoint(x: 0, y: 0.5)), ("r", CGPoint(x: 1, y: 0.5)),
         ("bl", CGPoint(x: 0, y: 1)), ("b", CGPoint(x: 0.5, y: 1)), ("br", CGPoint(x: 1, y: 1))]
    }

    private func dragHandle(_ h: CGPoint, _ g: DragGesture.Value, _ ir: CGRect) {
        if start == nil { start = app.cropDraft }
        guard let s = start else { return }
        let dx = g.translation.width / ir.width, dy = g.translation.height / ir.height
        var x0 = s.x, y0 = s.y, x1 = s.x + s.w, y1 = s.y + s.h
        if h.x == 0 { x0 = min(max(0, x0 + dx), x1 - 0.02) }
        if h.x == 1 { x1 = max(min(1, x1 + dx), x0 + 0.02) }
        if h.y == 0 { y0 = min(max(0, y0 + dy), y1 - 0.02) }
        if h.y == 1 { y1 = max(min(1, y1 + dy), y0 + 0.02) }
        var n = CropRect(x: x0, y: y0, w: x1 - x0, h: y1 - y0)
        if let a = normAspect {
            // adjust the free dimension to keep aspect
            if h.x == 0.5 { // vertical edge drag -> width follows
                let w = n.h * a
                n.x = max(0, min(1 - w, n.x + (n.w - w) / 2)); n.w = min(1, w)
            } else {
                let hh = n.w / a
                if h.y == 0 { n.y = max(0, y1 - hh) }
                n.h = min(1 - n.y, hh)
                n.w = n.h * a
                if h.x == 0 { n.x = x1 - n.w }
            }
        }
        app.cropDraft = n
    }

    private func applyAspect() {
        guard let a = normAspect else { return }
        var n = app.cropDraft
        let cx = n.x + n.w / 2, cy = n.y + n.h / 2
        var w = n.w, h = w / a
        if h > 1 { h = 1; w = h * a }
        if w > 1 { w = 1; h = w / a }
        // fit as large as possible around the current center
        let maxW = min(1.0, a)
        if w < maxW * 0.5 { w = maxW * 0.9; h = w / a }
        n.w = w; n.h = h
        n.x = min(max(0, cx - w / 2), 1 - w)
        n.y = min(max(0, cy - h / 2), 1 - h)
        app.cropDraft = n
    }

    private var cropBar: some View {
        HStack(spacing: 10) {
            Image(systemName: "crop").foregroundStyle(Theme.accentText)
            Menu {
                ForEach(CropAspect.options, id: \.0) { o in
                    Button(tr(o.1)) { app.cropAspect = o.0 }
                }
            } label: {
                Text(tr(CropAspect.options.first { $0.0 == app.cropAspect }?.1 ?? "Free")).font(.system(size: 12))
            }
            .menuStyle(.borderlessButton).fixedSize()
            if let s = app.source {
                let r = app.cropDraft.pixelRect(width: s.width, height: s.height)
                Text("\(Int(r.width)) × \(Int(r.height))").font(Theme.valueFont).foregroundStyle(Theme.textDim)
            }
            Divider().frame(height: 16)
            Button(tr("Reset")) { app.resetCrop() }.buttonStyle(PillButtonStyle())
            Button(tr("Cancel")) { app.cancelCrop() }.buttonStyle(PillButtonStyle()).keyboardShortcut(.cancelAction)
            Button(tr("Apply")) { app.commitCrop() }.buttonStyle(PillButtonStyle(prominent: true)).keyboardShortcut(.defaultAction)
        }
        .padding(.horizontal, 12).padding(.vertical, 8)
        .background(.regularMaterial, in: RoundedRectangle(cornerRadius: 10))
    }
}

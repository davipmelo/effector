import SwiftUI

struct CurveEditor: View {
    let curves: CurveSet
    let onChange: (CurveSet) -> Void
    @State private var channel = 0
    @State private var dragIndex: Int?

    private let channelColors: [Color] = [.white, .red, .green, Color(red: 0.3, green: 0.55, blue: 1)]

    var body: some View {
        VStack(spacing: 8) {
            SegmentedRow(label: "", options: opts([("0", "RGB"), ("1", "Red"), ("2", "Green"), ("3", "Blue")]),
                         selected: "\(channel)") { channel = Int($0) ?? 0 }
            GeometryReader { geo in
                let size = geo.size
                let pts = curves[channel].sorted { $0.x < $1.x }
                ZStack {
                    RoundedRectangle(cornerRadius: 6).fill(Theme.field)
                    Path { p in
                        for i in 1..<4 {
                            let f = CGFloat(i) / 4
                            p.move(to: CGPoint(x: f * size.width, y: 0)); p.addLine(to: CGPoint(x: f * size.width, y: size.height))
                            p.move(to: CGPoint(x: 0, y: f * size.height)); p.addLine(to: CGPoint(x: size.width, y: f * size.height))
                        }
                    }.stroke(Theme.separator, lineWidth: 1)
                    Path { p in
                        p.move(to: CGPoint(x: 0, y: size.height)); p.addLine(to: CGPoint(x: size.width, y: 0))
                    }.stroke(Theme.textFaint.opacity(0.4), style: StrokeStyle(lineWidth: 1, dash: [3, 3]))
                    // other channels faintly
                    ForEach(0..<4, id: \.self) { c in
                        if c != channel && curves[c] != CurveSet.identity {
                            curvePath(curves[c], size).stroke(channelColors[c].opacity(0.3), lineWidth: 1)
                        }
                    }
                    curvePath(curves[channel], size).stroke(channel == 0 ? Theme.text : channelColors[channel], lineWidth: 1.6)
                    ForEach(Array(pts.enumerated()), id: \.offset) { i, pt in
                        Circle()
                            .fill(dragIndex == i ? Theme.accent : Color.white)
                            .overlay(Circle().stroke(Color.black.opacity(0.4), lineWidth: 1))
                            .frame(width: 9, height: 9)
                            .position(x: pt.x * size.width, y: (1 - pt.y) * size.height)
                    }
                }
                .contentShape(Rectangle())
                .gesture(
                    DragGesture(minimumDistance: 0)
                        .onChanged { g in drag(g, size: size) }
                        .onEnded { g in
                            // dragged far outside → remove point
                            if let i = dragIndex {
                                let out = g.location.y < -20 || g.location.y > size.height + 20
                                var ps = curves[channel].sorted { $0.x < $1.x }
                                if out && i > 0 && i < ps.count - 1 {
                                    ps.remove(at: i)
                                    var c = curves; c[channel] = ps; onChange(c)
                                }
                            }
                            dragIndex = nil
                        }
                )
                .simultaneousGesture(SpatialTapGesture(count: 2).onEnded { e in
                    // double-click on a point removes it
                    var ps = curves[channel].sorted { $0.x < $1.x }
                    if let i = nearest(ps, e.location, size), i > 0, i < ps.count - 1 {
                        ps.remove(at: i)
                        var c = curves; c[channel] = ps; onChange(c)
                    }
                })
            }
            .aspectRatio(1, contentMode: .fit)
            HStack {
                Text(tr("Click to add · drag out to remove")).font(.system(size: 10)).foregroundStyle(Theme.textFaint)
                Spacer()
                Button(tr("Reset")) { var c = curves; c[channel] = CurveSet.identity; onChange(c) }
                    .buttonStyle(.plain).font(.system(size: 11)).foregroundStyle(Theme.accentText)
            }
        }
    }

    private func curvePath(_ pts: [CurvePoint], _ size: CGSize) -> Path {
        Path { p in
            let n = 96
            for i in 0...n {
                let x = Double(i) / Double(n)
                let y = CurveMath.evaluate(pts, x)
                let pt = CGPoint(x: x * size.width, y: (1 - y) * size.height)
                if i == 0 { p.move(to: pt) } else { p.addLine(to: pt) }
            }
        }
    }

    private func nearest(_ ps: [CurvePoint], _ loc: CGPoint, _ size: CGSize) -> Int? {
        var best: Int?; var bd = 12.0
        for (i, p) in ps.enumerated() {
            let d = hypot(p.x * size.width - loc.x, (1 - p.y) * size.height - loc.y)
            if d < bd { bd = d; best = i }
        }
        return best
    }

    private func drag(_ g: DragGesture.Value, size: CGSize) {
        var ps = curves[channel].sorted { $0.x < $1.x }
        if dragIndex == nil {
            if let i = nearest(ps, g.startLocation, size) {
                dragIndex = i
            } else {
                let x = min(1, max(0, g.startLocation.x / size.width))
                let np = CurvePoint(x: x, y: CurveMath.evaluate(ps, x))
                ps.append(np)
                ps.sort { $0.x < $1.x }
                dragIndex = ps.firstIndex(of: np)
            }
        }
        guard let i = dragIndex, i < ps.count else { return }
        var x = min(1, max(0, g.location.x / size.width))
        let y = min(1, max(0, 1 - g.location.y / size.height))
        let lo = i > 0 ? ps[i - 1].x + 0.01 : 0
        let hi = i < ps.count - 1 ? ps[i + 1].x - 0.01 : 1
        if i == 0 { x = min(x, hi) } else if i == ps.count - 1 { x = max(x, lo) } else { x = min(max(x, lo), hi) }
        ps[i] = CurvePoint(x: x, y: y)
        var c = curves; c[channel] = ps
        onChange(c)
    }
}

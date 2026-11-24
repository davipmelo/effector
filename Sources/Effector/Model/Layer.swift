import Foundation

enum BlendMode: String, Codable, CaseIterable, Hashable {
    case normal, multiply, screen, overlay, softLight, hardLight, darken, lighten, difference, color, luminosity
    // appended later (shader indices follow declaration order)
    case colorDodge, colorBurn, add, linearBurn, exclusion, hue, saturation

    static let menuOrder: [BlendMode] = [.normal, .multiply, .darken, .colorBurn, .linearBurn, .screen, .lighten, .colorDodge, .add,
                                         .overlay, .softLight, .hardLight, .difference, .exclusion, .hue, .saturation, .color, .luminosity]

    var label: String {
        switch self {
        case .normal: return "Normal"
        case .multiply: return "Multiply"
        case .screen: return "Screen"
        case .overlay: return "Overlay"
        case .softLight: return "Soft Light"
        case .hardLight: return "Hard Light"
        case .darken: return "Darken"
        case .lighten: return "Lighten"
        case .difference: return "Difference"
        case .color: return "Color"
        case .luminosity: return "Luminosity"
        case .colorDodge: return "Color Dodge"
        case .colorBurn: return "Color Burn"
        case .add: return "Add (Linear Dodge)"
        case .linearBurn: return "Linear Burn"
        case .exclusion: return "Exclusion"
        case .hue: return "Hue"
        case .saturation: return "Saturation"
        }
    }

    var group: String {
        switch self {
        case .normal: return "Normal"
        case .multiply, .darken, .colorBurn, .linearBurn: return "Darken"
        case .screen, .lighten, .colorDodge, .add: return "Lighten"
        case .overlay, .softLight, .hardLight: return "Contrast"
        case .difference, .exclusion: return "Inversion"
        case .color, .luminosity, .hue, .saturation: return "Component"
        }
    }

    var shaderIndex: Float { Float(BlendMode.allCases.firstIndex(of: self)!) }
}

/// A node in the layer tree. Either an effect layer or a group.
/// `children` is ordered top-first (index 0 = top of the list = applied last).
struct LayerNode: Codable, Hashable, Identifiable {
    var id = UUID()
    var name: String
    var isGroup: Bool
    var effectID: String = ""
    var params = Params()
    var enabled = true
    var opacity: Double = 1
    var blend: BlendMode = .normal
    var expanded = true
    var children: [LayerNode] = []

    static func effect(_ id: String, name: String? = nil, params: Params? = nil) -> LayerNode {
        let eff = EffectRegistry.effect(id)
        var p = params ?? Params()
        if let eff {
            for def in eff.params where p[def.key] == nil {
                if let d = def.defaultValue { p[def.key] = d }
            }
        }
        return LayerNode(name: name ?? eff.map { tr($0.name) } ?? id, isGroup: false, effectID: id, params: p)
    }

    static func group(_ name: String, _ children: [LayerNode]) -> LayerNode {
        LayerNode(name: name, isGroup: true, children: children)
    }

    /// Hash of the rendering-relevant state (ignores name / expansion).
    func renderHash(into h: inout Hasher) {
        h.combine(isGroup)
        h.combine(effectID)
        h.combine(params)
        h.combine(enabled)
        h.combine(opacity)
        h.combine(blend)
        for c in children { c.renderHash(into: &h) }
    }

    /// Fresh copy with new ids everywhere (used for duplicate / presets).
    func reidentified() -> LayerNode {
        var n = self
        n.id = UUID()
        n.children = children.map { $0.reidentified() }
        return n
    }
}

// MARK: - Tree helpers

extension Array where Element == LayerNode {
    func find(_ id: UUID) -> LayerNode? {
        for n in self {
            if n.id == id { return n }
            if let f = n.children.find(id) { return f }
        }
        return nil
    }

    mutating func update(_ id: UUID, _ body: (inout LayerNode) -> Void) {
        for i in indices {
            if self[i].id == id { body(&self[i]); return }
            self[i].children.update(id, body)
        }
    }

    /// Removes and returns the node.
    @discardableResult
    mutating func remove(_ id: UUID) -> LayerNode? {
        if let i = firstIndex(where: { $0.id == id }) { return self.remove(at: i) }
        for i in indices {
            if let r = self[i].children.remove(id) { return r }
        }
        return nil
    }

    /// Location of a node: parent id (nil = root) and index.
    func location(of id: UUID, parent: UUID? = nil) -> (parent: UUID?, index: Int)? {
        if let i = firstIndex(where: { $0.id == id }) { return (parent, i) }
        for n in self {
            if let l = n.children.location(of: id, parent: n.id) { return l }
        }
        return nil
    }

    mutating func insert(_ node: LayerNode, parent: UUID?, index: Int) {
        if let parent {
            update(parent) { p in
                let i = Swift.min(Swift.max(0, index), p.children.count)
                p.children.insert(node, at: i)
            }
        } else {
            let i = Swift.min(Swift.max(0, index), count)
            insert(node, at: i)
        }
    }

    func contains(_ id: UUID, inside ancestor: UUID) -> Bool {
        guard let a = find(ancestor) else { return false }
        return a.children.find(id) != nil
    }

    /// Flattened list for display, with depth.
    func flattened(depth: Int = 0) -> [(node: LayerNode, depth: Int)] {
        var out: [(LayerNode, Int)] = []
        for n in self {
            out.append((n, depth))
            if n.isGroup && n.expanded {
                out.append(contentsOf: n.children.flattened(depth: depth + 1))
            }
        }
        return out
    }

    var allEffectIDs: [String] {
        flatMap { $0.isGroup ? $0.children.allEffectIDs : [$0.effectID] }
    }
}

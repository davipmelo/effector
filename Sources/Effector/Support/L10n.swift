import Foundation

/// Translates an English UI string. English text is the key.
func tr(_ s: String) -> String {
    guard L10nState.isPortuguese else { return s }
    return PT.dict[s] ?? s
}

/// Translate with format arguments, e.g. tr("%d layers", n)
func tr(_ s: String, _ args: CVarArg...) -> String {
    String(format: tr(s), arguments: args)
}
